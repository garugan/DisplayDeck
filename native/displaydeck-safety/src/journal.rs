use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::ActorFence;

const HEADER_SIZE: usize = 4_096;
const SLOT_SIZE: usize = 4_096;
const FILE_SIZE: usize = HEADER_SIZE + SLOT_SIZE * 2;
const RECORD_LENGTH: usize = 440;
const HEADER_MAGIC: [u8; 16] = *b"DDDJV1\0\0\0\0\0\0\0\0\0\0";
const SLOT_MAGIC: [u8; 16] = *b"DJSLOTV1\0\0\0\0\0\0\0\0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    RevertRequired = 1,
    KeptSession = 2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionRecord {
    pub slot_index: u8,
    pub decision: Decision,
    pub generation: u64,
    pub previous_generation: u64,
    pub state_version: u64,
    pub fence: ActorFence,
    pub confirmation_deadline_tick_ms: u64,
    pub keep_authorized_tick_ms: u64,
    pub decision_written_tick_ms: u64,
    pub created_tick_ms: u64,
    pub candidate_digest: [u8; 32],
    pub expected_display_mode_digest: [u8; 32],
    pub previous_display_mode_digest: [u8; 32],
    pub expected_rollback_snapshot_digest: [u8; 32],
}

impl DecisionRecord {
    pub fn baseline(
        fence: ActorFence,
        created_tick_ms: u64,
        previous_display_mode_digest: [u8; 32],
        expected_rollback_snapshot_digest: [u8; 32],
    ) -> Self {
        Self {
            slot_index: 0,
            decision: Decision::RevertRequired,
            generation: 1,
            previous_generation: 0,
            state_version: 1,
            fence,
            confirmation_deadline_tick_ms: 0,
            keep_authorized_tick_ms: 0,
            decision_written_tick_ms: 0,
            created_tick_ms,
            candidate_digest: [0; 32],
            expected_display_mode_digest: [0; 32],
            previous_display_mode_digest,
            expected_rollback_snapshot_digest,
        }
    }

    pub fn kept(
        fence: ActorFence,
        confirmation_deadline_tick_ms: u64,
        keep_authorized_tick_ms: u64,
        decision_written_tick_ms: u64,
        candidate_digest: [u8; 32],
        expected_display_mode_digest: [u8; 32],
    ) -> Self {
        Self {
            slot_index: 1,
            decision: Decision::KeptSession,
            generation: 2,
            previous_generation: 1,
            state_version: 2,
            fence,
            confirmation_deadline_tick_ms,
            keep_authorized_tick_ms,
            decision_written_tick_ms,
            created_tick_ms: 0,
            candidate_digest,
            expected_display_mode_digest,
            previous_display_mode_digest: [0; 32],
            expected_rollback_snapshot_digest: [0; 32],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournalClassification {
    FreshUninitialized,
    Baseline(DecisionRecord),
    Kept {
        baseline: DecisionRecord,
        kept: DecisionRecord,
    },
    FailedClosed(String),
}

pub struct DecisionJournal {
    path: PathBuf,
}

impl DecisionJournal {
    pub fn create_test_storage(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("create decision journal: {error}"))?;
        file.write_all(&vec![0_u8; FILE_SIZE])
            .map_err(|error| format!("initialize decision journal: {error}"))?;
        file.seek(SeekFrom::Start(0))
            .map_err(|error| format!("seek decision journal header: {error}"))?;
        file.write_all(&header_bytes())
            .map_err(|error| format!("write decision journal header: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("flush decision journal: {error}"))?;
        drop(file);

        let journal = Self { path };
        match journal.classify_unbound()? {
            JournalClassification::FreshUninitialized => Ok(journal),
            other => Err(format!("fresh decision journal readback failed: {other:?}")),
        }
    }

    pub fn open_test_storage(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn publish(&self, record: &DecisionRecord) -> Result<JournalClassification, String> {
        validate_record_semantics(record)?;
        let offset = HEADER_SIZE + usize::from(record.slot_index) * SLOT_SIZE;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|error| format!("open decision journal writer: {error}"))?;
        file.seek(SeekFrom::Start(offset as u64))
            .map_err(|error| format!("seek decision slot: {error}"))?;
        file.write_all(&encode_record(record))
            .map_err(|error| format!("write decision slot: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("flush decision slot: {error}"))?;
        drop(file);
        self.classify_for(&record.fence)
    }

    pub fn classify_for(&self, fence: &ActorFence) -> Result<JournalClassification, String> {
        classify_bytes(&read_exact_file(&self.path)?, Some(fence))
    }

    pub fn classify_unbound(&self) -> Result<JournalClassification, String> {
        classify_bytes(&read_exact_file(&self.path)?, None)
    }
}

fn header_bytes() -> [u8; HEADER_SIZE] {
    let mut header = [0_u8; HEADER_SIZE];
    header[0..16].copy_from_slice(&HEADER_MAGIC);
    put_u16(&mut header, 16, 1);
    put_u16(&mut header, 18, 1);
    put_u32(&mut header, 20, HEADER_SIZE as u32);
    put_u32(&mut header, 24, SLOT_SIZE as u32);
    put_u32(&mut header, 28, FILE_SIZE as u32);
    header[32] = 2;
    let checksum = sha256(&header);
    header[64..96].copy_from_slice(&checksum);
    header
}

fn encode_record(record: &DecisionRecord) -> [u8; SLOT_SIZE] {
    let mut slot = [0_u8; SLOT_SIZE];
    slot[0..16].copy_from_slice(&SLOT_MAGIC);
    put_u16(&mut slot, 16, 1);
    put_u16(&mut slot, 18, 1);
    slot[20] = record.slot_index;
    slot[21] = record.decision as u8;
    put_u32(&mut slot, 24, RECORD_LENGTH as u32);
    put_u64(&mut slot, 32, record.generation);
    put_u64(&mut slot, 40, record.previous_generation);
    put_u64(&mut slot, 48, record.state_version);
    put_u64(&mut slot, 56, record.fence.lease_version);
    slot[64..80].copy_from_slice(&record.fence.session_id);
    slot[80..112].copy_from_slice(&record.fence.boot_id);
    slot[112..128].copy_from_slice(&record.fence.controller_instance_id);
    slot[128..144].copy_from_slice(&record.fence.watchdog_instance_id);
    slot[144..176].copy_from_slice(&record.fence.display_id);
    slot[176..208].copy_from_slice(&record.fence.owner_sid_digest);
    put_u64(&mut slot, 208, record.fence.logon_id);
    put_u64(&mut slot, 216, record.confirmation_deadline_tick_ms);
    put_u64(&mut slot, 224, record.keep_authorized_tick_ms);
    put_u64(&mut slot, 232, record.decision_written_tick_ms);
    put_u64(&mut slot, 240, record.created_tick_ms);
    slot[248..280].copy_from_slice(&record.candidate_digest);
    slot[280..312].copy_from_slice(&record.expected_display_mode_digest);
    slot[312..344].copy_from_slice(&record.previous_display_mode_digest);
    slot[344..376].copy_from_slice(&record.expected_rollback_snapshot_digest);
    let payload_checksum = sha256(&slot[64..376]);
    slot[376..408].copy_from_slice(&payload_checksum);
    let mut covered = slot[..RECORD_LENGTH].to_vec();
    covered[408..440].fill(0);
    slot[408..440].copy_from_slice(&sha256(&covered));
    slot
}

fn classify_bytes(
    bytes: &[u8],
    expected_fence: Option<&ActorFence>,
) -> Result<JournalClassification, String> {
    if bytes.len() != FILE_SIZE {
        return Ok(JournalClassification::FailedClosed(format!(
            "WrongFileLength({})",
            bytes.len()
        )));
    }
    if let Err(reason) = validate_header(&bytes[..HEADER_SIZE]) {
        return Ok(JournalClassification::FailedClosed(reason));
    }

    let mut records = Vec::new();
    for slot_index in 0..2_u8 {
        let start = HEADER_SIZE + usize::from(slot_index) * SLOT_SIZE;
        let bytes = &bytes[start..start + SLOT_SIZE];
        if bytes.iter().all(|byte| *byte == 0) {
            continue;
        }
        match decode_record(bytes, slot_index) {
            Ok(record) => records.push(record),
            Err(reason) => return Ok(JournalClassification::FailedClosed(reason)),
        }
    }
    if records.is_empty() {
        return Ok(JournalClassification::FreshUninitialized);
    }

    if let Some(fence) = expected_fence {
        if records.iter().any(|record| &record.fence != fence) {
            return Ok(JournalClassification::FailedClosed(
                "ForeignIdentity".into(),
            ));
        }
    }
    records.sort_by_key(|record| record.generation);
    match records.as_slice() {
        [baseline]
            if baseline.decision == Decision::RevertRequired
                && baseline.generation == 1
                && baseline.previous_generation == 0 =>
        {
            Ok(JournalClassification::Baseline(baseline.clone()))
        }
        [baseline, kept]
            if baseline.decision == Decision::RevertRequired
                && kept.decision == Decision::KeptSession
                && baseline.generation == 1
                && kept.generation == 2
                && kept.previous_generation == baseline.generation =>
        {
            Ok(JournalClassification::Kept {
                baseline: baseline.clone(),
                kept: kept.clone(),
            })
        }
        _ => Ok(JournalClassification::FailedClosed(
            "ConflictingGeneration".into(),
        )),
    }
}

fn validate_header(header: &[u8]) -> Result<(), String> {
    if header[0..16] != HEADER_MAGIC
        || u16_at(header, 16) != 1
        || u16_at(header, 18) != 1
        || u32_at(header, 20) != HEADER_SIZE as u32
        || u32_at(header, 24) != SLOT_SIZE as u32
        || u32_at(header, 28) != FILE_SIZE as u32
        || header[32] != 2
        || header[33..64].iter().any(|byte| *byte != 0)
        || header[96..].iter().any(|byte| *byte != 0)
    {
        return Err("InvalidHeader".into());
    }
    let expected = &header[64..96];
    let mut covered = header.to_vec();
    covered[64..96].fill(0);
    if expected != sha256(&covered) {
        return Err("HeaderChecksumMismatch".into());
    }
    Ok(())
}

fn decode_record(slot: &[u8], physical_slot_index: u8) -> Result<DecisionRecord, String> {
    if slot[0..16] != SLOT_MAGIC
        || u16_at(slot, 16) != 1
        || u16_at(slot, 18) != 1
        || slot[20] != physical_slot_index
        || slot[22..24].iter().any(|byte| *byte != 0)
        || u32_at(slot, 24) != RECORD_LENGTH as u32
        || u32_at(slot, 28) != 0
        || slot[RECORD_LENGTH..].iter().any(|byte| *byte != 0)
    {
        return Err("InvalidSlotEnvelope".into());
    }
    if slot[376..408] != sha256(&slot[64..376]) {
        return Err("PayloadChecksumMismatch".into());
    }
    let mut covered = slot[..RECORD_LENGTH].to_vec();
    covered[408..440].fill(0);
    if slot[408..440] != sha256(&covered) {
        return Err("SlotChecksumMismatch".into());
    }
    let decision = match slot[21] {
        1 => Decision::RevertRequired,
        2 => Decision::KeptSession,
        _ => return Err("UnknownDecision".into()),
    };
    let record = DecisionRecord {
        slot_index: slot[20],
        decision,
        generation: u64_at(slot, 32),
        previous_generation: u64_at(slot, 40),
        state_version: u64_at(slot, 48),
        fence: ActorFence {
            session_id: array_at(slot, 64),
            boot_id: array_at(slot, 80),
            controller_instance_id: array_at(slot, 112),
            watchdog_instance_id: array_at(slot, 128),
            display_id: array_at(slot, 144),
            owner_sid_digest: array_at(slot, 176),
            logon_id: u64_at(slot, 208),
            lease_version: u64_at(slot, 56),
        },
        confirmation_deadline_tick_ms: u64_at(slot, 216),
        keep_authorized_tick_ms: u64_at(slot, 224),
        decision_written_tick_ms: u64_at(slot, 232),
        created_tick_ms: u64_at(slot, 240),
        candidate_digest: array_at(slot, 248),
        expected_display_mode_digest: array_at(slot, 280),
        previous_display_mode_digest: array_at(slot, 312),
        expected_rollback_snapshot_digest: array_at(slot, 344),
    };
    validate_record_semantics(&record)?;
    Ok(record)
}

fn validate_record_semantics(record: &DecisionRecord) -> Result<(), String> {
    if !record.fence.is_valid() {
        return Err("InvalidFence".into());
    }
    match record.decision {
        Decision::RevertRequired
            if record.slot_index < 2
                && record.generation == 1
                && record.previous_generation == 0
                && record.state_version == 1
                && record.confirmation_deadline_tick_ms == 0
                && record.keep_authorized_tick_ms == 0
                && record.decision_written_tick_ms == 0
                && record.candidate_digest == [0; 32]
                && record.expected_display_mode_digest == [0; 32]
                && record.previous_display_mode_digest != [0; 32]
                && record.expected_rollback_snapshot_digest != [0; 32] =>
        {
            Ok(())
        }
        Decision::KeptSession
            if record.slot_index < 2
                && record.generation == 2
                && record.previous_generation == 1
                && record.state_version == 2
                && record.created_tick_ms == 0
                && record.candidate_digest != [0; 32]
                && record.expected_display_mode_digest != [0; 32]
                && record.previous_display_mode_digest == [0; 32]
                && record.expected_rollback_snapshot_digest == [0; 32]
                && record.keep_authorized_tick_ms <= record.confirmation_deadline_tick_ms
                && record.decision_written_tick_ms >= record.keep_authorized_tick_ms =>
        {
            Ok(())
        }
        _ => Err("InvalidDecisionSemantics".into()),
    }
}

fn read_exact_file(path: &Path) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|error| format!("open decision journal: {error}"))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| format!("read decision journal: {error}"))?;
    Ok(bytes)
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn array_at<const N: usize>(bytes: &[u8], offset: usize) -> [u8; N] {
    bytes[offset..offset + N]
        .try_into()
        .expect("fixed record slice must match array length")
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(array_at(bytes, offset))
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(array_at(bytes, offset))
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(array_at(bytes, offset))
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes")
            .join(name)
    }

    #[test]
    fn candidate_04_decision_journal_vectors_classify() {
        assert_eq!(
            DecisionJournal::open_test_storage(fixture("DJV1-P-001.bin"))
                .classify_unbound()
                .unwrap(),
            JournalClassification::FreshUninitialized
        );
        let expected_fence = match DecisionJournal::open_test_storage(fixture("DJV1-P-002.bin"))
            .classify_unbound()
            .unwrap()
        {
            JournalClassification::Kept { baseline, .. } => baseline.fence,
            other => panic!("expected Candidate 04 kept chain, got {other:?}"),
        };
        for name in ["DJV1-P-002.bin", "DJV1-P-003.bin"] {
            assert!(
                matches!(
                    DecisionJournal::open_test_storage(fixture(name))
                        .classify_for(&expected_fence)
                        .unwrap(),
                    JournalClassification::Kept { .. }
                ),
                "{name}"
            );
        }
        assert!(matches!(
            DecisionJournal::open_test_storage(fixture("DJV1-P-004.bin"))
                .classify_for(&expected_fence)
                .unwrap(),
            JournalClassification::Baseline(_)
        ));
        for index in 1..=13 {
            let name = format!("DJV1-N-{index:03}.bin");
            if index == 6 {
                continue;
            }
            assert!(
                matches!(
                    DecisionJournal::open_test_storage(fixture(&name))
                        .classify_for(&expected_fence)
                        .unwrap(),
                    JournalClassification::FailedClosed(_)
                ),
                "{name}"
            );
        }
    }
}
