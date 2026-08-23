use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ActorFence;

const SLOT_SIZE: usize = 4_096;
const FILE_SIZE: usize = SLOT_SIZE * 2;
const PREFIX_SIZE: usize = 136;
const MAGIC: [u8; 16] = *b"DDWALSLOT1\0\0\0\0\0\0";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WalState {
    BaselineProvisioned,
    ApplyInFlight,
    AwaitingDecision,
    KeepAuthorized,
    KeptSession,
    RevertInFlight,
    Reverted,
    FailedClosed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalRecord {
    pub generation: u64,
    pub previous_generation: u64,
    pub fence: ActorFence,
    pub machine_epoch: u64,
    pub state: WalState,
}

pub struct OperationalWal {
    path: PathBuf,
}

impl OperationalWal {
    pub fn create_test_storage(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("create operational WAL: {error}"))?;
        file.write_all(&vec![0_u8; FILE_SIZE])
            .map_err(|error| format!("initialize operational WAL: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("flush operational WAL: {error}"))?;
        drop(file);
        Ok(Self { path })
    }

    pub fn open_test_storage(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn publish(&self, record: &WalRecord) -> Result<WalRecord, String> {
        if record.generation == 0
            || record.previous_generation != record.generation - 1
            || !record.fence.is_valid()
            || record.machine_epoch == 0
        {
            return Err("invalid operational WAL transition".into());
        }
        let slot_index = ((record.generation - 1) % 2) as u8;
        let slot = encode(record, slot_index)?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|error| format!("open operational WAL writer: {error}"))?;
        file.seek(SeekFrom::Start(u64::from(slot_index) * SLOT_SIZE as u64))
            .map_err(|error| format!("seek operational WAL: {error}"))?;
        file.write_all(&slot)
            .map_err(|error| format!("write operational WAL: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("flush operational WAL: {error}"))?;
        drop(file);
        let latest = self.read_latest_for(&record.fence)?;
        if latest.as_ref() != Some(record) {
            return Err("operational WAL readback mismatch".into());
        }
        Ok(record.clone())
    }

    pub fn read_latest_for(&self, fence: &ActorFence) -> Result<Option<WalRecord>, String> {
        let bytes = read_exact_file(&self.path)?;
        if bytes.len() != FILE_SIZE {
            return Err(format!("wrong operational WAL length: {}", bytes.len()));
        }
        let mut records = Vec::new();
        for slot_index in 0..2_u8 {
            let start = usize::from(slot_index) * SLOT_SIZE;
            let slot = &bytes[start..start + SLOT_SIZE];
            if slot.iter().all(|byte| *byte == 0) {
                continue;
            }
            let record = decode(slot, slot_index)?;
            if &record.fence != fence {
                return Err("foreign operational WAL identity".into());
            }
            records.push(record);
        }
        records.sort_by_key(|record| record.generation);
        for pair in records.windows(2) {
            if pair[1].generation != pair[0].generation + 1
                || pair[1].previous_generation != pair[0].generation
            {
                return Err("operational WAL generation conflict".into());
            }
        }
        Ok(records.pop())
    }
}

fn encode(record: &WalRecord, slot_index: u8) -> Result<[u8; SLOT_SIZE], String> {
    let payload = serde_json::to_vec(record)
        .map_err(|error| format!("serialize operational WAL: {error}"))?;
    let record_length = PREFIX_SIZE
        .checked_add(payload.len())
        .ok_or_else(|| "operational WAL length overflow".to_string())?;
    if record_length > SLOT_SIZE {
        return Err("operational WAL payload exceeds fixed slot".into());
    }

    let mut slot = [0_u8; SLOT_SIZE];
    slot[0..16].copy_from_slice(&MAGIC);
    slot[16..18].copy_from_slice(&1_u16.to_le_bytes());
    slot[18] = slot_index;
    slot[20..24].copy_from_slice(&(record_length as u32).to_le_bytes());
    slot[24..32].copy_from_slice(&record.generation.to_le_bytes());
    slot[32..40].copy_from_slice(&record.previous_generation.to_le_bytes());
    slot[40..72].copy_from_slice(&sha256(
        &serde_json::to_vec(&record.fence)
            .map_err(|error| format!("serialize WAL fence: {error}"))?,
    ));
    slot[72..104].copy_from_slice(&sha256(&payload));
    slot[PREFIX_SIZE..record_length].copy_from_slice(&payload);
    let mut covered = slot[..record_length].to_vec();
    covered[104..136].fill(0);
    slot[104..136].copy_from_slice(&sha256(&covered));
    Ok(slot)
}

fn decode(slot: &[u8], physical_slot_index: u8) -> Result<WalRecord, String> {
    if slot[0..16] != MAGIC
        || u16::from_le_bytes(slot[16..18].try_into().unwrap()) != 1
        || slot[18] != physical_slot_index
        || slot[19] != 0
    {
        return Err("invalid operational WAL envelope".into());
    }
    let record_length = u32::from_le_bytes(slot[20..24].try_into().unwrap()) as usize;
    if !(PREFIX_SIZE..=SLOT_SIZE).contains(&record_length)
        || slot[record_length..].iter().any(|byte| *byte != 0)
    {
        return Err("invalid operational WAL length or trailing bytes".into());
    }
    let payload = &slot[PREFIX_SIZE..record_length];
    if slot[72..104] != sha256(payload) {
        return Err("operational WAL payload checksum mismatch".into());
    }
    let mut covered = slot[..record_length].to_vec();
    covered[104..136].fill(0);
    if slot[104..136] != sha256(&covered) {
        return Err("operational WAL record checksum mismatch".into());
    }
    let record: WalRecord = serde_json::from_slice(payload)
        .map_err(|error| format!("parse operational WAL payload: {error}"))?;
    if record.generation == 0
        || record.previous_generation != record.generation - 1
        || record.generation != u64::from_le_bytes(slot[24..32].try_into().unwrap())
        || record.previous_generation != u64::from_le_bytes(slot[32..40].try_into().unwrap())
        || !record.fence.is_valid()
        || record.machine_epoch == 0
    {
        return Err("operational WAL semantic mismatch".into());
    }
    let fence_bytes = serde_json::to_vec(&record.fence)
        .map_err(|error| format!("serialize parsed WAL fence: {error}"))?;
    if slot[40..72] != sha256(&fence_bytes) {
        return Err("operational WAL fence checksum mismatch".into());
    }
    Ok(record)
}

fn read_exact_file(path: &Path) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|error| format!("open operational WAL: {error}"))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| format!("read operational WAL: {error}"))?;
    Ok(bytes)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
