use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const HEADER_SIZE: usize = 4_096;
const SLOT_SIZE: usize = 4_096;
const FILE_SIZE: usize = HEADER_SIZE + 2 * SLOT_SIZE;
const SLOT_PREFIX_SIZE: usize = 224;
const MACHINE_ACTOR_FILE_SIZE: u64 = 135_168;
const HEADER_MAGIC: [u8; 16] = *b"DDMAPRV1\0\0\0\0\0\0\0\0";
const SLOT_MAGIC: [u8; 16] = *b"MAPRSLOT1\0\0\0\0\0\0\0";
const MACHINE_SLOT_SIZE: usize = 65_536;
const MACHINE_PREFIX_SIZE: usize = 136;
const MACHINE_HEADER_MAGIC: [u8; 16] = *b"DDMARV1\0\0\0\0\0\0\0\0\0";
const MACHINE_SLOT_MAGIC: [u8; 16] = *b"MARSLOT1\0\0\0\0\0\0\0\0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvisionState {
    PostCreateCheckpoint,
    MachineIntentPublished,
    MachineActivePublished,
    MachineCleanObserved,
    TerminalRetained,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvisionRecordClassification {
    FreshUninitialized,
    /// Requires independently proven manifest authority and fresh target absence.
    UnqualifiedCreateIntent,
    Current(ProvisionState),
    FailedClosed,
}

/// Checks the record's internal structure and resident predecessor only.
/// File identity, manifest authority, and the referenced MachineActor bytes still require
/// independent observations; even `TerminalRetained` never grants a write or cleanup.
pub fn classify_candidate04_maprv1(
    bytes: &[u8],
) -> Result<ProvisionRecordClassification, &'static str> {
    parse_provision_record(bytes).map(|(classification, _)| classification)
}

fn parse_provision_record(
    bytes: &[u8],
) -> Result<(ProvisionRecordClassification, Vec<Slot<'_>>), &'static str> {
    validate_header(bytes)?;
    let mut slots = Vec::with_capacity(2);
    for index in 0..2_u8 {
        let start = HEADER_SIZE + usize::from(index) * SLOT_SIZE;
        if let Some(slot) = parse_slot(&bytes[start..start + SLOT_SIZE], index)? {
            slots.push(slot);
        }
    }

    slots.sort_by_key(|slot| slot.state_version);
    let classification = match slots.len() {
        0 => Ok(ProvisionRecordClassification::FreshUninitialized),
        1 if slots[0].state == 1 && slots[0].state_version == 1 => {
            Ok(ProvisionRecordClassification::UnqualifiedCreateIntent)
        }
        1 => Err("provision record is missing its resident predecessor"),
        2 => classify_chain(&slots),
        _ => unreachable!("a provision record has exactly two slots"),
    }?;
    Ok((classification, slots))
}

fn validate_header(bytes: &[u8]) -> Result<(), &'static str> {
    validate_envelope_header(bytes, HEADER_MAGIC, SLOT_SIZE, FILE_SIZE)
}

fn validate_envelope_header(
    bytes: &[u8],
    magic: [u8; 16],
    slot_size: usize,
    file_size: usize,
) -> Result<(), &'static str> {
    if bytes.len() != file_size
        || bytes[0..16] != magic
        || read_u16(bytes, 16) != 1
        || read_u16(bytes, 18) != 1
        || read_u32(bytes, 20) as usize != HEADER_SIZE
        || read_u32(bytes, 24) as usize != slot_size
        || read_u32(bytes, 28) as usize != file_size
        || bytes[32] != 2
        || bytes[33..64].iter().any(|byte| *byte != 0)
        || bytes[96..HEADER_SIZE].iter().any(|byte| *byte != 0)
    {
        return Err("invalid record header");
    }
    let mut covered = bytes[..HEADER_SIZE].to_vec();
    covered[64..96].fill(0);
    if bytes[64..96] != sha256(&covered) {
        return Err("record header checksum mismatch");
    }
    Ok(())
}

fn parse_slot(bytes: &[u8], physical_index: u8) -> Result<Option<Slot<'_>>, &'static str> {
    if bytes.iter().all(|byte| *byte == 0) {
        return Ok(None);
    }
    if bytes[0..16] != SLOT_MAGIC
        || read_u16(bytes, 16) != 1
        || read_u16(bytes, 18) != 1
        || bytes[20] != physical_index
        || bytes[22..24].iter().any(|byte| *byte != 0)
    {
        return Err("invalid provision slot envelope");
    }
    let state = bytes[21];
    if !(1..=7).contains(&state) || (state <= 6 && physical_index != (state - 1) % 2) {
        return Err("invalid provision state or slot parity");
    }
    let record_length = read_u32(bytes, 24) as usize;
    let payload_length = read_u32(bytes, 28) as usize;
    if !(1..=SLOT_SIZE - SLOT_PREFIX_SIZE).contains(&payload_length)
        || record_length != SLOT_PREFIX_SIZE + payload_length
        || bytes[record_length..].iter().any(|byte| *byte != 0)
    {
        return Err("invalid provision slot length or trailing bytes");
    }
    let payload_bytes = &bytes[SLOT_PREFIX_SIZE..record_length];
    if bytes[160..192] != sha256(payload_bytes) {
        return Err("provision payload checksum mismatch");
    }
    let mut covered = bytes[..record_length].to_vec();
    covered[192..224].fill(0);
    if bytes[192..224] != sha256(&covered) {
        return Err("provision slot checksum mismatch");
    }
    let payload: ProvisionPayload =
        serde_json::from_slice(payload_bytes).map_err(|_| "invalid provision payload JSON")?;
    if serde_json::to_vec(&payload).ok().as_deref() != Some(payload_bytes) {
        return Err("non-canonical provision payload JSON");
    }
    validate_payload_shape(&payload)?;

    let slot = Slot {
        bytes,
        physical_index,
        state,
        state_version: read_u64(bytes, 32),
        created_tick_ms: read_u64(bytes, 40),
        updated_tick_ms: read_u64(bytes, 48),
        provision_volume_serial: read_u64(bytes, 56),
        provision_file_id: bytes[64..80].try_into().expect("fixed slice"),
        machine_volume_serial: read_u64(bytes, 80),
        machine_file_id: bytes[88..104].try_into().expect("fixed slice"),
        expected_machine_length: read_u64(bytes, 104),
        observed_machine_length: read_u64(bytes, 112),
        machine_record_state_version: read_u64(bytes, 120),
        machine_epoch: read_u64(bytes, 128),
        machine_lease: read_u64(bytes, 136),
        provision_nonce: bytes[144..160].try_into().expect("fixed slice"),
        payload,
    };
    validate_slot_semantics(&slot)?;
    Ok(Some(slot))
}

fn validate_payload_shape(payload: &ProvisionPayload) -> Result<(), &'static str> {
    for digest in [
        &payload.provision_record_path_digest,
        &payload.machine_actor_path_digest,
        &payload.directory_anchor_digest,
        &payload.provision_record_dacl_digest,
        &payload.machine_actor_dacl_digest,
        &payload.provision_record_attribute_stream_digest,
        &payload.machine_actor_attribute_stream_digest,
        &payload.installer_manifest_digest,
        &payload.designated_owner_sid_digest,
    ] {
        if !is_nonzero_lower_hex(digest, 64) {
            return Err("invalid provision binding digest");
        }
    }
    for digest in [
        &payload.previous_slot_digest,
        &payload.machine_actor_header_digest,
        &payload.machine_actor_slot_digest,
        &payload.package_completion_digest,
        &payload.failure_evidence_digest,
    ] {
        if !is_lower_hex(digest, 64) {
            return Err("invalid optional provision digest");
        }
    }
    if payload.creator_lane != "0001"
        || payload.retention_mode != "0001"
        || !is_lower_hex(&payload.machine_actor_record_state, 4)
        || !is_lower_hex(&payload.failure_class, 4)
        || u16::from_str_radix(&payload.failure_class, 16).unwrap_or(u16::MAX) > 10
        || !valid_provision_actor(&payload.provision_actor)
    {
        return Err("invalid provision actor or enum");
    }
    Ok(())
}

fn valid_provision_actor(actor: &ProvisionActor) -> bool {
    is_nonzero_lower_hex(&actor.instance_id, 32)
        && is_nonzero_lower_hex(&actor.process.pid, 8)
        && is_nonzero_lower_hex(&actor.process.process_creation_time, 16)
        && is_nonzero_lower_hex(&actor.process.signed_image_identity, 64)
        && actor.process.role == "0004"
        && is_nonzero_lower_hex(&actor.process.process_nonce, 32)
}

fn validate_slot_semantics(slot: &Slot<'_>) -> Result<(), &'static str> {
    if slot.state_version == 0
        || slot.updated_tick_ms < slot.created_tick_ms
        || slot.provision_volume_serial == 0
        || slot.provision_file_id == [0; 16]
        || slot.expected_machine_length != MACHINE_ACTOR_FILE_SIZE
        || slot.provision_nonce == [0; 16]
    {
        return Err("invalid provision slot identity");
    }
    let no_failure = is_zero_hex(&slot.payload.failure_class)
        && is_zero_hex(&slot.payload.failure_evidence_digest);
    match slot.state {
        1 => {
            if slot.state_version != 1
                || !is_zero_hex(&slot.payload.previous_slot_digest)
                || !machine_link_is_absent(slot)
                || !is_zero_hex(&slot.payload.package_completion_digest)
                || !no_failure
            {
                return Err("invalid CREATE_INTENT slot");
            }
        }
        2 => {
            if !is_nonzero_lower_hex(&slot.payload.previous_slot_digest, 64)
                || slot.machine_volume_serial == 0
                || slot.machine_file_id == [0; 16]
                || !machine_observation_is_empty(slot)
                || !is_zero_hex(&slot.payload.package_completion_digest)
                || !no_failure
            {
                return Err("invalid POST_CREATE_CHECKPOINT slot");
            }
        }
        3..=5 => {
            let expected_record_state = match slot.state {
                3 => "000a",
                4 => "000b",
                _ => "0009",
            };
            if !is_nonzero_lower_hex(&slot.payload.previous_slot_digest, 64)
                || !machine_link_is_present(slot)
                || slot.payload.machine_actor_record_state != expected_record_state
                || !is_zero_hex(&slot.payload.package_completion_digest)
                || !no_failure
            {
                return Err("invalid linked MachineActor provision slot");
            }
        }
        6 => {
            if !is_nonzero_lower_hex(&slot.payload.previous_slot_digest, 64)
                || !machine_link_is_present(slot)
                || slot.payload.machine_actor_record_state != "0009"
                || !is_nonzero_lower_hex(&slot.payload.package_completion_digest, 64)
                || !no_failure
            {
                return Err("invalid TERMINAL_RETAINED slot");
            }
        }
        7 => {
            if !is_nonzero_lower_hex(&slot.payload.previous_slot_digest, 64)
                || is_zero_hex(&slot.payload.failure_class)
                || is_zero_hex(&slot.payload.failure_evidence_digest)
                || !machine_link_is_coherent(slot)
            {
                return Err("invalid FAILED_CLOSED slot");
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn machine_link_is_absent(slot: &Slot<'_>) -> bool {
    slot.machine_volume_serial == 0
        && slot.machine_file_id == [0; 16]
        && machine_observation_is_empty(slot)
}

fn machine_observation_is_empty(slot: &Slot<'_>) -> bool {
    slot.observed_machine_length == 0
        && slot.machine_record_state_version == 0
        && slot.machine_epoch == 0
        && slot.machine_lease == 0
        && is_zero_hex(&slot.payload.machine_actor_header_digest)
        && is_zero_hex(&slot.payload.machine_actor_slot_digest)
        && slot.payload.machine_actor_record_state == "0000"
}

fn machine_link_is_present(slot: &Slot<'_>) -> bool {
    slot.machine_volume_serial != 0
        && slot.machine_file_id != [0; 16]
        && slot.observed_machine_length == MACHINE_ACTOR_FILE_SIZE
        && slot.machine_record_state_version != 0
        && slot.machine_epoch != 0
        && slot.machine_lease != 0
        && is_nonzero_lower_hex(&slot.payload.machine_actor_header_digest, 64)
        && is_nonzero_lower_hex(&slot.payload.machine_actor_slot_digest, 64)
        && !is_zero_hex(&slot.payload.machine_actor_record_state)
}

fn machine_link_is_coherent(slot: &Slot<'_>) -> bool {
    machine_link_is_absent(slot)
        || (slot.machine_volume_serial != 0
            && slot.machine_file_id != [0; 16]
            && (machine_observation_is_empty(slot) || machine_link_is_present(slot)))
}

fn classify_chain(slots: &[Slot<'_>]) -> Result<ProvisionRecordClassification, &'static str> {
    let predecessor = &slots[0];
    let current = &slots[1];
    if Some(current.state_version) != predecessor.state_version.checked_add(1)
        || current.physical_index == predecessor.physical_index
        || !valid_transition(predecessor.state, current.state)
        || !same_chain_binding(predecessor, current)
        || decode_hex_32(&current.payload.previous_slot_digest) != Some(sha256(predecessor.bytes))
    {
        return Err("invalid provision predecessor chain");
    }

    match current.state {
        2 => Ok(ProvisionRecordClassification::Current(
            ProvisionState::PostCreateCheckpoint,
        )),
        3 => Ok(ProvisionRecordClassification::Current(
            ProvisionState::MachineIntentPublished,
        )),
        4 => Ok(ProvisionRecordClassification::Current(
            ProvisionState::MachineActivePublished,
        )),
        5 => Ok(ProvisionRecordClassification::Current(
            ProvisionState::MachineCleanObserved,
        )),
        6 => Ok(ProvisionRecordClassification::Current(
            ProvisionState::TerminalRetained,
        )),
        7 => Ok(ProvisionRecordClassification::FailedClosed),
        _ => Err("invalid current provision state"),
    }
}

fn valid_transition(predecessor: u8, current: u8) -> bool {
    (2..=6).contains(&current) && predecessor + 1 == current
        || current == 7 && (1..=5).contains(&predecessor)
}

/// Structural steady-state link check for INITIAL_PROVISION only (MAP states 3–6).
/// Rejects ahead/lagging/crash-resume pairs and every non-bootstrap MAR state.
/// This does NOT prove file IDs, native SID validity, checkpoint contents, live
/// actors, manifest/boot authority, durability or a right to write/clean up.
pub fn validate_candidate04_current_provision_link(
    provision: &[u8],
    machine: &[u8],
) -> Result<ProvisionState, &'static str> {
    let (classification, maps) = parse_provision_record(provision)?;
    let state = match classification {
        ProvisionRecordClassification::Current(state)
            if state != ProvisionState::PostCreateCheckpoint =>
        {
            state
        }
        _ => return Err("provision record has no supported current MachineActor link"),
    };
    let machine_slots = parse_bootstrap_machine_slots(machine)?;
    let machine_current = machine_slots.last().ok_or("no current MachineActor slot")?;
    let header_digest = sha256(&machine[..HEADER_SIZE]);
    let latest_link_version = maps
        .last()
        .ok_or("missing current provision slot")?
        .machine_record_state_version;
    for map in &maps {
        if map.state < 3 {
            continue;
        }
        let linked = machine_slots
            .iter()
            .find(|slot| slot.version == map.machine_record_state_version)
            .ok_or("linked MachineActor slot is not resident")?;
        validate_machine_link(map, linked, header_digest)?;
    }
    if latest_link_version != machine_current.version {
        return Err("MachineActor is ahead of the current provision link");
    }
    Ok(state)
}

fn validate_machine_binding(map: &Slot<'_>, machine: &MachineSlot<'_>) -> Result<(), &'static str> {
    if map.payload.provision_actor != machine.payload.operation_intent.actor
        || decode_hex::<16>(&machine.payload.operation_nonce) != Some(map.provision_nonce)
    {
        return Err("provision/MachineActor actor or nonce mismatch");
    }
    if let Some(owner) = &machine.payload.owner_sid {
        if owner.digest() != decode_hex_32(&map.payload.designated_owner_sid_digest) {
            return Err("provision/MachineActor designated owner mismatch");
        }
    }
    Ok(())
}

fn validate_machine_link(
    map: &Slot<'_>,
    machine: &MachineSlot<'_>,
    header_digest: [u8; 32],
) -> Result<(), &'static str> {
    if decode_hex_32(&map.payload.machine_actor_header_digest) != Some(header_digest)
        || decode_hex_32(&map.payload.machine_actor_slot_digest) != Some(sha256(machine.bytes))
        || map.machine_record_state_version != machine.version
        || map.machine_epoch != machine.epoch
        || map.machine_lease != machine.lease
        || map.payload.machine_actor_record_state != format!("{:04x}", machine.state)
    {
        return Err("provision/MachineActor exact link mismatch");
    }
    validate_machine_binding(map, machine)
}

/// Caller-reported observations, not verified handle identities or authorization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProvisionFileIdentity {
    pub volume_serial: u64,
    pub file_id: [u8; 16],
}

pub enum MachineFileObservation<'a> {
    /// Must ultimately come from an exact, anchored missing-leaf observation.
    Absent,
    Present {
        identity: ProvisionFileIdentity,
        bytes: &'a [u8],
    },
    Unavailable,
}

/// Structural pairs from the architecture's bootstrap crash table. No variant
/// authorizes a next write, retry, repair, cleanup, or a successful installation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvisionPairClassification {
    CreateIntentTargetAbsent,
    CheckpointTargetEmpty,
    CheckpointTargetFresh,
    CheckpointIntentObserved,
    IntentAligned,
    IntentActiveObserved,
    ActiveAligned,
    ActiveCleanObserved,
    CleanAligned,
    TerminalRetainedAligned,
}

/// Read-only structural bootstrap classifier. Trusted manifest/owner/boot/actor
/// provenance, actual handle/ACL observations, machine gate, durable checkpoint
/// and package-completion evidence remain independent requirements for any grant.
pub fn classify_candidate04_provision_pair(
    provision: &[u8],
    provision_identity: ProvisionFileIdentity,
    observation: MachineFileObservation<'_>,
) -> Result<ProvisionPairClassification, &'static str> {
    use ProvisionPairClassification::*;
    let (classification, maps) = parse_provision_record(provision)?;
    if matches!(
        classification,
        ProvisionRecordClassification::FreshUninitialized
            | ProvisionRecordClassification::FailedClosed
    ) {
        return Err("provision record has no resumable bootstrap intent");
    }
    let current = maps.last().ok_or("missing current provision slot")?;
    if provision_identity.volume_serial != current.provision_volume_serial
        || provision_identity.file_id != current.provision_file_id
    {
        return Err("observed provision file identity mismatch");
    }
    let (identity, machine) = match observation {
        MachineFileObservation::Absent if current.state == 1 => {
            return Ok(CreateIntentTargetAbsent)
        }
        MachineFileObservation::Present { identity, bytes } if current.state != 1 => {
            (identity, bytes)
        }
        _ => return Err("target absence/existence is not admissible for this provision state"),
    };
    if identity.volume_serial != current.machine_volume_serial
        || identity.file_id != current.machine_file_id
    {
        return Err("observed MachineActor file identity mismatch");
    }
    if current.state == 2 && machine.is_empty() {
        return Ok(CheckpointTargetEmpty);
    }
    let machines = parse_bootstrap_machine_slots(machine)?;
    let Some(latest) = machines.last() else {
        return if current.state == 2 {
            Ok(CheckpointTargetFresh)
        } else {
            Err("fresh MachineActor without a post-create checkpoint")
        };
    };
    let pair = match (current.state, latest.state) {
        (2, 10) => CheckpointIntentObserved,
        (3, 10) => IntentAligned,
        (3, 11) => IntentActiveObserved,
        (4, 11) => ActiveAligned,
        (4, 9) => ActiveCleanObserved,
        (5, 9) => CleanAligned,
        (6, 9) => TerminalRetainedAligned,
        _ => return Err("unsupported or multi-step-ahead bootstrap pair"),
    };
    for machine in &machines {
        validate_machine_binding(current, machine)?;
    }
    let header_digest = sha256(&machine[..HEADER_SIZE]);
    for map in &maps {
        if map.state < 3 {
            continue;
        }
        if let Some(linked) = machines
            .iter()
            .find(|slot| slot.version == map.machine_record_state_version)
        {
            validate_machine_link(map, linked, header_digest)?;
        } else if pair == ActiveCleanObserved
            && map.state == 3
            && map.machine_record_state_version == 1
            && map.machine_epoch == latest.epoch
            && map.machine_lease == latest.lease
            && decode_hex_32(&map.payload.machine_actor_header_digest) == Some(header_digest)
        {
            // Only this historical MAR A/intent slot was necessarily overwritten by
            // clean A. MAP4's exact active link is still resident and checked above.
            // Do not reconstruct or pretend to verify the overwritten intent bytes.
        } else {
            return Err("required linked MachineActor slot is not resident");
        }
    }
    Ok(pair)
}

fn parse_bootstrap_machine_slots(bytes: &[u8]) -> Result<Vec<MachineSlot<'_>>, &'static str> {
    validate_envelope_header(
        bytes,
        MACHINE_HEADER_MAGIC,
        MACHINE_SLOT_SIZE,
        MACHINE_ACTOR_FILE_SIZE as usize,
    )?;
    let mut slots = Vec::with_capacity(2);
    for index in 0..2_u8 {
        let start = HEADER_SIZE + usize::from(index) * MACHINE_SLOT_SIZE;
        let bytes = &bytes[start..start + MACHINE_SLOT_SIZE];
        if bytes.iter().all(|byte| *byte == 0) {
            continue;
        }
        let state = bytes[21];
        let version = match state {
            10 => 1,
            11 => 2,
            9 => 3,
            _ => return Err("unsupported bootstrap MachineActor state"),
        };
        let length = read_u32(bytes, 24) as usize;
        let payload_length = read_u32(bytes, 28) as usize;
        if bytes[..16] != MACHINE_SLOT_MAGIC
            || read_u16(bytes, 16) != 1
            || read_u16(bytes, 18) != 1
            || bytes[20] != index
            || u64::from(index) != (version - 1) % 2
            || bytes[22..24] != [0, 0]
            || read_u64(bytes, 32) != version
            || !(1..=32_768).contains(&payload_length)
            || length != MACHINE_PREFIX_SIZE + payload_length
            || bytes[length..].iter().any(|byte| *byte != 0)
        {
            return Err("invalid bootstrap MachineActor envelope");
        }
        let raw_payload = &bytes[MACHINE_PREFIX_SIZE..length];
        let mut covered = bytes[..length].to_vec();
        covered[104..136].fill(0);
        if bytes[72..104] != sha256(raw_payload) || bytes[104..136] != sha256(&covered) {
            return Err("MachineActor checksum mismatch");
        }
        let payload: BootstrapMachinePayload = serde_json::from_slice(raw_payload)
            .map_err(|_| "invalid bootstrap MachineActor JSON")?;
        if serde_json::to_vec(&payload).ok().as_deref() != Some(raw_payload) {
            return Err("non-canonical bootstrap MachineActor JSON");
        }
        validate_bootstrap_payload(&payload, state)?;
        let slot = MachineSlot {
            bytes,
            state,
            version,
            epoch: read_u64(bytes, 40),
            lease: read_u64(bytes, 48),
            created: read_u64(bytes, 56),
            updated: read_u64(bytes, 64),
            payload,
        };
        if slot.epoch == 0 || slot.lease == 0 || slot.updated < slot.created {
            return Err("invalid bootstrap MachineActor fence or ticks");
        }
        slots.push(slot);
    }
    slots.sort_by_key(|slot| slot.version);
    match slots.as_slice() {
        [] => {} // A valid header plus exact zero slots; caller must require checkpoint.
        [first] if first.state == 10 => {}
        [before, after]
            if before.version.checked_add(1) == Some(after.version)
                && before.epoch == after.epoch
                && before.lease == after.lease
                && before.created == after.created
                && before.updated <= after.updated
                && before.payload.boot_id == after.payload.boot_id
                && before.payload.binary_version == after.payload.binary_version
                && before.payload.recovery_binary_version
                    == after.payload.recovery_binary_version
                && before.payload.created_wall_clock == after.payload.created_wall_clock
                && before.payload.operation_intent == after.payload.operation_intent => {}
        _ => return Err("invalid resident bootstrap MachineActor chain"),
    }
    Ok(slots)
}

fn validate_bootstrap_payload(
    payload: &BootstrapMachinePayload,
    state: u8,
) -> Result<(), &'static str> {
    let intent = &payload.operation_intent;
    if !is_nonzero_lower_hex(&payload.boot_id, 64)
        || ![&payload.binary_version, &payload.recovery_binary_version]
            .iter()
            .all(|value| {
                !value.is_empty()
                    && value.len() <= 64
                    && value.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-')
                    })
            })
        || !is_lower_hex(&payload.created_wall_clock, 16)
        || !is_lower_hex(&payload.updated_wall_clock, 16)
        || !is_lower_hex(&payload.owner_terminal_digest, 64)
        || !is_zero_hex(&payload.owner_terminal_digest)
        || !is_lower_hex(&payload.terminal_generation, 16)
        || !is_zero_hex(&payload.terminal_generation)
        || payload.operation_kind != "0007"
        || intent.schema != "0001"
        || intent.kind != payload.operation_kind
        || !is_nonzero_lower_hex(&payload.operation_nonce, 32)
        || intent.operation_nonce != payload.operation_nonce
        || !valid_provision_actor(&intent.actor)
        || !is_nonzero_lower_hex(&intent.expected_record_state_version, 16)
        || ![
            &intent.target_digest,
            &intent.plan_digest,
            &intent.details_digest,
        ]
        .iter()
        .all(|value| is_nonzero_lower_hex(value, 64))
    {
        return Err("invalid bootstrap MachineActor payload binding");
    }
    let owner_zero_fields = [
        (&payload.active_display_id, 64),
        (&payload.owner_logon_id, 16),
        (&payload.owner_session_id, 8),
        (&payload.owner_wal_path_digest, 64),
        (&payload.owner_wal_generation, 16),
        (&payload.owner_wal_state, 4),
    ];
    if state != 9 {
        if payload.owner_sid.is_some()
            || payload.operation_completion.is_some()
            || owner_zero_fields.iter().any(|(value, _)| value.is_some())
        {
            return Err("forbidden owner/completion group before provisioned clean");
        }
    } else {
        let owner = payload
            .owner_sid
            .as_ref()
            .ok_or("missing provisioned owner SID")?;
        let completion = payload
            .operation_completion
            .as_ref()
            .ok_or("missing provision completion")?;
        if owner.digest().is_none()
            || !owner_zero_fields.iter().all(|(value, width)| {
                value
                    .as_ref()
                    .is_some_and(|value| is_lower_hex(value, *width) && is_zero_hex(value))
            })
            || completion.schema != intent.schema
            || completion.kind != intent.kind
            || completion.operation_nonce != intent.operation_nonce
            || completion.actor != intent.actor
            || completion.result != "0001"
            || !is_nonzero_lower_hex(&completion.provision_checkpoint_digest, 64)
        {
            return Err("invalid provisioned clean owner/completion tuple");
        }
    }
    Ok(())
}

fn same_chain_binding(predecessor: &Slot<'_>, current: &Slot<'_>) -> bool {
    let before = &predecessor.payload;
    let after = &current.payload;
    predecessor.created_tick_ms == current.created_tick_ms
        && predecessor.updated_tick_ms <= current.updated_tick_ms
        && predecessor.provision_volume_serial == current.provision_volume_serial
        && predecessor.provision_file_id == current.provision_file_id
        && predecessor.provision_nonce == current.provision_nonce
        && before.provision_record_path_digest == after.provision_record_path_digest
        && before.machine_actor_path_digest == after.machine_actor_path_digest
        && before.directory_anchor_digest == after.directory_anchor_digest
        && before.provision_record_dacl_digest == after.provision_record_dacl_digest
        && before.machine_actor_dacl_digest == after.machine_actor_dacl_digest
        && before.provision_record_attribute_stream_digest
            == after.provision_record_attribute_stream_digest
        && before.machine_actor_attribute_stream_digest
            == after.machine_actor_attribute_stream_digest
        && before.installer_manifest_digest == after.installer_manifest_digest
        && before.designated_owner_sid_digest == after.designated_owner_sid_digest
        && before.creator_lane == after.creator_lane
        && before.provision_actor == after.provision_actor
        && before.retention_mode == after.retention_mode
        && (current.state == 2
            || (predecessor.machine_volume_serial == current.machine_volume_serial
                && predecessor.machine_file_id == current.machine_file_id))
        && (current.state != 6
            || (predecessor.machine_record_state_version == current.machine_record_state_version
                && predecessor.machine_epoch == current.machine_epoch
                && predecessor.machine_lease == current.machine_lease
                && before.machine_actor_header_digest == after.machine_actor_header_digest
                && before.machine_actor_slot_digest == after.machine_actor_slot_digest
                && before.machine_actor_record_state == after.machine_actor_record_state))
        && (current.state != 7
            || (predecessor.machine_volume_serial == current.machine_volume_serial
                && predecessor.machine_file_id == current.machine_file_id
                && predecessor.observed_machine_length == current.observed_machine_length
                && predecessor.machine_record_state_version
                    == current.machine_record_state_version
                && predecessor.machine_epoch == current.machine_epoch
                && predecessor.machine_lease == current.machine_lease
                && before.machine_actor_header_digest == after.machine_actor_header_digest
                && before.machine_actor_slot_digest == after.machine_actor_slot_digest
                && before.machine_actor_record_state == after.machine_actor_record_state
                && before.package_completion_digest == after.package_completion_digest))
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().expect("fixed slice"))
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("fixed slice"))
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("fixed slice"))
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_nonzero_lower_hex(value: &str, length: usize) -> bool {
    is_lower_hex(value, length) && !is_zero_hex(value)
}

fn is_zero_hex(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte == b'0')
}

fn decode_hex_32(value: &str) -> Option<[u8; 32]> {
    decode_hex(value)
}

fn decode_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    if !is_lower_hex(value, N * 2) {
        return None;
    }
    let mut result = [0_u8; N];
    for (target, pair) in result.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
        *target = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Some(result)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

struct Slot<'a> {
    bytes: &'a [u8],
    physical_index: u8,
    state: u8,
    state_version: u64,
    created_tick_ms: u64,
    updated_tick_ms: u64,
    provision_volume_serial: u64,
    provision_file_id: [u8; 16],
    machine_volume_serial: u64,
    machine_file_id: [u8; 16],
    expected_machine_length: u64,
    observed_machine_length: u64,
    machine_record_state_version: u64,
    machine_epoch: u64,
    machine_lease: u64,
    provision_nonce: [u8; 16],
    payload: ProvisionPayload,
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProvisionPayload {
    provision_record_path_digest: String,
    machine_actor_path_digest: String,
    directory_anchor_digest: String,
    provision_record_dacl_digest: String,
    machine_actor_dacl_digest: String,
    provision_record_attribute_stream_digest: String,
    machine_actor_attribute_stream_digest: String,
    installer_manifest_digest: String,
    designated_owner_sid_digest: String,
    creator_lane: String,
    provision_actor: ProvisionActor,
    previous_slot_digest: String,
    machine_actor_header_digest: String,
    machine_actor_slot_digest: String,
    machine_actor_record_state: String,
    package_completion_digest: String,
    retention_mode: String,
    failure_class: String,
    failure_evidence_digest: String,
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProvisionActor {
    instance_id: String,
    process: ProvisionProcess,
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProvisionProcess {
    pid: String,
    process_creation_time: String,
    signed_image_identity: String,
    role: String,
    process_nonce: String,
}

struct MachineSlot<'a> {
    bytes: &'a [u8],
    state: u8,
    version: u64,
    epoch: u64,
    lease: u64,
    created: u64,
    updated: u64,
    payload: BootstrapMachinePayload,
}

// INITIAL_PROVISION subset only. Other state families/optional E groups are
// intentionally rejected; adding a general maintenance decoder is separate work.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BootstrapMachinePayload {
    boot_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_display_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_sid: Option<OwnerSidValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_logon_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_wal_path_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_wal_generation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_wal_state: Option<String>,
    owner_terminal_digest: String,
    binary_version: String,
    recovery_binary_version: String,
    created_wall_clock: String,
    updated_wall_clock: String,
    operation_kind: String,
    operation_nonce: String,
    operation_intent: BootstrapIntent,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_completion: Option<BootstrapCompletion>,
    terminal_generation: String,
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BootstrapIntent {
    schema: String,
    kind: String,
    operation_nonce: String,
    actor: ProvisionActor,
    expected_record_state_version: String,
    target_digest: String,
    plan_digest: String,
    details_digest: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BootstrapCompletion {
    schema: String,
    kind: String,
    operation_nonce: String,
    result: String,
    actor: ProvisionActor,
    provision_checkpoint_digest: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OwnerSidValue {
    kind: String,
    byte_length: String,
    bytes: String,
}

impl OwnerSidValue {
    fn digest(&self) -> Option<[u8; 32]> {
        if self.kind != "0001" || !is_lower_hex(&self.byte_length, 8) {
            return None;
        }
        let length = usize::from_str_radix(&self.byte_length, 16).ok()?;
        let bytes = decode_hex::<68>(&self.bytes)?;
        if !(8..=68).contains(&length) || bytes[length..].iter().any(|byte| *byte != 0) {
            return None;
        }
        crate::machine_storage::candidate04_owner_sid_digest(&bytes[..length])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes")
                .join(name),
        )
        .unwrap()
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    // Test-only memory rewriting. Never generates or changes qualified artifacts.
    fn replace_payload(slot: &mut [u8], prefix: usize, payload: &[u8]) {
        slot[prefix..].fill(0);
        slot[24..28].copy_from_slice(&((prefix + payload.len()) as u32).to_le_bytes());
        slot[28..32].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        slot[prefix..prefix + payload.len()].copy_from_slice(payload);
        slot[prefix - 64..prefix - 32].copy_from_slice(&sha256(payload));
        slot[prefix - 32..prefix].fill(0);
        let checksum = sha256(&slot[..prefix + payload.len()]);
        slot[prefix - 32..prefix].copy_from_slice(&checksum);
    }

    fn memory_bootstrap_pair(state: u8) -> (Vec<u8>, Vec<u8>) {
        let mut map = fixture(&format!("MAPRV1-P-STATE-{state:02}.bin"));
        let current_start = HEADER_SIZE + usize::from((state - 1) % 2) * SLOT_SIZE;
        let map_current = parse_slot(
            &map[current_start..current_start + SLOT_SIZE],
            (state - 1) % 2,
        )
        .unwrap()
        .unwrap();
        let nonce = hex(&map_current.provision_nonce);
        let actor = serde_json::to_vec(&map_current.payload.provision_actor).unwrap();
        let mut machine = fixture("MARV1-P-STATE-0A.bin");
        machine[HEADER_SIZE..].fill(0);
        for version in 1..=(state - 2).min(3) {
            let template = fixture(match version {
                1 => "MARV1-P-STATE-0A.bin",
                2 => "MARV1-P-STATE-0B.bin",
                _ => "MARV1-P-PROVISIONED-CLEAN.bin",
            });
            let mut slot = template[HEADER_SIZE..HEADER_SIZE + MACHINE_SLOT_SIZE].to_vec();
            let mut payload: BootstrapMachinePayload =
                serde_json::from_slice(&slot[MACHINE_PREFIX_SIZE..read_u32(&slot, 24) as usize])
                    .unwrap();
            payload.operation_nonce = nonce.clone();
            payload.operation_intent.operation_nonce = nonce.clone();
            payload.operation_intent.actor = serde_json::from_slice(&actor).unwrap();
            if let Some(completion) = &mut payload.operation_completion {
                completion.operation_nonce = nonce.clone();
                completion.actor = serde_json::from_slice(&actor).unwrap();
            }
            slot[20] = (version - 1) % 2;
            slot[32..40].copy_from_slice(&u64::from(version).to_le_bytes());
            slot[64..72].copy_from_slice(&u64::from(version).to_le_bytes());
            replace_payload(
                &mut slot,
                MACHINE_PREFIX_SIZE,
                &serde_json::to_vec(&payload).unwrap(),
            );
            let start = HEADER_SIZE + usize::from((version - 1) % 2) * MACHINE_SLOT_SIZE;
            machine[start..start + MACHINE_SLOT_SIZE].copy_from_slice(&slot);
        }
        let mut prior_digest = None;
        for map_state in state - 1..=state {
            let index = (map_state - 1) % 2;
            let start = HEADER_SIZE + usize::from(index) * SLOT_SIZE;
            let mut slot = map[start..start + SLOT_SIZE].to_vec();
            let mut payload: ProvisionPayload =
                serde_json::from_slice(&slot[SLOT_PREFIX_SIZE..read_u32(&slot, 24) as usize])
                    .unwrap();
            if map_state >= 3 {
                let version = (map_state - 2).min(3);
                let linked_start = HEADER_SIZE + usize::from((version - 1) % 2) * MACHINE_SLOT_SIZE;
                slot[120..128].copy_from_slice(&u64::from(version).to_le_bytes());
                payload.machine_actor_header_digest = hex(&sha256(&machine[..HEADER_SIZE]));
                payload.machine_actor_slot_digest = hex(&sha256(
                    &machine[linked_start..linked_start + MACHINE_SLOT_SIZE],
                ));
            }
            if let Some(digest) = prior_digest {
                payload.previous_slot_digest = digest;
            }
            replace_payload(
                &mut slot,
                SLOT_PREFIX_SIZE,
                &serde_json::to_vec(&payload).unwrap(),
            );
            prior_digest = Some(hex(&sha256(&slot)));
            map[start..start + SLOT_SIZE].copy_from_slice(&slot);
        }
        (map, machine)
    }

    #[test]
    fn candidate04_bootstrap_current_link_contract() {
        for (state, expected) in [
            (3, ProvisionState::MachineIntentPublished),
            (4, ProvisionState::MachineActivePublished),
            (5, ProvisionState::MachineCleanObserved),
            (6, ProvisionState::TerminalRetained),
        ] {
            let (map, machine) = memory_bootstrap_pair(state);
            assert_eq!(
                validate_candidate04_current_provision_link(&map, &machine),
                Ok(expected)
            );
            for length in [0, 16, HEADER_SIZE, machine.len() - 1] {
                assert!(
                    validate_candidate04_current_provision_link(&map, &machine[..length]).is_err()
                );
            }
            let original_map = fixture(&format!("MAPRV1-P-STATE-{state:02}.bin"));
            let original_mar = fixture(match state {
                3 => "MARV1-P-STATE-0A.bin",
                4 => "MARV1-P-STATE-0B.bin",
                _ => "MARV1-P-PROVISIONED-CLEAN.bin",
            });
            // These layout fixtures are not exact actor/nonce/version bootstrap chains.
            assert!(
                validate_candidate04_current_provision_link(&original_map, &original_mar).is_err()
            );
        }
        let (map, machine) = memory_bootstrap_pair(3);
        let (_, ahead) = memory_bootstrap_pair(4);
        assert!(validate_candidate04_current_provision_link(&map, &ahead).is_err());
        let raw = &machine[HEADER_SIZE + MACHINE_PREFIX_SIZE
            ..HEADER_SIZE + read_u32(&machine, HEADER_SIZE + 24) as usize];
        for (offset, value) in [
            (0, 0),
            (16, 2),
            (18, 2),
            (20, 1),
            (21, 1),
            (22, 1),
            (32, 2),
            (40, 0),
            (48, 0),
            (56, 2),
        ] {
            let mut bad = machine.clone();
            bad[HEADER_SIZE + offset] = value;
            replace_payload(
                &mut bad[HEADER_SIZE..HEADER_SIZE + MACHINE_SLOT_SIZE],
                MACHINE_PREFIX_SIZE,
                raw,
            );
            assert!(
                parse_bootstrap_machine_slots(&bad).is_err(),
                "offset {offset}"
            );
        }
        for offset in [HEADER_SIZE + 72, HEADER_SIZE + 104, machine.len() - 1] {
            let mut bad = machine.clone();
            bad[offset] ^= 1;
            assert!(parse_bootstrap_machine_slots(&bad).is_err());
        }
        let text = std::str::from_utf8(raw).unwrap();
        for changed in [
            format!("{text}\n"),
            text.replacen("\"bootId\":", "\"extra\":\"x\",\"bootId\":", 1),
            text.replacen("\"bootId\":", "\"bootId\":\"duplicate\",\"bootId\":", 1),
            text.replacen("\"bootId\"", "\"\\u0062ootId\"", 1),
            text.replacen(
                "\"binaryVersion\":",
                "\"ownerSid\":null,\"binaryVersion\":",
                1,
            ),
            text.replacen(
                "\"operationKind\":\"0007\"",
                "\"operationKind\":\"0003\"",
                1,
            ),
            text.replacen("\"role\":\"0004\"", "\"role\":\"0002\"", 1),
            text.replacen("\"schema\":\"0001\"", "\"schema\":\"0002\"", 1),
        ] {
            let mut bad = machine.clone();
            replace_payload(
                &mut bad[HEADER_SIZE..HEADER_SIZE + MACHINE_SLOT_SIZE],
                MACHINE_PREFIX_SIZE,
                changed.as_bytes(),
            );
            assert!(parse_bootstrap_machine_slots(&bad).is_err());
        }
        let (map, machine) = memory_bootstrap_pair(5);
        let current = &machine[HEADER_SIZE..HEADER_SIZE + MACHINE_SLOT_SIZE];
        for case in 0..9 {
            let mut payload: BootstrapMachinePayload = serde_json::from_slice(
                &current[MACHINE_PREFIX_SIZE..read_u32(current, 24) as usize],
            )
            .unwrap();
            let completion = payload.operation_completion.as_mut().unwrap();
            match case {
                0 => completion.schema = "0002".into(),
                1 => completion.kind = "0003".into(),
                2 => completion.operation_nonce = "ab".repeat(16),
                3 => completion.actor.process.pid = "00001001".into(),
                4 => completion.result = "0007".into(),
                5 => completion.provision_checkpoint_digest = "0".repeat(64),
                6 => payload.owner_wal_state = Some("0008".into()),
                7 => payload.active_display_id = None,
                _ => payload.operation_intent.expected_record_state_version = "0".repeat(16),
            }
            let mut bad = machine.clone();
            replace_payload(
                &mut bad[HEADER_SIZE..HEADER_SIZE + MACHINE_SLOT_SIZE],
                MACHINE_PREFIX_SIZE,
                &serde_json::to_vec(&payload).unwrap(),
            );
            assert!(
                parse_bootstrap_machine_slots(&bad).is_err(),
                "completion/owner case {case}"
            );
        }
        let mut owner_mismatch: BootstrapMachinePayload =
            serde_json::from_slice(&current[MACHINE_PREFIX_SIZE..read_u32(current, 24) as usize])
                .unwrap();
        owner_mismatch
            .owner_sid
            .as_mut()
            .unwrap()
            .bytes
            .replace_range(14..16, "06");
        let mut different_owner = machine.clone();
        replace_payload(
            &mut different_owner[HEADER_SIZE..HEADER_SIZE + MACHINE_SLOT_SIZE],
            MACHINE_PREFIX_SIZE,
            &serde_json::to_vec(&owner_mismatch).unwrap(),
        );
        // Rebind the slot hash, so rejection must reach the D03 owner link check.
        let mut map_slot = map[HEADER_SIZE..HEADER_SIZE + SLOT_SIZE].to_vec();
        let mut payload: ProvisionPayload =
            serde_json::from_slice(&map_slot[SLOT_PREFIX_SIZE..read_u32(&map_slot, 24) as usize])
                .unwrap();
        payload.machine_actor_slot_digest = hex(&sha256(
            &different_owner[HEADER_SIZE..HEADER_SIZE + MACHINE_SLOT_SIZE],
        ));
        replace_payload(
            &mut map_slot,
            SLOT_PREFIX_SIZE,
            &serde_json::to_vec(&payload).unwrap(),
        );
        let mut map = map;
        map[HEADER_SIZE..HEADER_SIZE + SLOT_SIZE].copy_from_slice(&map_slot);
        assert_eq!(
            validate_candidate04_current_provision_link(&map, &different_owner),
            Err("provision/MachineActor designated owner mismatch")
        );
    }

    #[test]
    fn candidate04_bootstrap_crash_pair_matrix() {
        use ProvisionPairClassification::*;
        let maps = [
            fixture("MAPRV1-P-STATE-01.bin"),
            fixture("MAPRV1-P-STATE-02.bin"),
            memory_bootstrap_pair(3).0,
            memory_bootstrap_pair(4).0,
            memory_bootstrap_pair(5).0,
            memory_bootstrap_pair(6).0,
        ];
        let intent = memory_bootstrap_pair(3).1;
        let mut fresh = intent.clone();
        fresh[HEADER_SIZE..].fill(0);
        let machines = [
            Vec::new(),
            fresh,
            intent,
            memory_bootstrap_pair(4).1,
            memory_bootstrap_pair(5).1,
        ];
        let identities = |map: &[u8]| {
            let (_, slots) = parse_provision_record(map).unwrap();
            let current = slots.last().unwrap();
            (
                ProvisionFileIdentity {
                    volume_serial: current.provision_volume_serial,
                    file_id: current.provision_file_id,
                },
                ProvisionFileIdentity {
                    volume_serial: current.machine_volume_serial,
                    file_id: current.machine_file_id,
                },
            )
        };
        // Columns: absent, unreadable, empty, fresh, intent, active, clean.
        for (row, map) in maps.iter().enumerate() {
            let (provision_id, machine_id) = identities(map);
            for column in 0..7 {
                let observation = || match column {
                    0 => MachineFileObservation::Absent,
                    1 => MachineFileObservation::Unavailable,
                    _ => MachineFileObservation::Present {
                        identity: machine_id,
                        bytes: &machines[column - 2],
                    },
                };
                let expected = match (row + 1, column) {
                    (1, 0) => Some(CreateIntentTargetAbsent),
                    (2, 2) => Some(CheckpointTargetEmpty),
                    (2, 3) => Some(CheckpointTargetFresh),
                    (2, 4) => Some(CheckpointIntentObserved),
                    (3, 4) => Some(IntentAligned),
                    (3, 5) => Some(IntentActiveObserved),
                    (4, 5) => Some(ActiveAligned),
                    (4, 6) => Some(ActiveCleanObserved),
                    (5, 6) => Some(CleanAligned),
                    (6, 6) => Some(TerminalRetainedAligned),
                    _ => None,
                };
                assert_eq!(
                    classify_candidate04_provision_pair(map, provision_id, observation()).ok(),
                    expected,
                    "MAP{} / column {column}",
                    row + 1
                );
                let mut wrong_id = provision_id;
                wrong_id.file_id[0] ^= 1;
                assert!(classify_candidate04_provision_pair(map, wrong_id, observation()).is_err());
                if column >= 2 {
                    let mut wrong_id = machine_id;
                    wrong_id.volume_serial ^= 1;
                    assert!(classify_candidate04_provision_pair(
                        map,
                        provision_id,
                        MachineFileObservation::Present {
                            identity: wrong_id,
                            bytes: &machines[column - 2]
                        }
                    )
                    .is_err());
                }
            }
        }
        let (provision_id, machine_id) = identities(&maps[1]);
        let mut corrupt = machines[1].clone();
        corrupt[64] ^= 1;
        for invalid in [
            vec![0; MACHINE_ACTOR_FILE_SIZE as usize],
            vec![0],
            machines[1][..HEADER_SIZE].to_vec(),
            corrupt,
        ] {
            assert!(classify_candidate04_provision_pair(
                &maps[1],
                provision_id,
                MachineFileObservation::Present {
                    identity: machine_id,
                    bytes: &invalid
                }
            )
            .is_err());
        }
        let uninitialized = fixture("MAPRV1-P-000.bin");
        assert!(classify_candidate04_provision_pair(
            &uninitialized,
            provision_id,
            MachineFileObservation::Absent
        )
        .is_err());
        // Valid FAILED_CLOSED after checkpoint remains blocked, including an empty target.
        let mut failed = maps[1].clone();
        let previous = &maps[1][HEADER_SIZE + SLOT_SIZE..];
        let mut slot = previous.to_vec();
        slot[20] = 0;
        slot[21] = 7;
        slot[32..40].copy_from_slice(&3_u64.to_le_bytes());
        let mut payload: ProvisionPayload =
            serde_json::from_slice(&slot[SLOT_PREFIX_SIZE..read_u32(&slot, 24) as usize]).unwrap();
        payload.previous_slot_digest = hex(&sha256(previous));
        payload.failure_class = "0002".into();
        payload.failure_evidence_digest = "ab".repeat(32);
        replace_payload(
            &mut slot,
            SLOT_PREFIX_SIZE,
            &serde_json::to_vec(&payload).unwrap(),
        );
        failed[HEADER_SIZE..HEADER_SIZE + SLOT_SIZE].copy_from_slice(&slot);
        assert_eq!(
            classify_candidate04_maprv1(&failed),
            Ok(ProvisionRecordClassification::FailedClosed)
        );
        assert!(classify_candidate04_provision_pair(
            &failed,
            provision_id,
            MachineFileObservation::Present {
                identity: machine_id,
                bytes: &[]
            }
        )
        .is_err());

        // The one-step-ahead clean case must still verify the resident active slot.
        let (provision_id, machine_id) = identities(&maps[3]);
        let mut changed = machines[4].clone();
        let active_start = HEADER_SIZE + MACHINE_SLOT_SIZE;
        changed[active_start + 64..active_start + 72].copy_from_slice(&1_u64.to_le_bytes());
        let raw = changed[active_start + MACHINE_PREFIX_SIZE
            ..active_start + read_u32(&changed, active_start + 24) as usize]
            .to_vec();
        replace_payload(&mut changed[active_start..], MACHINE_PREFIX_SIZE, &raw);
        assert!(parse_bootstrap_machine_slots(&changed).is_ok());
        assert_eq!(
            classify_candidate04_provision_pair(
                &maps[3],
                provision_id,
                MachineFileObservation::Present {
                    identity: machine_id,
                    bytes: &changed
                }
            ),
            Err("provision/MachineActor exact link mismatch")
        );
        // Overwritten history is not an exemption for a contradictory epoch.
        let mut bad_history = maps[3].clone();
        bad_history[HEADER_SIZE + 128..HEADER_SIZE + 136].copy_from_slice(&2_u64.to_le_bytes());
        refresh_checksum(&mut bad_history[HEADER_SIZE..HEADER_SIZE + SLOT_SIZE]);
        let prior_digest = hex(&sha256(&bad_history[HEADER_SIZE..HEADER_SIZE + SLOT_SIZE]));
        let current = &mut bad_history[HEADER_SIZE + SLOT_SIZE..];
        let mut payload: ProvisionPayload =
            serde_json::from_slice(&current[SLOT_PREFIX_SIZE..read_u32(current, 24) as usize])
                .unwrap();
        payload.previous_slot_digest = prior_digest;
        replace_payload(
            current,
            SLOT_PREFIX_SIZE,
            &serde_json::to_vec(&payload).unwrap(),
        );
        assert!(classify_candidate04_maprv1(&bad_history).is_ok());
        assert!(classify_candidate04_provision_pair(
            &bad_history,
            provision_id,
            MachineFileObservation::Present {
                identity: machine_id,
                bytes: &machines[4]
            }
        )
        .is_err());
    }

    #[test]
    fn candidate04_owner_sid_digest_known_answers() {
        for name in [
            "OWNERSIDV1-P-001.bin",
            "OWNERSIDV1-P-002.bin",
            "OWNERSIDV1-P-003.bin",
        ] {
            let vector: serde_json::Value = serde_json::from_slice(&fixture(name)).unwrap();
            let sid: OwnerSidValue = serde_json::from_value(vector["sid"].clone()).unwrap();
            assert_eq!(
                hex(&sid.digest().unwrap()),
                vector["ownerSidDigest"].as_str().unwrap()
            );
        }
        let sid = [1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0];
        let digest = crate::machine_storage::candidate04_owner_sid_digest(&sid).unwrap();
        assert_ne!(digest, sha256(&sid));
        for length in 0..sid.len() {
            assert!(crate::machine_storage::candidate04_owner_sid_digest(&sid[..length]).is_none());
        }
        assert!(crate::machine_storage::candidate04_owner_sid_digest(&[1; 69]).is_none());
    }

    #[test]
    fn candidate_04_maprv1_fixture_contract() {
        let positives: &[(&[u8], ProvisionRecordClassification)] = &[
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-P-000.bin"
                )),
                ProvisionRecordClassification::FreshUninitialized,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-P-STATE-01.bin"
                )),
                ProvisionRecordClassification::UnqualifiedCreateIntent,
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-P-STATE-02.bin"
                )),
                ProvisionRecordClassification::Current(ProvisionState::PostCreateCheckpoint),
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-P-STATE-03.bin"
                )),
                ProvisionRecordClassification::Current(ProvisionState::MachineIntentPublished),
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-P-STATE-04.bin"
                )),
                ProvisionRecordClassification::Current(ProvisionState::MachineActivePublished),
            ),
            (
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-P-STATE-05.bin"
                )),
                ProvisionRecordClassification::Current(ProvisionState::MachineCleanObserved),
            ),
        ];
        for (bytes, expected) in positives {
            assert_eq!(classify_candidate04_maprv1(bytes), Ok(*expected));
        }

        let contextual_state_1: &[&[u8]] = &[
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-N-002.bin"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-N-003.bin"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-N-004.bin"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-N-005.bin"
            )),
        ];
        for bytes in contextual_state_1 {
            assert_eq!(
                classify_candidate04_maprv1(bytes),
                Ok(ProvisionRecordClassification::UnqualifiedCreateIntent)
            );
        }

        let invalid: &[&[u8]] = &[
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-N-001.bin"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-N-006.bin"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-N-007.bin"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-N-008.bin"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-N-009.bin"
            )),
        ];
        for bytes in invalid {
            assert!(classify_candidate04_maprv1(bytes).is_err());
        }

        // The retained Candidate 04 fixture changes the linked clean record version
        // from 3 to 4, although state 6 must carry the same clean link as state 5.
        let terminal = include_bytes!(
            "../../../fixtures/dd-fr-002-wire-v1-candidate-04/bytes/MAPRV1-P-STATE-06.bin"
        );
        assert!(classify_candidate04_maprv1(terminal).is_err());
        let mut corrected = terminal.to_vec();
        let start = HEADER_SIZE + SLOT_SIZE;
        corrected[start + 120..start + 128].copy_from_slice(&3_u64.to_le_bytes());
        refresh_checksum(&mut corrected[start..]);
        assert_eq!(
            classify_candidate04_maprv1(&corrected),
            Ok(ProvisionRecordClassification::Current(
                ProvisionState::TerminalRetained
            ))
        );
        for length in [0, 16, HEADER_SIZE, FILE_SIZE - 1] {
            assert!(classify_candidate04_maprv1(&corrected[..length]).is_err());
        }
        // Authenticated-looking hostile versions must reject without arithmetic panic.
        for (index, version) in [(0, u64::MAX), (1, u64::MAX)] {
            let offset = HEADER_SIZE + index * SLOT_SIZE;
            corrected[offset + 32..offset + 40].copy_from_slice(&version.to_le_bytes());
            refresh_checksum(&mut corrected[offset..offset + SLOT_SIZE]);
        }
        assert!(classify_candidate04_maprv1(&corrected).is_err());
    }

    fn refresh_checksum(slot: &mut [u8]) {
        let length = read_u32(slot, 24) as usize;
        slot[192..224].fill(0);
        let checksum = sha256(&slot[..length]);
        slot[192..224].copy_from_slice(&checksum);
    }
}
