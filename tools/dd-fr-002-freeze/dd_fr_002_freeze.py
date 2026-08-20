#!/usr/bin/env python3
"""Deterministic DD-FR-002 candidate-wire evidence generator.

This is an offline freeze-artifact tool, not DisplayDeck product code and not a
Phase 2A serializer, watchdog, worker, or Windows API implementation. It only
materializes stable candidate vectors and verifies their index and digests.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import shutil
from collections import OrderedDict
from pathlib import Path
from typing import Any, Iterable

PROFILE = "DD-FR-002-WIRE-PROFILE-V1-CANDIDATE-04"
ARTIFACT_ID = "dd-fr-002-wire-v1-candidate-04"
TOOL_ID = "DD-FR-002-FREEZE-GENERATOR-V1"
ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT = ROOT / "fixtures" / ARTIFACT_ID
DJ_FILE_LENGTH, DJ_HEADER_SIZE, DJ_SLOT_SIZE = 12_288, 4_096, 4_096
MAR_FILE_LENGTH, MAR_HEADER_SIZE, MAR_SLOT_SIZE = 135_168, 4_096, 65_536
MAP_FILE_LENGTH, MAP_HEADER_SIZE, MAP_SLOT_SIZE = 12_288, 4_096, 4_096
ZERO32, ZERO16, ZERO8, ZERO4 = "0" * 64, "0" * 32, "0" * 16, "0" * 8


def digest(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


def hex_digest(label: str) -> str:
    return digest(b"DisplayDeck.DD-FR-002.VectorSeed.V1\0" + label.encode("ascii")).hex()


def hex_id(label: str) -> str:
    return digest(b"DisplayDeck.DD-FR-002.IdSeed.V1\0" + label.encode("ascii"))[:16].hex()


def canonical_json(value: Any) -> bytes:
    encoded = json.dumps(value, ensure_ascii=True, separators=(",", ":"), allow_nan=False)
    if any(ord(character) > 0x7F for character in encoded):
        raise ValueError("candidate canonical JSON must be ASCII")
    return encoded.encode("ascii")


def write_u16(buffer: bytearray, offset: int, value: int) -> None:
    buffer[offset : offset + 2] = value.to_bytes(2, "little")


def write_u32(buffer: bytearray, offset: int, value: int) -> None:
    buffer[offset : offset + 4] = value.to_bytes(4, "little")


def write_u64(buffer: bytearray, offset: int, value: int) -> None:
    buffer[offset : offset + 8] = value.to_bytes(8, "little")


def envelope_header(magic: bytes, slot_size: int, file_length: int) -> bytes:
    header = bytearray(DJ_HEADER_SIZE)
    header[0:16] = magic
    write_u16(header, 16, 1)
    write_u16(header, 18, 1)
    write_u32(header, 20, DJ_HEADER_SIZE)
    write_u32(header, 24, slot_size)
    write_u32(header, 28, file_length)
    header[32] = 2
    header[64:96] = digest(bytes(header))
    return bytes(header)


def record_file(header: bytes, slot_size: int, slots: dict[int, bytes]) -> bytes:
    result = bytearray(len(header) + slot_size * 2)
    result[: len(header)] = header
    for slot_index, slot in slots.items():
        offset = DJ_HEADER_SIZE + slot_index * slot_size
        result[offset : offset + slot_size] = slot
    return bytes(result)


def dj_slot(slot_index: int, decision: int, *, tick_zero: bool = False) -> bytes:
    slot = bytearray(DJ_SLOT_SIZE)
    slot[0:16] = bytes.fromhex("444a534c4f5456310000000000000000")
    write_u16(slot, 16, 1)
    write_u16(slot, 18, 1)
    slot[20], slot[21] = slot_index, decision
    write_u32(slot, 24, 440)
    generation, previous = (1, 0) if decision == 1 else (2, 1)
    write_u64(slot, 32, generation)
    write_u64(slot, 40, previous)
    write_u64(slot, 48, generation)
    write_u64(slot, 56, 1)
    slot[64:80] = bytes.fromhex(hex_id("session"))
    slot[80:112] = bytes.fromhex(hex_digest("boot"))
    slot[112:128] = bytes.fromhex(hex_id("controller"))
    slot[128:144] = bytes.fromhex(hex_id("watchdog"))
    slot[144:176] = bytes.fromhex(hex_digest("display"))
    slot[176:208] = bytes.fromhex(owner_sid_digest())
    write_u64(slot, 208, 0x1020304050607080)
    if decision == 1:
        write_u64(slot, 240, 0 if tick_zero else 7)
        slot[312:344] = bytes.fromhex(hex_digest("previous-mode"))
        slot[344:376] = bytes.fromhex(hex_digest("rollback"))
    else:
        write_u64(slot, 216, 10)
        write_u64(slot, 224, 10)
        write_u64(slot, 232, 11)
        slot[248:280] = bytes.fromhex(hex_digest("candidate"))
        slot[280:312] = bytes.fromhex(hex_digest("expected-mode"))
    slot[376:408] = digest(bytes(slot[64:376]))
    zeroed = bytearray(slot[:440])
    zeroed[408:440] = b"\0" * 32
    slot[408:440] = digest(bytes(zeroed))
    return bytes(slot)


def dj_bytes(vector_id: str) -> bytes:
    header = envelope_header(bytes.fromhex("4444444a563100000000000000000000"), DJ_SLOT_SIZE, DJ_FILE_LENGTH)
    if vector_id == "DJV1-P-001":
        return record_file(header, DJ_SLOT_SIZE, {})
    if vector_id == "DJV1-P-002":
        return record_file(header, DJ_SLOT_SIZE, {0: dj_slot(0, 1), 1: dj_slot(1, 2)})
    if vector_id == "DJV1-P-003":
        return record_file(header, DJ_SLOT_SIZE, {0: dj_slot(0, 2), 1: dj_slot(1, 1)})
    if vector_id == "DJV1-P-004":
        return record_file(header, DJ_SLOT_SIZE, {0: dj_slot(0, 1, tick_zero=True)})
    number = int(vector_id.rsplit("-", 1)[1])
    base = bytearray(record_file(header, DJ_SLOT_SIZE, {0: dj_slot(0, 1), 1: dj_slot(1, 2)} if number in (5, 10, 11, 12, 13) else {0: dj_slot(0, 1)}))
    if number == 1:
        return bytes(base[:-1])
    if number == 2:
        base[64] ^= 0x01
    elif number == 3:
        base[DJ_HEADER_SIZE + 20] = 1
    elif number == 4:
        base[DJ_HEADER_SIZE + 40] = 1
    elif number == 5:
        base[DJ_HEADER_SIZE + DJ_SLOT_SIZE + 176] ^= 0x01
    elif number == 6:
        return record_file(header, DJ_SLOT_SIZE, {})
    elif number == 7:
        base[DJ_HEADER_SIZE + 21] = 0x7F
    elif number == 8:
        base[DJ_HEADER_SIZE + 22] = 1
    elif number == 9:
        base[DJ_HEADER_SIZE + 440] = 1
    elif number == 10:
        base[DJ_HEADER_SIZE + DJ_SLOT_SIZE + 64] ^= 1
    elif number == 11:
        base[DJ_HEADER_SIZE + DJ_SLOT_SIZE + 80] ^= 1
    elif number == 12:
        base[DJ_HEADER_SIZE + DJ_SLOT_SIZE + 144] ^= 1
    elif number == 13:
        base[DJ_HEADER_SIZE + DJ_SLOT_SIZE + 56] = 2
    if number in (3, 4, 7, 8):
        refresh_dj_slot_integrity(base, DJ_HEADER_SIZE)
    if number in (5, 10, 11, 12, 13):
        refresh_dj_slot_integrity(base, DJ_HEADER_SIZE + DJ_SLOT_SIZE)
    return bytes(base)


DJ_NEGATIVE_CASES = {
    "DJV1-N-001": ("truncated fixed-length file", "recordLength/physical length short; integrity unavailable"),
    "DJV1-N-002": ("dedicated header checksum mismatch", "checksum byte changed; checksum intentionally not repaired"),
    "DJV1-N-003": ("slot A carries physical slotIndex B", "slot checksum repaired"),
    "DJV1-N-004": ("generation=1 with previousGeneration=1", "slot checksum repaired"),
    "DJV1-N-005": ("foreign ownerSidDigest", "slot checksum repaired"),
    "DJV1-N-006": ("fresh zero-slot file with external post-create checkpoint absent", "no byte mutation; external prerequisite absent"),
    "DJV1-N-007": ("unknown decision enum", "decision byte=0x7f; slot checksum repaired"),
    "DJV1-N-008": ("nonzero reserved field", "reserved byte=0x01; slot checksum repaired"),
    "DJV1-N-009": ("trailing nonzero slot byte", "trailing byte=0x01; reserved/trailing reject"),
    "DJV1-N-010": ("foreign session identity", "sessionId byte changed; slot checksum repaired"),
    "DJV1-N-011": ("foreign boot identity", "bootId byte changed; slot checksum repaired"),
    "DJV1-N-012": ("foreign display identity", "displayId byte changed; slot checksum repaired"),
    "DJV1-N-013": ("foreign lease identity", "leaseVersion changed; slot checksum repaired"),
}


def refresh_dj_slot_integrity(file_bytes: bytearray, start: int) -> None:
    file_bytes[start + 376 : start + 408] = digest(bytes(file_bytes[start + 64 : start + 376]))
    covered = bytearray(file_bytes[start : start + 440])
    covered[408:440] = b"\0" * 32
    file_bytes[start + 408 : start + 440] = digest(bytes(covered))


def process_identity(label: str, role: int) -> OrderedDict[str, str]:
    return OrderedDict((
        ("pid", "00001000"),
        ("processCreationTime", "01d0000000000000"),
        ("signedImageIdentity", hex_digest(label + "-image")),
        ("role", f"{role:04x}"),
        ("processNonce", hex_id(label + "-nonce")),
    ))


def actor_ref(label: str, role: int) -> OrderedDict[str, Any]:
    return OrderedDict((("instanceId", hex_id(label + "-instance")), ("process", process_identity(label, role))))


def sid_value(*, length: int = 8, tail_byte: int = 0, kind: str = "0001") -> OrderedDict[str, str]:
    """Encode a real binary SID in the fixed 68-byte SidValueV1 capacity.

    The SID binary header is revision=1, subauthority-count, NT authority=5,
    followed by little-endian u32 subauthorities.  The only admissible lengths
    are consequently 8 + 4*n. The short boundary is the valid no-
    subauthority SID header (8 bytes); the long boundary has fifteen
    subauthorities (68 bytes).
    """
    if length < 8 or length > 68 or (length - 8) % 4:
        raise ValueError("SID length must be 8 + 4*n")
    count = (length - 8) // 4
    raw = bytearray(68)
    raw[0], raw[1] = 1, count
    raw[2:8] = bytes.fromhex("000000000005")
    for index in range(count):
        write_u32(raw, 8 + index * 4, 18 if index == 0 else index)
    if tail_byte:
        raw[length:] = bytes([tail_byte]) * (68 - length)
    return OrderedDict((("kind", kind), ("byteLength", f"{length:08x}"), ("bytes", bytes(raw).hex())))


def actual_sid_bytes(value: OrderedDict[str, str] | dict[str, str] | None = None) -> bytes:
    sid = sid_value() if value is None else value
    if sid.get("kind") != "0001":
        raise ValueError("only present SID is valid in this candidate")
    length = int(sid["byteLength"], 16)
    encoded = bytes.fromhex(sid["bytes"])
    if (
        not 8 <= length <= 68
        or len(encoded) != 68
        or encoded[0] != 1
        or encoded[2:8] != bytes.fromhex("000000000005")
        or length != 8 + 4 * encoded[1]
        or any(encoded[length:])
    ):
        raise ValueError("invalid fixed-capacity SID")
    return encoded[:length]


def owner_sid_digest(value: OrderedDict[str, str] | dict[str, str] | None = None) -> str:
    raw = actual_sid_bytes(value)
    return digest(b"DisplayDeck.OwnerSidDigest.V1\0" + len(raw).to_bytes(4, "little") + raw).hex()


def owner_sid_cross_link_bytes() -> bytes:
    sid = sid_value()
    sid_digest = owner_sid_digest(sid)
    return canonical_json(OrderedDict((
        ("schema", "OwnerSidDigestV1-cross-link"),
        ("vectorId", "OWNERSIDV1-P-001"),
        ("sid", sid),
        ("ownerSidDigest", sid_digest),
        ("decisionJournalOwnerSidDigest", sid_digest),
        ("machineActorOwnerSidDigest", sid_digest),
        ("lockNameOwnerSidDigest", sid_digest),
    )))


def owner_sid_vector_bytes(vector_id: str) -> bytes:
    if vector_id == "OWNERSIDV1-P-001":
        return owner_sid_cross_link_bytes()
    if vector_id == "OWNERSIDV1-P-002":
        sid = sid_value(length=68)
    elif vector_id == "OWNERSIDV1-P-003":
        sid = sid_value(length=8)
    elif vector_id == "OWNERSIDV1-N-001":
        sid = OrderedDict((("kind", "0001"), ("byteLength", "00000000"), ("bytes", "00" * 68)))
    elif vector_id == "OWNERSIDV1-N-002":
        sid = OrderedDict((("kind", "0000"), ("byteLength", "00000008"), ("bytes", sid_value()["bytes"])))
    elif vector_id == "OWNERSIDV1-N-003":
        sid = OrderedDict((("kind", "0001"), ("byteLength", "00000008"), ("bytes", "00" * 68)))
    elif vector_id == "OWNERSIDV1-N-004":
        sid = sid_value(length=8, tail_byte=1)
    elif vector_id == "OWNERSIDV1-N-005":
        return canonical_json(OrderedDict((("schema", "OwnerSidDigestV1-negative"), ("sid", "S-1-5-0"), ("reason", "STRING_SID_FORBIDDEN"))))
    elif vector_id == "OWNERSIDV1-N-006":
        return canonical_json(OrderedDict((("schema", "OwnerSidDigestV1-negative"), ("sid", "account-name"), ("reason", "ACCOUNT_NAME_FORBIDDEN"))))
    elif vector_id == "OWNERSIDV1-N-007":
        sid = sid_value(length=8)
        capacity = bytes.fromhex(sid["bytes"])
        wrong = digest(b"DisplayDeck.OwnerSidDigest.V1\0" + (68).to_bytes(4, "little") + capacity).hex()
        return canonical_json(OrderedDict((("schema", "OwnerSidDigestV1-negative"), ("sid", sid), ("wrongPreimageDigest", wrong), ("reason", "FIXED_TAIL_IN_PREIMAGE"))))
    else:
        raise ValueError(vector_id)
    try:
        sid_digest = owner_sid_digest(sid)
    except ValueError:
        sid_digest = "INVALID"
    return canonical_json(OrderedDict((("schema", "OwnerSidDigestV1-boundary"), ("vectorId", vector_id), ("sid", sid), ("ownerSidDigest", sid_digest))))


def critical_subject_source(detail: int, context: str = "standalone") -> bytes:
    # Opaque but concrete bounded identity material.  The source never carries
    # raw SID/path/log data; it carries the deterministic trusted-target digest
    # that the candidate binds into the inner hash.
    return canonical_json(OrderedDict((("schema", "CriticalSubjectV1"), ("detail", f"{detail:04x}"), ("targetIdentityDigest", hex_digest(f"critical-target-{context}-{detail:04x}")))))


def critical_observed_source(evidence_kind: int, detail: int, context: str = "standalone") -> bytes:
    return canonical_json(OrderedDict((("schema", "CriticalObservationV1"), ("evidenceKind", f"{evidence_kind:04x}"), ("detail", f"{detail:04x}"), ("observationDigest", hex_digest(f"critical-observation-{context}-{evidence_kind:04x}-{detail:04x}")))))


def critical_evidence_inner_source(evidence_kind: int, error_class: int, detail: int, context: str = "standalone") -> bytes:
    """The 70-byte canonical inner source of the D02 outer hash."""
    subject = digest(b"DisplayDeck.CriticalSubject.V1\0" + critical_subject_source(detail, context))
    observed = digest(b"DisplayDeck.CriticalObservation.V1\0" + critical_observed_source(evidence_kind, detail, context))
    return evidence_kind.to_bytes(2, "little") + error_class.to_bytes(2, "little") + detail.to_bytes(2, "little") + subject + observed


def critical_evidence_preimage(evidence_kind: int, error_class: int, detail: int, context: str = "standalone") -> bytes:
    return b"DisplayDeck.CriticalEvidence.V1\0" + critical_evidence_inner_source(evidence_kind, error_class, detail, context)


def critical_evidence_digest(evidence_kind: int, error_class: int, detail: int, context: str = "standalone") -> str:
    return digest(critical_evidence_preimage(evidence_kind, error_class, detail, context)).hex()


def critical_evidence_inputs(evidence_kind: int, error_class: int, detail: int, context: str = "standalone") -> OrderedDict[str, str]:
    inner = critical_evidence_inner_source(evidence_kind, error_class, detail, context)
    preimage = critical_evidence_preimage(evidence_kind, error_class, detail, context)
    subject_source = critical_subject_source(detail, context)
    observed_source = critical_observed_source(evidence_kind, detail, context)
    subject = digest(b"DisplayDeck.CriticalSubject.V1\0" + subject_source).hex()
    observed = digest(b"DisplayDeck.CriticalObservation.V1\0" + observed_source).hex()
    return OrderedDict((
        ("domainSeparatorHex", b"DisplayDeck.CriticalEvidence.V1\0".hex()),
        ("subjectDomainSeparatorHex", b"DisplayDeck.CriticalSubject.V1\0".hex()),
        ("subjectSourceHex", subject_source.hex()),
        ("observedDomainSeparatorHex", b"DisplayDeck.CriticalObservation.V1\0".hex()),
        ("observedSourceHex", observed_source.hex()),
        ("innerCanonicalSourceHex", inner.hex()),
        ("outerPreimageHex", preimage.hex()),
        ("outerPreimageByteLength", f"{len(preimage):04x}"),
        ("evidenceKind", f"{evidence_kind:04x}"),
        ("lastErrorClass", f"{error_class:04x}"),
        ("lastErrorDetailCode", f"{detail:04x}"),
        ("subjectIdentityDigest", subject),
        ("observedEvidenceDigest", observed),
        ("preservedEvidenceDigest", critical_evidence_digest(evidence_kind, error_class, detail, context)),
    ))


DETAIL_MATRIX: dict[int, tuple[int, int]] = {
    1: (2, 1), 2: (1, 1), 3: (3, 1), 4: (4, 4), 5: (4, 4),
    6: (7, 8), 7: (1, 5), 8: (3, 2), 9: (9, 6), 10: (10, 7),
}

# Each MAR record carrying a D02 E-group is bound to a *context-specific*
# known-answer artifact.  The context is part of both inner sources, so equal
# detail codes in different records cannot accidentally share evidence bytes.
MAR_D02_BINDINGS: dict[str, tuple[str, int, int, int, str]] = {
    "MARV1-P-STATE-00": ("D02MARV1-P-STATE-00", 1, 2, 1, "mar-0-normal"),
    "MARV1-P-STATE-06": ("D02MARV1-P-STATE-06", 4, 4, 4, "mar-6-normal"),
    "MARV1-P-FC-01": ("D02MARV1-P-FC-01", 1, 2, 1, "mar-12-fc-01"),
    "MARV1-P-FC-02": ("D02MARV1-P-FC-02", 8, 7, 6, "mar-12-fc-02"),
    "MARV1-P-FC-03": ("D02MARV1-P-FC-03", 1, 3, 3, "mar-12-fc-03"),
}


def d02_vector_bytes(vector_id: str) -> bytes:
    if vector_id.startswith("D02MARV1-P-"):
        mar_id = next(mar for mar, binding in MAR_D02_BINDINGS.items() if binding[0] == vector_id)
        _, evidence_kind, error_class, detail, context = MAR_D02_BINDINGS[mar_id]
        return canonical_json(OrderedDict((
            ("schema", "D02-CriticalEvidenceV1-mar-context-known-answer"),
            ("vectorId", vector_id), ("boundMarVectorId", mar_id),
            ("detail", f"{detail:04x}"), ("requiredErrorClass", f"{error_class:04x}"),
            ("requiredEvidenceKind", f"{evidence_kind:04x}"), ("context", context),
            ("inputs", critical_evidence_inputs(evidence_kind, error_class, detail, context)),
            ("expectedClassification", "CRITICAL_EVIDENCE_EXACT_MATCH"),
        )))
    if vector_id.startswith("D02V1-N-"):
        inputs = critical_evidence_inputs(1, 2, 1)
        if vector_id == "D02V1-N-001":
            # Stored inner/outer values deliberately remain from the base while
            # one bounded subject-source byte changes.
            mutated = bytearray.fromhex(inputs["subjectSourceHex"]); mutated[-2] ^= 1
            inputs["subjectSourceHex"] = bytes(mutated).hex()
            classification = "REJECT_SUBJECT_SOURCE_REHASH_MISMATCH"
        elif vector_id == "D02V1-N-002":
            inputs["preservedEvidenceDigest"] = ("0" if inputs["preservedEvidenceDigest"][0] != "0" else "1") + inputs["preservedEvidenceDigest"][1:]
            classification = "REJECT_OUTER_DIGEST_MISMATCH"
        else:
            inputs["subjectIdentityDigest"] = ZERO32
            # A recomputed outer digest with zero inner identity is still
            # forbidden because both D02 inner digests are required nonzero.
            outer = b"DisplayDeck.CriticalEvidence.V1\0" + (1).to_bytes(2, "little") + (2).to_bytes(2, "little") + (1).to_bytes(2, "little") + bytes(32) + bytes.fromhex(inputs["observedEvidenceDigest"])
            inputs["outerPreimageHex"], inputs["preservedEvidenceDigest"] = outer.hex(), digest(outer).hex()
            classification = "REJECT_ZERO_SUBJECT_INNER_DIGEST"
        return canonical_json(OrderedDict((("schema", "D02-CriticalEvidenceV1-negative"), ("vectorId", vector_id), ("inputs", inputs), ("expectedClassification", classification), ("actionCounts", OrderedDict((("fileWrite", 0), ("displayMutation", 0)))))))
    detail = int(vector_id[-2:], 16)
    error_class, evidence_kind = DETAIL_MATRIX[detail]
    return canonical_json(OrderedDict((
        ("schema", "D02-CriticalEvidenceV1-known-answer"), ("vectorId", vector_id),
        ("detail", f"{detail:04x}"), ("requiredErrorClass", f"{error_class:04x}"),
        ("requiredEvidenceKind", f"{evidence_kind:04x}"), ("inputs", critical_evidence_inputs(evidence_kind, error_class, detail)),
        ("expectedClassification", "CRITICAL_EVIDENCE_EXACT_MATCH"),
    )))


OWNER_WAL_CODES = tuple(range(0x0000, 0x0015)) + tuple(range(0x0020, 0x0029))


def owner_wal_vector_bytes(vector_id: str) -> bytes:
    if vector_id.startswith("OWNWALV1-P-"):
        code = int(vector_id.removeprefix("OWNWALV1-P-"), 16)
        return canonical_json(OrderedDict((
            ("schema", "OwnerWalLinkStateV1-roundtrip"), ("vectorId", vector_id),
            ("ownerWalState", f"{code:04x}"), ("wireBytesLe", code.to_bytes(2, "little").hex()),
            ("classification", "ABSENT_EXPECTED_MACHINE_LINK_ONLY" if code == 0 else ("TERMINAL_OWNER_WAL" if code >= 0x20 else "NONTERMINAL_OWNER_WAL")),
            ("ownerWalFrameEncodable", code != 0),
        )))
    suffix = vector_id.removeprefix("OWNWALV1-N-")
    code = {"RESERVED-GAP": 0x0015, "RESERVED-TERMINAL": 0x0029, "UNKNOWN": 0xFFFF}[suffix]
    return canonical_json(OrderedDict((
        ("schema", "OwnerWalLinkStateV1-reject"), ("vectorId", vector_id), ("ownerWalState", f"{code:04x}"),
        ("wireBytesLe", code.to_bytes(2, "little").hex()), ("classification", "REJECT_RESERVED_OR_UNKNOWN"),
        ("ownerWalFrameEncodable", False),
    )))


MAPSCN_POSITIVE = {
    # stored, current authoritative, candidate-next (0 means no write), MAR observation
    "MAPSCNV1-P-CREATE-ABSENT": (1, 1, 0, None, "MACHINE_ACTOR_FILE_ABSENT", "CREATE_INTENT_FILE_ABSENT", "MACHINE_ACTOR", "CREATE_NEW_THEN_POST_CREATE_CHECKPOINT"),
    "MAPSCNV1-P-RESUME-01": (1, 2, 0, None, "EMPTY_ZERO_LENGTH", "POST_CREATE_CHECKPOINT_SAME_ID_EMPTY", "MACHINE_ACTOR", "INITIALIZE_FULL_ZERO_HEADER"),
    "MAPSCNV1-P-RESUME-02": (1, 2, 0, None, "FRESH_UNINITIALIZED", "FRESH_UNINITIALIZED_SAME_ID", "MACHINE_ACTOR", "PUBLISH_MAINTENANCE_INTENT"),
    "MAPSCNV1-P-STATE-03": (1, 2, 3, "MARV1-P-STATE-0A", "MAINTENANCE_INTENT", "EXACT_MAINTENANCE_INTENT", "PROVISION_RECORD", "PUBLISH_MACHINE_INTENT"),
    "MAPSCNV1-P-MAP03-INTENT": (2, 3, 0, "MARV1-P-STATE-0A", "MAINTENANCE_INTENT", "MAP3_EXACT_MAINTENANCE_INTENT", "MACHINE_ACTOR", "PUBLISH_MAINTENANCE_ACTIVE"),
    "MAPSCNV1-P-STATE-04": (2, 3, 4, "MARV1-P-STATE-0B", "MAINTENANCE_ACTIVE", "EXACT_MAINTENANCE_ACTIVE", "PROVISION_RECORD", "PUBLISH_MACHINE_ACTIVE"),
    "MAPSCNV1-P-MAP04-ACTIVE": (3, 4, 0, "MARV1-P-STATE-0B", "MAINTENANCE_ACTIVE", "MAP4_EXACT_MAINTENANCE_ACTIVE", "MACHINE_ACTOR", "PUBLISH_PROVISIONED_CLEAN"),
    "MAPSCNV1-P-STATE-05": (3, 4, 5, "MARV1-P-PROVISIONED-CLEAN", "PROVISIONED_TERMINAL_CLEAN", "EXACT_PROVISIONED_CLEAN", "PROVISION_RECORD", "PUBLISH_MACHINE_CLEAN"),
    "MAPSCNV1-P-STATE-06": (4, 5, 6, "MARV1-P-PROVISIONED-CLEAN", "PROVISIONED_TERMINAL_CLEAN", "EXACT_TERMINAL_RETAINED", "PROVISION_RECORD", "PUBLISH_TERMINAL_RETAINED"),
    "MAPSCNV1-P-TERMINAL-READONLY": (5, 6, 0, "MARV1-P-PROVISIONED-CLEAN", "PROVISIONED_TERMINAL_CLEAN", "TERMINAL_RETAINED_READ_ONLY", "NONE", "NONE"),
}


def fresh_machine_actor_bytes() -> bytes:
    return record_file(envelope_header(bytes.fromhex("44444d41525631000000000000000000"), MAR_SLOT_SIZE, MAR_FILE_LENGTH), MAR_SLOT_SIZE, {})


def map_scenario_bytes(vector_id: str) -> bytes:
    """Bounded, read-only 19.8 provisioning/resume evidence scenario."""
    positive = MAPSCN_POSITIVE.get(vector_id)
    if positive is None:
        # Each negative starts from a complete checkpoint/fresh observation and
        # changes exactly one bounded fact or requested forbidden action.
        positive = MAPSCN_POSITIVE["MAPSCNV1-P-RESUME-02"]
    stored_state, current_state, candidate_state, mar_id, mar_class, classification, write_target, next_write = positive
    stored_map = map_bytes(f"MAPRV1-P-STATE-{stored_state:02d}")
    current_map = map_bytes(f"MAPRV1-P-STATE-{current_state:02d}")
    checkpoint_map = map_bytes("MAPRV1-P-STATE-02")
    facts = OrderedDict((("volumeSerial", "1122334455667788"), ("fileId", hex_id("machine-file")), ("daclDigest", hex_digest("machineActorDaclDigest")), ("anchorDigest", hex_digest("directoryAnchorDigest")), ("attributeDigest", hex_digest("machineActorAttributeStreamDigest"))))
    if mar_id is None:
        observed_bytes = b"" if vector_id in {"MAPSCNV1-P-RESUME-01", "MAPSCNV1-P-CREATE-ABSENT"} else fresh_machine_actor_bytes()
        header_digest = ZERO32 if not observed_bytes else digest(observed_bytes[:MAR_HEADER_SIZE]).hex()
        slot_digest, full_digest = (ZERO32, ZERO32) if not observed_bytes else (digest(observed_bytes[MAR_HEADER_SIZE:MAR_HEADER_SIZE + MAR_SLOT_SIZE]).hex(), digest(observed_bytes).hex())
        record_state, epoch, lease, actor, nonce = "0000", ZERO8, ZERO8, actor_ref("system-maintenance", 4), hex_id("provision-nonce")
    else:
        observed_bytes = mar_bytes(mar_id)
        header_digest, slot_digest, full_digest = digest(observed_bytes[:MAR_HEADER_SIZE]).hex(), digest(observed_bytes[MAR_HEADER_SIZE:MAR_HEADER_SIZE + MAR_SLOT_SIZE]).hex(), digest(observed_bytes).hex()
        record_state = f"{int(mar_id[-2:], 16) if 'STATE' in mar_id else 9:04x}"
        epoch, lease, actor, nonce = "0000000000000001", "0000000000000001", actor_ref("system-maintenance", 4), hex_id("provision-nonce")
    base = OrderedDict((
        ("schema", "MachineActorProvisionReadOnlyScenarioV2"), ("vectorId", vector_id),
        ("storedMapState", f"{stored_state:04x}"), ("currentMapState", f"{current_state:04x}"), ("candidateNextMapState", f"{candidate_state:04x}"),
        ("storedMapRecordSha256", digest(stored_map).hex()), ("currentMapRecordSha256", digest(current_map).hex()),
        ("candidateNextMapRecordSha256", ZERO32 if candidate_state == 0 else digest(map_bytes(f"MAPRV1-P-STATE-{candidate_state:02d}")).hex()),
        ("checkpointMapRecordSha256", digest(checkpoint_map).hex()), ("checkpointLinkState", "0002"),
        ("stored", OrderedDict(facts)), ("observed", OrderedDict(facts)),
        ("observedPresence", True), ("observedIdentityStatus", "PRESENT_EXACT"), ("observedByteLength", f"{len(observed_bytes):016x}"),
        ("observedMachineActorClassification", mar_class), ("observedMachineActorFullSha256", full_digest),
        ("observedHeaderDigest", header_digest), ("observedSlotDigest", slot_digest),
        ("observedRecordState", record_state), ("observedEpoch", epoch), ("observedLease", lease),
        ("provisionActor", actor), ("provisionNonce", nonce), ("installerManifestDigest", hex_digest("installerManifestDigest")),
        ("designatedOwnerSidDigest", owner_sid_digest()), ("requestedAction", "READ_ONLY_CLASSIFY"),
        ("classification", classification), ("soleNextWriteTarget", write_target), ("soleNextWrite", next_write), ("d07DirectoryAnchorEvidence", "UNPROVEN"), ("runtimeWriteAuthorized", False),
        ("packageCompletionDigest", ZERO32), ("packageCompletionEvidenceStatus", "NOT_REQUIRED"),
        ("actionCounts", OrderedDict((("resume", 0), ("delete", 0), ("recreate", 0), ("cleanup", 0), ("fileWrite", 0), ("displayMutation", 0)))),
    ))
    if current_state == 6 or candidate_state == 6:
        map6 = map_bytes("MAPRV1-P-STATE-06")
        map6_slot = map6[MAP_HEADER_SIZE + MAP_SLOT_SIZE:MAP_HEADER_SIZE + 2 * MAP_SLOT_SIZE]
        map6_payload_length = int.from_bytes(map6_slot[28:32], "little")
        current_payload = json.loads(map6_slot[224:224 + map6_payload_length].decode("ascii"), object_pairs_hook=OrderedDict)
        base["packageCompletionDigest"] = current_payload["packageCompletionDigest"]
        base["packageCompletionEvidenceStatus"] = "DURABLE_READBACK_EXACT"
    if vector_id == "MAPSCNV1-P-CREATE-ABSENT":
        base["observed"] = OrderedDict((
            ("volumeSerial", "0000000000000000"), ("fileId", "00000000000000000000000000000000"),
            ("daclDigest", ZERO32), ("anchorDigest", ZERO32), ("attributeDigest", ZERO32),
        ))
        base["observedPresence"] = False
        base["observedIdentityStatus"] = "ABSENT_EXPECTED"
    negatives = {
        "MAPSCNV1-N-010": ("observed.fileId", hex_id("different-machine-file"), "RESUME", "REJECT_DIFFERENT_FILE_ID"),
        "MAPSCNV1-N-011": ("observed.daclDigest", hex_digest("wrong-dacl"), "DELETE_RECREATE", "REJECT_DACL_MISMATCH"),
        "MAPSCNV1-N-012": ("observed.anchorDigest", hex_digest("wrong-anchor"), "RESUME", "REJECT_ANCHOR_MISMATCH"),
        "MAPSCNV1-N-013": ("observed.attributeDigest", hex_digest("wrong-attribute"), "RESUME", "REJECT_ATTRIBUTE_MISMATCH"),
        "MAPSCNV1-N-017": ("provisionActor.instanceId", hex_id("wrong-provision-actor"), "RESUME", "REJECT_PROVISION_ACTOR_MISMATCH"),
        "MAPSCNV1-N-018": ("provisionNonce", hex_id("wrong-provision-nonce"), "RESUME", "REJECT_PROVISION_NONCE_MISMATCH"),
        "MAPSCNV1-N-019": ("installerManifestDigest", hex_digest("wrong-manifest"), "RESUME", "REJECT_MANIFEST_MISMATCH"),
        "MAPSCNV1-N-020": ("observedEpoch", "0000000000000002", "RESUME", "REJECT_EPOCH_MISMATCH"),
        "MAPSCNV1-N-021": ("observedLease", "0000000000000002", "RESUME", "REJECT_LEASE_MISMATCH"),
    }
    if vector_id in negatives:
        path, value, action, reject = negatives[vector_id]
        container, key = path.split(".") if "." in path else (None, path)
        if container == "observed": base["observed"][key] = value
        elif container == "provisionActor": base["provisionActor"][key] = value
        else: base[key] = value
        base["requestedAction"], base["classification"], base["soleNextWrite"] = action, reject, "NONE"
    elif vector_id == "MAPSCNV1-N-014":
        base["observedByteLength"], base["observedHeaderDigest"] = "0000000000001000", hex_digest("partial-header-bytes")
        base["requestedAction"], base["classification"], base["soleNextWrite"] = "RESUME", "REJECT_PARTIAL_HEADER", "NONE"
    elif vector_id == "MAPSCNV1-N-015":
        base["observedByteLength"], base["observedSlotDigest"] = "0000000000011000", hex_digest("partial-slot-bytes")
        base["requestedAction"], base["classification"], base["soleNextWrite"] = "DELETE_RECREATE", "REJECT_PARTIAL_SLOT", "NONE"
    elif vector_id == "MAPSCNV1-N-016":
        base["observedSlotDigest"] = hex_digest("checksum-mismatch-slot")
        base["requestedAction"], base["classification"], base["soleNextWrite"] = "CLEANUP", "REJECT_CHECKSUM_MISMATCH", "NONE"
    elif vector_id == "MAPSCNV1-N-022":
        failed = mar_bytes("MARV1-P-FC-01")
        base["observedMachineActorClassification"], base["observedMachineActorFullSha256"] = "FAILED_CLOSED", digest(failed).hex()
        base["observedHeaderDigest"], base["observedSlotDigest"], base["observedRecordState"] = digest(failed[:MAR_HEADER_SIZE]).hex(), digest(failed[MAR_HEADER_SIZE:MAR_HEADER_SIZE + MAR_SLOT_SIZE]).hex(), "000c"
        base["observedEpoch"], base["observedLease"] = "0000000000000001", "0000000000000001"
        base["requestedAction"], base["classification"], base["soleNextWrite"] = "CLEANUP", "REJECT_FAILED_CLOSED_CLEANUP", "NONE"
    elif vector_id == "MAPSCNV1-N-023":
        base["storedMapState"], base["currentMapState"], base["candidateNextMapState"] = "0001", "0002", "0004"
        base["storedMapRecordSha256"], base["currentMapRecordSha256"], base["candidateNextMapRecordSha256"] = digest(map_bytes("MAPRV1-P-STATE-01")).hex(), digest(map_bytes("MAPRV1-P-STATE-02")).hex(), digest(map_bytes("MAPRV1-P-STATE-04")).hex()
        base["requestedAction"], base["classification"], base["soleNextWrite"] = "RESUME", "REJECT_TWO_STEP_AHEAD", "NONE"
    elif vector_id in {"MAPSCNV1-N-024", "MAPSCNV1-N-025", "MAPSCNV1-N-026"}:
        base["storedMapState"], base["currentMapState"], base["candidateNextMapState"] = "0001", "0001", "0000"
        base["storedMapRecordSha256"] = base["currentMapRecordSha256"] = digest(map_bytes("MAPRV1-P-STATE-01")).hex()
        base["candidateNextMapRecordSha256"] = ZERO32
        action = {"MAPSCNV1-N-024": "RESUME", "MAPSCNV1-N-025": "DELETE_RECREATE", "MAPSCNV1-N-026": "RECREATE"}[vector_id]
        base["requestedAction"], base["classification"], base["soleNextWrite"] = action, "REJECT_PRECHECKPOINT_EXISTING_FILE", "NONE"
    if vector_id.startswith("MAPSCNV1-N-"):
        base["soleNextWriteTarget"] = "NONE"
    return canonical_json(base)


def d04_tuple_id(kind: int, result: int, owner_state: int, record_state: int, variant: str) -> str:
    return f"D04TUPV1-P-K{kind:02X}-R{result:02X}-W{owner_state:04X}-S{record_state:02X}-V{variant}"


def d04_allowed_tuples() -> dict[str, tuple[int, int, int, int, str]]:
    """Approved R01--R04 witness rows, materialized without inferred tuples."""
    rows: dict[str, tuple[int, int, int, int, str]] = {}

    def add(kind: int, result: int, owner: int, state: int, variant: str) -> None:
        rows[d04_tuple_id(kind, result, owner, state, variant)] = (kind, result, owner, state, variant)

    # Display and recovery are exact owner-result rows; R06/R07 enumerate the
    # approved uncertainty and critical owner classes rather than a default.
    for kind, normal_rows, critical_rows in (
        (1, ((1, 0x20), (2, 0x23), (3, 0x21), (4, 0x22)), ((5, 0x27),)),
        (2, ((3, 0x21), (4, 0x22)), ((5, 0x27),)),
    ):
        for result, owner in normal_rows:
            add(kind, result, owner, 8, "TERMINALIZING")
            add(kind, result, owner, 9, "TERMINAL_CLEAN")
        for result, owner in critical_rows:
            add(kind, result, owner, 12, "CRITICAL")
    for kind in (1, 2):
        for owner in range(0x0002, 0x0015):
            add(kind, 6, owner, 12, "UNCERTAINTY")
        for owner in (0x24, 0x25, 0x26, 0x28):
            add(kind, 7, owner, 12, "CRITICAL")

    # R01: maintenance/update/repair retain a byte-exact normal terminal for
    # normal results and retain a normal or lossless critical terminal for
    # critical results.  Every allowed owner class is an independent witness.
    for kind in (3, 4, 5):
        for result in (1, 2, 3):
            for owner in range(0x20, 0x24):
                add(kind, result, owner, 9, "NORMAL_RETAINED")
        for result in (4, 5, 6, 7):
            for owner in range(0x20, 0x24):
                add(kind, result, owner, 12, "NORMAL_RETAINED_FAILED_CLOSED")
            for owner in range(0x24, 0x29):
                add(kind, result, owner, 12, "LOSSLESS_CRITICAL")

    # R04 splits structural completion at MAP state 5 from readiness at 6.
    add(7, 1, 0x0000, 9, "PROVISION_STRUCTURAL")
    return rows


D04_ALLOWED_TUPLES = d04_allowed_tuples()
D04_READINESS_ID = "D04READYV1-P-K07-R01-W0000-S06-VPROVISION_READINESS"


def d04_evidence_id(tuple_id: str) -> str:
    return "D04D02V1-P-" + tuple_id.removeprefix("D04TUPV1-P-")


def d04_evidence_parameters(result: int) -> tuple[int, int, int]:
    # These are the pre-approved D02 lanes.  The witness records the exact
    # lane; it does not infer a runtime cause beyond the D04 result boundary.
    return {1: (7, 10, 10), 4: (2, 3, 8), 5: (4, 4, 5), 6: (8, 7, 6), 7: (1, 2, 1)}[result]


D04_NEGATIVE_EVIDENCE_BOUNDARIES: dict[str, tuple[int, int, int]] = {
    "D04TUPV1-N-UNCERTAINTY-ABSENT": (1, 6, 0x0000),
    "D04TUPV1-N-UNCERTAINTY-EMPTY": (1, 6, 0x0001),
    "D04TUPV1-N-UNCERTAINTY-TERMINAL": (1, 6, 0x0020),
    "D04TUPV1-N-R07-NORMAL-OWNER": (1, 7, 0x0020),
    "D04TUPV1-N-R07-W0027-NOT-ALLOWED": (1, 7, 0x0027),
    "D04TUPV1-N-MAINTENANCE-CLEAN-STATE": (3, 1, 0x0020),
}


def d04_negative_evidence_id(vector_id: str) -> str:
    return "D04D02V1-P-BOUNDARY-" + vector_id.removeprefix("D04TUPV1-N-")


D04_EVIDENCE_BINDINGS: dict[str, tuple[str, int, int, int]] = {
    d04_evidence_id(tuple_id): (tuple_id, kind, result, owner)
    for tuple_id, (kind, result, owner, state, _variant) in D04_ALLOWED_TUPLES.items()
    if state == 12
}
D04_EVIDENCE_BINDINGS.update({
    d04_negative_evidence_id(vector_id): (vector_id, kind, result, owner)
    for vector_id, (kind, result, owner) in D04_NEGATIVE_EVIDENCE_BOUNDARIES.items()
})
D04_D02_EVIDENCE_IDS = tuple(sorted(D04_EVIDENCE_BINDINGS, key=lambda value: value.encode("utf-8")))


def d04_actor(kind: int) -> OrderedDict[str, Any]:
    return actor_ref("system-maintenance", 4) if kind == 7 else actor_ref("maintenance", 4) if kind in (3, 4, 5) else actor_ref("watchdog", 2)


def d04_owner_provenance(owner_state: int, *, retained: str) -> OrderedDict[str, Any]:
    owner_id = f"OWNWALV1-P-{owner_state:04X}"
    return OrderedDict((
        ("ownerWalState", f"{owner_state:04x}"), ("ownerWalGeneration", "0000000000000001" if owner_state else ZERO8),
        ("ownerTerminalDigest", hex_digest(f"d04-owner-terminal-{owner_state:04x}") if owner_state >= 0x20 else ZERO32),
        ("sourceOwnerWalVectorId", owner_id), ("sourceOwnerWalFixtureSha256", digest(owner_wal_vector_bytes(owner_id)).hex()),
        ("retentionMode", retained),
    ))


def d04_evidence_object(bound_id: str, kind: int, result: int, owner: int, evidence_id: str) -> OrderedDict[str, Any]:
    evidence_kind, error_class, detail = d04_evidence_parameters(result)
    inputs = critical_evidence_inputs(evidence_kind, error_class, detail, f"d04-{bound_id}")
    return OrderedDict((
        ("evidenceKind", f"{evidence_kind:04x}"), ("errorClass", f"{error_class:04x}"), ("detail", f"{detail:04x}"),
        ("d02VectorId", evidence_id), ("d02FixtureSha256", digest(d04_d02_bytes(evidence_id)).hex()),
        ("inputs", inputs), ("preservedEvidenceDigest", inputs["preservedEvidenceDigest"]),
    ))


def d04_critical_evidence(tuple_id: str) -> OrderedDict[str, Any]:
    kind, result, owner, _state, _variant = D04_ALLOWED_TUPLES[tuple_id]
    return d04_evidence_object(tuple_id, kind, result, owner, d04_evidence_id(tuple_id))


def d04_d02_bytes(vector_id: str) -> bytes:
    bound_id, kind, result, owner = D04_EVIDENCE_BINDINGS[vector_id]
    evidence_kind, error_class, detail = d04_evidence_parameters(result)
    inputs = critical_evidence_inputs(evidence_kind, error_class, detail, f"d04-{bound_id}")
    return canonical_json(OrderedDict((
        ("schema", "D04CriticalEvidenceKnownAnswerV1"), ("vectorId", vector_id),
        ("boundD04WitnessVectorId", bound_id), ("operationKind", f"{kind:04x}"), ("operationResult", f"{result:04x}"), ("ownerWalState", f"{owner:04x}"),
        ("evidenceKind", f"{evidence_kind:04x}"), ("errorClass", f"{error_class:04x}"), ("detail", f"{detail:04x}"),
        ("inputs", inputs), ("preservedEvidenceDigest", inputs["preservedEvidenceDigest"]),
        ("classification", "D04_CRITICAL_EVIDENCE_EXACT"),
        ("actionCounts", OrderedDict((("displayMutation", 0), ("fileWrite", 0), ("processLaunch", 0)))),
    )))


def d04_machine_actor_fixture(state: int, variant: str) -> str:
    if variant.startswith("PROVISION_"):
        return "MARV1-P-PROVISIONED-CLEAN"
    if state == 12:
        return "MARV1-P-FC-01"
    return f"MARV1-P-STATE-{state:02X}"


def d04_normal_pair(kind: int, result: int, owner: int) -> OrderedDict[str, Any]:
    predecessor_id = d04_tuple_id(kind, result, owner, 8, "TERMINALIZING")
    predecessor_digest = digest(d04_tuple_bytes(predecessor_id)).hex()
    return OrderedDict((
        ("terminalizingVectorId", predecessor_id), ("terminalizingFixtureSha256", predecessor_digest),
        ("terminalCleanPredecessorSha256", predecessor_digest), ("terminalCleanState", "0009"),
    ))


def d04_tuple_projection(value: OrderedDict[str, Any]) -> OrderedDict[str, Any]:
    """Canonical semantic tuple source; the linked MAR is layout-only."""
    projection = OrderedDict((
        ("recordState", value["recordState"]),
        ("operationKind", value["operationKind"]),
        ("operationNonce", value["operationNonce"]),
        ("operationIntent", value["operationIntent"]),
        ("operationCompletion", value["operationCompletion"]),
        ("ownerWal", value["ownerWal"]),
    ))
    for key in ("criticalEvidence", "terminalPair", "initialProvision"):
        if key in value:
            projection[key] = value[key]
    return projection


def d04_tuple_bytes(vector_id: str) -> bytes:
    kind, result, owner, state, variant = D04_ALLOWED_TUPLES[vector_id]
    actor, nonce = d04_actor(kind), hex_id(f"d04-k{kind:04x}-r{result:04x}-w{owner:04x}-nonce")
    p, q = intent(kind, actor, nonce), completion(kind, actor, nonce)
    q["result"] = f"{result:04x}"
    retained = "BYTE_EXACT_NORMAL" if owner in range(0x20, 0x24) else "LOSSLESS_CRITICAL" if owner >= 0x24 else "LAST_VALID_NONTERMINAL"
    value = OrderedDict((
        ("schema", "D04TupleWitnessV1"), ("vectorId", vector_id), ("operationKind", f"{kind:04x}"),
        ("operationNonce", nonce), ("operationIntent", p), ("operationCompletion", q),
        ("ownerWal", d04_owner_provenance(owner, retained=retained)), ("recordState", f"{state:04x}"),
        ("variant", variant), ("classification", "D04_EXACT_ALLOWED_TUPLE"),
        ("machineActorLayoutFixtureId", d04_machine_actor_fixture(state, variant)),
        ("machineActorLayoutFixtureSha256", digest(mar_bytes(d04_machine_actor_fixture(state, variant))).hex()),
        ("machineActorLayoutOnly", True),
    ))
    if variant == "TERMINAL_CLEAN":
        value["terminalPair"] = d04_normal_pair(kind, result, owner)
    if state == 12:
        value["criticalEvidence"] = d04_critical_evidence(vector_id)
    if variant.startswith("PROVISION_"):
        checkpoint = hex_digest("provision-checkpoint")
        value["initialProvision"] = OrderedDict((
            ("bootId", hex_digest("boot")), ("epoch", "0000000000000001"), ("lease", "0000000000000001"),
            ("designatedOwnerSidDigest", owner_sid_digest()), ("systemMaintenanceActor", actor_ref("system-maintenance", 4)),
            ("ownerWalTypedAbsence", "ABSENT_EXPECTED"), ("terminalTypedAbsence", "ABSENT_EXPECTED"),
            ("provisionCheckpointDigest", checkpoint),
            ("provisionRecordState", "0005"), ("provisionRecordVectorId", "MAPRV1-P-STATE-05"),
            ("provisionRecordFixtureSha256", digest(map_bytes("MAPRV1-P-STATE-05")).hex()),
            ("machineActorTupleSource", "THIS_D04_TUPLE_PROJECTION"),
            ("readiness", "STRUCTURAL_ONLY"), ("packageCompletionDigest", ZERO32),
        ))
    projection = d04_tuple_projection(value)
    value["tupleProjection"] = projection
    value["tupleProjectionSha256"] = digest(canonical_json(projection)).hex()
    value["actionCounts"] = OrderedDict((("displayMutation", 0), ("fileWrite", 0), ("processLaunch", 0)))
    return canonical_json(value)


def d04_readiness_bytes() -> bytes:
    structural = d04_tuple_id(7, 1, 0, 9, "PROVISION_STRUCTURAL")
    state6 = map_bytes("MAPRV1-P-STATE-06")
    slot = state6[MAP_HEADER_SIZE + MAP_SLOT_SIZE:MAP_HEADER_SIZE + 2 * MAP_SLOT_SIZE]
    payload_length = int.from_bytes(slot[28:32], "little")
    payload = json.loads(slot[224:224 + payload_length].decode("ascii"), object_pairs_hook=OrderedDict)
    return canonical_json(OrderedDict((
        ("schema", "D04ProvisionReadinessCompanionV1"), ("vectorId", D04_READINESS_ID),
        ("structuralTupleVectorId", structural), ("structuralTupleFixtureSha256", digest(d04_tuple_bytes(structural)).hex()),
        ("provisionRecordVectorId", "MAPRV1-P-STATE-06"), ("provisionRecordFixtureSha256", digest(state6).hex()),
        ("provisionRecordState", "0006"), ("packageCompletionDigest", payload["packageCompletionDigest"]),
        ("readiness", "DURABLE_PACKAGE_COMPLETION_EXACT"), ("runtimeWriteAuthorized", False),
        ("classification", "D04_R04_READINESS_EVIDENCE_ONLY"),
        ("actionCounts", OrderedDict((("displayMutation", 0), ("fileWrite", 0), ("processLaunch", 0)))),
    )))


D04_NEGATIVE_IDS = (
    "D04TUPV1-N-NONE-RESERVED", "D04TUPV1-N-UNINSTALL-NOT-ADMITTED", "D04TUPV1-N-PQ-NONCE-MISMATCH",
    "D04TUPV1-N-ACTOR-MISMATCH", "D04TUPV1-N-VARIANT-MISMATCH", "D04TUPV1-N-OWNER-PROVENANCE-MISSING",
    "D04TUPV1-N-UNCERTAINTY-EVIDENCE-MISSING", "D04TUPV1-N-PROVISION-STATE5-MISSING",
    "D04TUPV1-N-PROVISION-STATE6-PREMATURE", "D04TUPV1-N-NORMAL-PREDECESSOR-MISMATCH",
    "D04TUPV1-N-OWNER-WAL-FIXTURE-MISMATCH", "D04TUPV1-N-UNCERTAINTY-EMPTY",
    "D04TUPV1-N-PROVISION-BOOT-ZERO", "D04TUPV1-N-RECOVERY-R01-NOT-ADMITTED",
    "D04TUPV1-N-RECOVERY-R02-NOT-ADMITTED", "D04TUPV1-N-UNCERTAINTY-ABSENT",
    "D04TUPV1-N-UNCERTAINTY-TERMINAL", "D04TUPV1-N-R07-NORMAL-OWNER",
    "D04TUPV1-N-R07-W0027-NOT-ALLOWED", "D04TUPV1-N-MAINTENANCE-ACTOR-MISMATCH",
    "D04TUPV1-N-MAINTENANCE-CLEAN-STATE", "D04TUPV1-N-MAINTENANCE-OWNER-REWRITE",
    "D04TUPV1-N-PROVISION-SID-MISMATCH", "D04TUPV1-N-PROVISION-CHECKPOINT-ZERO",
    "D04TUPV1-N-PROVISION-ACTOR-MISMATCH", "D04TUPV1-N-PROVISION-TYPED-ABSENCE",
    "D04TUPV1-N-PROVISION-FENCE-ZERO", "D04TUPV1-N-NORMAL-PAIR-TUPLE-CHANGE",
)


def d04_negative_bytes(vector_id: str) -> bytes:
    # Each negative is a bounded, read-only malformed witness.  It starts from
    # a named allowed row only where the mutation must prove a binding rule.
    base_id = d04_tuple_id(1, 3, 0x21, 9, "TERMINAL_CLEAN")
    if vector_id in {
        "D04TUPV1-N-UNCERTAINTY-EVIDENCE-MISSING",
        "D04TUPV1-N-UNCERTAINTY-EMPTY",
        "D04TUPV1-N-UNCERTAINTY-ABSENT",
        "D04TUPV1-N-UNCERTAINTY-TERMINAL",
    }:
        base_id = d04_tuple_id(1, 6, 0x0002, 12, "UNCERTAINTY")
    elif vector_id in {"D04TUPV1-N-R07-NORMAL-OWNER", "D04TUPV1-N-R07-W0027-NOT-ALLOWED"}:
        base_id = d04_tuple_id(1, 7, 0x0024, 12, "CRITICAL")
    elif vector_id.startswith("D04TUPV1-N-PROVISION"):
        base_id = d04_tuple_id(7, 1, 0, 9, "PROVISION_STRUCTURAL")
    elif vector_id.startswith("D04TUPV1-N-MAINTENANCE"):
        base_id = d04_tuple_id(3, 1, 0x20, 9, "NORMAL_RETAINED")
    value = json.loads(d04_tuple_bytes(base_id).decode("ascii"), object_pairs_hook=OrderedDict)
    value["schema"], value["vectorId"], value["classification"] = "D04TupleWitnessV1-reject", vector_id, "D04_REJECT_FAIL_CLOSED"
    if vector_id == "D04TUPV1-N-NONE-RESERVED":
        value["operationCompletion"]["result"] = "0000"
    elif vector_id == "D04TUPV1-N-UNINSTALL-NOT-ADMITTED":
        actor, nonce = d04_actor(6), hex_id("d04-uninstall-not-admitted")
        value["operationKind"], value["operationNonce"] = "0006", nonce
        value["operationIntent"], value["operationCompletion"] = intent(6, actor, nonce), completion(6, actor, nonce)
        value["operationCompletion"]["result"] = "0001"
        value["recordState"], value["variant"] = "0009", "UNINSTALL_RESERVED"
    elif vector_id == "D04TUPV1-N-PQ-NONCE-MISMATCH":
        value["operationCompletion"]["operationNonce"] = hex_id("d04-wrong-q-nonce")
    elif vector_id == "D04TUPV1-N-ACTOR-MISMATCH":
        value["operationCompletion"]["actor"] = d04_actor(3)
    elif vector_id == "D04TUPV1-N-VARIANT-MISMATCH":
        value["operationCompletion"].pop("completionEvidenceDigest")
        value["operationCompletion"]["tombstoneDigest"] = hex_digest("wrong-tagged-variant")
    elif vector_id == "D04TUPV1-N-OWNER-PROVENANCE-MISSING":
        value["ownerWal"]["ownerTerminalDigest"] = ZERO32
    elif vector_id == "D04TUPV1-N-UNCERTAINTY-EVIDENCE-MISSING":
        value.pop("criticalEvidence")
    elif vector_id == "D04TUPV1-N-PROVISION-STATE5-MISSING":
        value["initialProvision"]["provisionRecordState"] = "0004"
    elif vector_id == "D04TUPV1-N-PROVISION-STATE6-PREMATURE":
        value["initialProvision"]["provisionRecordState"] = "0006"
        value["initialProvision"]["readiness"] = "DURABLE_PACKAGE_COMPLETION_EXACT"
        value["initialProvision"]["packageCompletionDigest"] = hex_digest("package-completion")
    elif vector_id == "D04TUPV1-N-NORMAL-PREDECESSOR-MISMATCH":
        value["terminalPair"]["terminalCleanPredecessorSha256"] = hex_digest("wrong-d04-predecessor")
    elif vector_id == "D04TUPV1-N-OWNER-WAL-FIXTURE-MISMATCH":
        value["ownerWal"]["sourceOwnerWalFixtureSha256"] = hex_digest("wrong-owner-wal-fixture")
    elif vector_id == "D04TUPV1-N-UNCERTAINTY-EMPTY":
        value["ownerWal"]["ownerWalState"] = "0001"
    elif vector_id == "D04TUPV1-N-PROVISION-BOOT-ZERO":
        value["initialProvision"]["bootId"] = ZERO32
    elif vector_id == "D04TUPV1-N-RECOVERY-R01-NOT-ADMITTED":
        value["operationKind"], value["operationIntent"]["kind"], value["operationCompletion"]["kind"] = "0002", "0002", "0002"
        value["operationCompletion"]["result"] = "0001"
    elif vector_id == "D04TUPV1-N-RECOVERY-R02-NOT-ADMITTED":
        value["operationKind"], value["operationIntent"]["kind"], value["operationCompletion"]["kind"] = "0002", "0002", "0002"
        value["operationCompletion"]["result"] = "0002"
    elif vector_id == "D04TUPV1-N-UNCERTAINTY-ABSENT":
        value["ownerWal"]["ownerWalState"] = "0000"
    elif vector_id == "D04TUPV1-N-UNCERTAINTY-TERMINAL":
        value["ownerWal"]["ownerWalState"] = "0020"
    elif vector_id == "D04TUPV1-N-R07-NORMAL-OWNER":
        value["ownerWal"]["ownerWalState"] = "0020"
    elif vector_id == "D04TUPV1-N-R07-W0027-NOT-ALLOWED":
        value["ownerWal"]["ownerWalState"] = "0027"
    elif vector_id == "D04TUPV1-N-MAINTENANCE-ACTOR-MISMATCH":
        value["operationCompletion"]["actor"] = d04_actor(1)
    elif vector_id == "D04TUPV1-N-MAINTENANCE-CLEAN-STATE":
        value["recordState"] = "000c"
    elif vector_id == "D04TUPV1-N-MAINTENANCE-OWNER-REWRITE":
        value["ownerWal"]["ownerWalState"] = "0024"
    elif vector_id == "D04TUPV1-N-PROVISION-SID-MISMATCH":
        value["initialProvision"]["designatedOwnerSidDigest"] = hex_digest("wrong-provision-sid")
    elif vector_id == "D04TUPV1-N-PROVISION-CHECKPOINT-ZERO":
        value["initialProvision"]["provisionCheckpointDigest"] = ZERO32
    elif vector_id == "D04TUPV1-N-PROVISION-ACTOR-MISMATCH":
        value["initialProvision"]["systemMaintenanceActor"] = d04_actor(1)
    elif vector_id == "D04TUPV1-N-PROVISION-TYPED-ABSENCE":
        value["initialProvision"]["ownerWalTypedAbsence"] = "PRESENT"
    elif vector_id == "D04TUPV1-N-PROVISION-FENCE-ZERO":
        value["initialProvision"]["epoch"] = ZERO8
    elif vector_id == "D04TUPV1-N-NORMAL-PAIR-TUPLE-CHANGE":
        wrong_predecessor = d04_tuple_id(1, 4, 0x22, 8, "TERMINALIZING")
        wrong_digest = digest(d04_tuple_bytes(wrong_predecessor)).hex()
        value["terminalPair"]["terminalizingVectorId"] = wrong_predecessor
        value["terminalPair"]["terminalizingFixtureSha256"] = wrong_digest
        value["terminalPair"]["terminalCleanPredecessorSha256"] = wrong_digest
    if vector_id in D04_NEGATIVE_EVIDENCE_BOUNDARIES:
        kind, result, owner = D04_NEGATIVE_EVIDENCE_BOUNDARIES[vector_id]
        retained = "BYTE_EXACT_NORMAL" if owner in range(0x20, 0x24) else "LOSSLESS_CRITICAL" if owner >= 0x24 else "LAST_VALID_NONTERMINAL"
        evidence_id = d04_negative_evidence_id(vector_id)
        value["ownerWal"] = d04_owner_provenance(owner, retained=retained)
        value["criticalEvidence"] = d04_evidence_object(vector_id, kind, result, owner, evidence_id)
    if vector_id == "D04TUPV1-N-MAINTENANCE-OWNER-REWRITE":
        value["ownerWal"] = d04_owner_provenance(0x0024, retained="LOSSLESS_CRITICAL")
    if vector_id == "D04TUPV1-N-MAINTENANCE-CLEAN-STATE":
        value["machineActorLayoutFixtureId"] = d04_machine_actor_fixture(12, "NORMAL_RETAINED")
        value["machineActorLayoutFixtureSha256"] = digest(mar_bytes(value["machineActorLayoutFixtureId"])).hex()
        value["criticalEvidence"] = d04_evidence_object(
            vector_id, 3, 1, 0x0020, d04_negative_evidence_id(vector_id)
        )
    if vector_id in {
        "D04TUPV1-N-RECOVERY-R01-NOT-ADMITTED",
        "D04TUPV1-N-RECOVERY-R02-NOT-ADMITTED",
        "D04TUPV1-N-UNINSTALL-NOT-ADMITTED",
    }:
        value.pop("terminalPair", None)
    projection = d04_tuple_projection(value)
    value["tupleProjection"] = projection
    value["tupleProjectionSha256"] = digest(canonical_json(projection)).hex()
    return canonical_json(value)


def check_d04_tuple(vector_id: str, data: bytes) -> bool:
    try:
        parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
        counts = parsed["actionCounts"]
        p, q = parsed["operationIntent"], parsed["operationCompletion"]
    except (UnicodeDecodeError, json.JSONDecodeError, KeyError):
        return False
    if any(value != 0 for value in counts.values()):
        return False
    projection = d04_tuple_projection(parsed)
    if parsed.get("tupleProjection") != projection or parsed.get("tupleProjectionSha256") != digest(canonical_json(projection)).hex():
        return False
    if vector_id in D04_ALLOWED_TUPLES:
        kind, result, owner, state, variant = D04_ALLOWED_TUPLES[vector_id]
        actor = d04_actor(kind)
        required_suffix = {1: ["completionEvidenceDigest"], 2: ["completionEvidenceDigest"], 3: ["binarySetDigest", "signatureEvidenceDigest", "recoveryReaderDigest", "retentionDigest"], 4: ["binarySetDigest", "signatureEvidenceDigest", "recoveryReaderDigest", "retentionDigest"], 5: ["binarySetDigest", "signatureEvidenceDigest", "recoveryReaderDigest", "retentionDigest"], 7: ["provisionCheckpointDigest"]}[kind]
        if not (
            parsed.get("schema") == "D04TupleWitnessV1" and parsed.get("vectorId") == vector_id
            and parsed.get("operationKind") == f"{kind:04x}" and parsed.get("recordState") == f"{state:04x}"
            and parsed.get("variant") == variant and parsed.get("classification") == "D04_EXACT_ALLOWED_TUPLE"
            and p.get("schema") == q.get("schema") == "0001" and p.get("kind") == q.get("kind") == f"{kind:04x}"
            and p.get("operationNonce") == q.get("operationNonce") == parsed.get("operationNonce")
            and p.get("actor") == q.get("actor") == actor and q.get("result") == f"{result:04x}"
            and list(q)[5:] == required_suffix and all(q[key] != ZERO32 for key in required_suffix)
        ):
            return False
        owner_wal = parsed.get("ownerWal", {})
        retained = "BYTE_EXACT_NORMAL" if owner in range(0x20, 0x24) else "LOSSLESS_CRITICAL" if owner >= 0x24 else "LAST_VALID_NONTERMINAL"
        if owner_wal != d04_owner_provenance(owner, retained=retained):
            return False
        if variant == "TERMINAL_CLEAN" and parsed.get("terminalPair") != d04_normal_pair(kind, result, owner):
            return False
        if variant != "TERMINAL_CLEAN" and "terminalPair" in parsed:
            return False
        layout_id = d04_machine_actor_fixture(state, variant)
        if (
            parsed.get("machineActorLayoutFixtureId") != layout_id
            or parsed.get("machineActorLayoutFixtureSha256") != digest(mar_bytes(layout_id)).hex()
            or parsed.get("machineActorLayoutOnly") is not True
        ):
            return False
        if "uncertaintyEvidence" in parsed:
            return False
        if state == 12 and parsed.get("criticalEvidence") != d04_critical_evidence(vector_id):
            return False
        if state != 12 and "criticalEvidence" in parsed:
            return False
        if variant.startswith("PROVISION_"):
            provision = parsed.get("initialProvision", {})
            if not (provision.get("bootId") != ZERO32 and provision.get("epoch") != ZERO8 and provision.get("lease") != ZERO8
                    and provision.get("designatedOwnerSidDigest") == owner_sid_digest() and provision.get("systemMaintenanceActor") == actor_ref("system-maintenance", 4)
                    and provision.get("ownerWalTypedAbsence") == provision.get("terminalTypedAbsence") == "ABSENT_EXPECTED"
                    and provision.get("provisionCheckpointDigest") != ZERO32 and provision.get("provisionRecordState") == "0005"
                    and provision.get("provisionRecordVectorId") == "MAPRV1-P-STATE-05"
                    and provision.get("provisionRecordFixtureSha256") == digest(map_bytes("MAPRV1-P-STATE-05")).hex()
                    and provision.get("machineActorTupleSource") == "THIS_D04_TUPLE_PROJECTION"):
                return False
            if not (provision.get("readiness") == "STRUCTURAL_ONLY" and provision.get("packageCompletionDigest") == ZERO32):
                return False
        elif "initialProvision" in parsed:
            return False
        projection = d04_tuple_projection(parsed)
        return parsed.get("tupleProjection") == projection and parsed.get("tupleProjectionSha256") == digest(canonical_json(projection)).hex()
    # Negative predicates intentionally check the exact prohibited field rather
    # than accept a generic malformed JSON object.
    if parsed.get("schema") != "D04TupleWitnessV1-reject" or parsed.get("vectorId") != vector_id or parsed.get("classification") != "D04_REJECT_FAIL_CLOSED":
        return False
    if vector_id == "D04TUPV1-N-NONE-RESERVED": return q.get("result") == "0000"
    if vector_id == "D04TUPV1-N-UNINSTALL-NOT-ADMITTED": return parsed.get("operationKind") == "0006" and q.get("result") == "0001" and parsed.get("variant") == "UNINSTALL_RESERVED"
    if vector_id == "D04TUPV1-N-PQ-NONCE-MISMATCH": return p.get("operationNonce") != q.get("operationNonce")
    if vector_id == "D04TUPV1-N-ACTOR-MISMATCH": return p.get("actor") != q.get("actor")
    if vector_id == "D04TUPV1-N-VARIANT-MISMATCH": return "completionEvidenceDigest" not in q and "tombstoneDigest" in q
    if vector_id == "D04TUPV1-N-OWNER-PROVENANCE-MISSING": return parsed.get("ownerWal", {}).get("ownerTerminalDigest") == ZERO32
    if vector_id == "D04TUPV1-N-UNCERTAINTY-EVIDENCE-MISSING": return "criticalEvidence" not in parsed
    if vector_id == "D04TUPV1-N-PROVISION-STATE5-MISSING": return parsed.get("initialProvision", {}).get("provisionRecordState") != "0005"
    if vector_id == "D04TUPV1-N-PROVISION-STATE6-PREMATURE": return parsed.get("initialProvision", {}).get("provisionRecordState") == "0006" and parsed.get("initialProvision", {}).get("packageCompletionDigest") != ZERO32
    if vector_id == "D04TUPV1-N-NORMAL-PREDECESSOR-MISMATCH": return parsed.get("terminalPair", {}).get("terminalCleanPredecessorSha256") == hex_digest("wrong-d04-predecessor")
    if vector_id == "D04TUPV1-N-OWNER-WAL-FIXTURE-MISMATCH": return parsed.get("ownerWal", {}).get("sourceOwnerWalFixtureSha256") == hex_digest("wrong-owner-wal-fixture")
    if vector_id == "D04TUPV1-N-UNCERTAINTY-EMPTY": return parsed.get("ownerWal", {}).get("ownerWalState") == "0001"
    if vector_id == "D04TUPV1-N-PROVISION-BOOT-ZERO": return parsed.get("initialProvision", {}).get("bootId") == ZERO32
    if vector_id == "D04TUPV1-N-RECOVERY-R01-NOT-ADMITTED": return parsed.get("operationKind") == "0002" and q.get("result") == "0001"
    if vector_id == "D04TUPV1-N-RECOVERY-R02-NOT-ADMITTED": return parsed.get("operationKind") == "0002" and q.get("result") == "0002"
    if vector_id == "D04TUPV1-N-UNCERTAINTY-ABSENT": return parsed.get("ownerWal", {}).get("ownerWalState") == "0000"
    if vector_id == "D04TUPV1-N-UNCERTAINTY-TERMINAL": return parsed.get("ownerWal", {}).get("ownerWalState") == "0020"
    if vector_id == "D04TUPV1-N-R07-NORMAL-OWNER": return q.get("result") == "0007" and parsed.get("ownerWal", {}).get("ownerWalState") == "0020"
    if vector_id == "D04TUPV1-N-R07-W0027-NOT-ALLOWED": return q.get("result") == "0007" and parsed.get("ownerWal", {}).get("ownerWalState") == "0027"
    if vector_id == "D04TUPV1-N-MAINTENANCE-ACTOR-MISMATCH": return p.get("actor") != q.get("actor")
    if vector_id == "D04TUPV1-N-MAINTENANCE-CLEAN-STATE": return parsed.get("recordState") == "000c"
    if vector_id == "D04TUPV1-N-MAINTENANCE-OWNER-REWRITE": return parsed.get("ownerWal", {}).get("ownerWalState") == "0024"
    if vector_id == "D04TUPV1-N-PROVISION-SID-MISMATCH": return parsed.get("initialProvision", {}).get("designatedOwnerSidDigest") == hex_digest("wrong-provision-sid")
    if vector_id == "D04TUPV1-N-PROVISION-CHECKPOINT-ZERO": return parsed.get("initialProvision", {}).get("provisionCheckpointDigest") == ZERO32
    if vector_id == "D04TUPV1-N-PROVISION-ACTOR-MISMATCH": return parsed.get("initialProvision", {}).get("systemMaintenanceActor") == d04_actor(1)
    if vector_id == "D04TUPV1-N-PROVISION-TYPED-ABSENCE": return parsed.get("initialProvision", {}).get("ownerWalTypedAbsence") == "PRESENT"
    if vector_id == "D04TUPV1-N-PROVISION-FENCE-ZERO": return parsed.get("initialProvision", {}).get("epoch") == ZERO8
    if vector_id == "D04TUPV1-N-NORMAL-PAIR-TUPLE-CHANGE": return parsed.get("terminalPair", {}).get("terminalizingVectorId") == d04_tuple_id(1, 4, 0x22, 8, "TERMINALIZING")
    return False


def check_d04_d02(vector_id: str, data: bytes) -> bool:
    try:
        bound_id, kind, result, owner = D04_EVIDENCE_BINDINGS[vector_id]
        evidence_kind, error_class, detail = d04_evidence_parameters(result)
        parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
        inputs = parsed["inputs"]
    except (ValueError, UnicodeDecodeError, json.JSONDecodeError, KeyError):
        return False
    expected = critical_evidence_inputs(evidence_kind, error_class, detail, f"d04-{bound_id}")
    return (
        parsed.get("schema") == "D04CriticalEvidenceKnownAnswerV1" and parsed.get("vectorId") == vector_id
        and parsed.get("boundD04WitnessVectorId") == bound_id and parsed.get("operationKind") == f"{kind:04x}" and parsed.get("operationResult") == f"{result:04x}" and parsed.get("ownerWalState") == f"{owner:04x}"
        and parsed.get("evidenceKind") == f"{evidence_kind:04x}" and parsed.get("errorClass") == f"{error_class:04x}" and parsed.get("detail") == f"{detail:04x}"
        and parsed.get("inputs") == expected and parsed.get("preservedEvidenceDigest") == expected["preservedEvidenceDigest"]
        and parsed.get("classification") == "D04_CRITICAL_EVIDENCE_EXACT"
        and all(value == 0 for value in parsed.get("actionCounts", {}).values())
    )


def check_d04_readiness(data: bytes) -> bool:
    try:
        parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
        state6 = map_bytes("MAPRV1-P-STATE-06")
        slot = state6[MAP_HEADER_SIZE + MAP_SLOT_SIZE:MAP_HEADER_SIZE + 2 * MAP_SLOT_SIZE]
        payload_length = int.from_bytes(slot[28:32], "little")
        payload = json.loads(slot[224:224 + payload_length].decode("ascii"), object_pairs_hook=OrderedDict)
    except (UnicodeDecodeError, json.JSONDecodeError, KeyError, ValueError):
        return False
    structural = d04_tuple_id(7, 1, 0, 9, "PROVISION_STRUCTURAL")
    return (
        parsed.get("schema") == "D04ProvisionReadinessCompanionV1" and parsed.get("vectorId") == D04_READINESS_ID
        and parsed.get("structuralTupleVectorId") == structural and parsed.get("structuralTupleFixtureSha256") == digest(d04_tuple_bytes(structural)).hex()
        and parsed.get("provisionRecordVectorId") == "MAPRV1-P-STATE-06" and parsed.get("provisionRecordFixtureSha256") == digest(state6).hex()
        and parsed.get("provisionRecordState") == "0006" and parsed.get("packageCompletionDigest") == payload["packageCompletionDigest"]
        and parsed.get("readiness") == "DURABLE_PACKAGE_COMPLETION_EXACT" and parsed.get("runtimeWriteAuthorized") is False
        and parsed.get("classification") == "D04_R04_READINESS_EVIDENCE_ONLY"
        and all(value == 0 for value in parsed.get("actionCounts", {}).values())
    )


def completion(kind: int, actor: OrderedDict[str, Any], nonce: str) -> OrderedDict[str, Any]:
    # Q repeats kind/nonce so P, Q, and top-level P group bind byte-exactly.
    common: list[tuple[str, Any]] = [("schema", "0001"), ("kind", f"{kind:04x}"), ("operationNonce", nonce), ("result", "0001"), ("actor", actor)]
    if kind in (1, 2):
        common.append(("completionEvidenceDigest", hex_digest("completion-evidence")))
    elif kind in (3, 4, 5):
        common.extend((
            ("binarySetDigest", hex_digest("binary-set")),
            ("signatureEvidenceDigest", hex_digest("signature")),
            ("recoveryReaderDigest", hex_digest("reader")),
            ("retentionDigest", hex_digest("retention")),
        ))
    elif kind == 6:
        common.extend((("tombstoneDigest", hex_digest("tombstone")), ("retentionDigest", hex_digest("retention"))))
    else:
        common.append(("provisionCheckpointDigest", hex_digest("provision-checkpoint")))
    return OrderedDict(common)


def intent(kind: int, actor: OrderedDict[str, Any], nonce: str) -> OrderedDict[str, Any]:
    result: list[tuple[str, Any]] = [
        ("schema", "0001"), ("kind", f"{kind:04x}"), ("operationNonce", nonce), ("actor", actor),
        ("expectedRecordStateVersion", "0000000000000001"), ("targetDigest", hex_digest("target")),
        ("planDigest", hex_digest("plan")), ("detailsDigest", hex_digest("details")),
    ]
    if kind in (4, 5, 6):
        result.extend((("fromBinaryVersion", "1.0.0"), ("toBinaryVersion", "1.0.1")))
    return OrderedDict(result)


MASTER_KEYS = (
    "bootId activeDisplayId ownerSid ownerLogonId ownerSessionId ownerWalPathDigest "
    "ownerWalGeneration ownerWalState ownerTerminalDigest controllerInstanceId "
    "controllerProcessIdentity watchdogInstanceId watchdogProcessIdentity workerInstanceId "
    "workerProcessIdentity binaryVersion recoveryBinaryVersion createdWallClock updatedWallClock "
    "operationKind operationNonce operationIntent operationCompletion terminalGeneration "
    "lastErrorClass lastErrorDetailCode preservedEvidenceDigest"
).split()


def machine_payload(state: int, variant: str = "normal") -> bytes:
    provisioned_clean = variant == "provisioned-clean"
    maintenance = state in (10, 11) or provisioned_clean
    kind = 7 if maintenance else 1
    actor = actor_ref("maintenance", 4) if maintenance else actor_ref("watchdog", 2)
    nonce = hex_id("operation")
    values: dict[str, Any] = {
        "bootId": hex_digest("boot"), "activeDisplayId": hex_digest("display"), "ownerSid": sid_value(),
        "ownerLogonId": "1020304050607080", "ownerSessionId": "00000001", "ownerWalPathDigest": hex_digest("wal-path"),
        "ownerWalGeneration": "0000000000000001", "ownerWalState": "00000008", "ownerTerminalDigest": hex_digest("terminal"),
        "controllerInstanceId": hex_id("controller"), "controllerProcessIdentity": process_identity("controller", 1),
        # D04: every active P/Q watchdog actor is byte-exactly the required W
        # group.  actor_ref("watchdog", 2) derives this exact instance value.
        "watchdogInstanceId": hex_id("watchdog-instance"), "watchdogProcessIdentity": process_identity("watchdog", 2),
        "workerInstanceId": hex_id("worker"), "workerProcessIdentity": process_identity("worker", 3),
        "binaryVersion": "1.0.0", "recoveryBinaryVersion": "1.0.0",
        "createdWallClock": "01d0000000000000", "updatedWallClock": "01d0000000000001",
        "operationKind": f"{kind:04x}", "operationNonce": nonce, "operationIntent": intent(kind, actor, nonce),
        "operationCompletion": completion(kind, actor, nonce), "terminalGeneration": "0000000000000001",
        "lastErrorClass": "0000", "lastErrorDetailCode": "0000", "preservedEvidenceDigest": ZERO32,
    }
    wal_states = {1: "0000", 2: "0008", 3: "0009", 4: "000c", 5: "000d", 6: "0011", 7: "0012", 8: "0020", 9: "0020"}
    if state in wal_states:
        values["ownerWalState"] = wal_states[state]
    if state in (0, 6, 12):
        details = {"fc-01": (1, 2, 1), "fc-02": (8, 7, 6), "fc-03": (1, 3, 3)}
        evidence_kind, error_class, detail = details.get(variant, (4, 4, 4) if state == 6 else (1, 2, 1))
        values["lastErrorClass"], values["lastErrorDetailCode"] = f"{error_class:04x}", f"{detail:04x}"
        values["preservedEvidenceDigest"] = critical_evidence_digest(evidence_kind, error_class, detail, f"mar-{state}-{variant}")
    if state == 1:
        values["ownerWalGeneration"], values["ownerWalState"] = ZERO8, "0000"
    if state in (10, 11):
        values["ownerTerminalDigest"], values["terminalGeneration"] = ZERO32, ZERO8
    if provisioned_clean:
        values["activeDisplayId"], values["ownerWalPathDigest"], values["ownerTerminalDigest"] = ZERO32, ZERO32, ZERO32
        values["ownerLogonId"], values["ownerSessionId"], values["ownerWalGeneration"], values["ownerWalState"], values["terminalGeneration"] = ZERO8, ZERO4, ZERO8, "0000", ZERO8
    if variant == "fc-01":
        values["ownerWalState"] = "0028"
        values["operationCompletion"]["result"] = "0007"
    elif variant == "fc-02":
        values["ownerWalState"] = "0011"
        values["operationCompletion"] = completion(kind, actor, nonce)
        values["operationCompletion"]["result"] = "0006"
    groups: dict[int, tuple[str, ...]] = {
        0: ("H", "E"), 1: ("H", "O", "C", "W", "P"), 2: ("H", "O", "C", "W", "P"),
        3: ("H", "O", "C", "W", "P"), 4: ("H", "O", "C", "W", "X", "P"),
        5: ("H", "O", "C", "W", "P"), 6: ("H", "O", "W", "P", "E"),
        7: ("H", "O", "W", "P"), 8: ("H", "O", "W", "T", "P", "Q"),
        9: ("H", "O", "T", "P", "Q"), 10: ("H", "T", "P"),
        11: ("H", "T", "P"), 12: ("H", "E"),
    }
    if variant == "fc-01":
        groups[12] = ("H", "O", "T", "P", "Q", "E")
    elif variant == "fc-02":
        groups[12] = ("H", "O", "P", "Q", "E")
    members = {
        "H": ("bootId", "binaryVersion", "recoveryBinaryVersion", "createdWallClock", "updatedWallClock"),
        "O": ("activeDisplayId", "ownerSid", "ownerLogonId", "ownerSessionId", "ownerWalPathDigest", "ownerWalGeneration", "ownerWalState"),
        "C": ("controllerInstanceId", "controllerProcessIdentity"),
        "W": ("watchdogInstanceId", "watchdogProcessIdentity"),
        "X": ("workerInstanceId", "workerProcessIdentity"),
        "T": ("ownerTerminalDigest", "terminalGeneration"),
        "P": ("operationKind", "operationNonce", "operationIntent"),
        "Q": ("operationCompletion",),
        "E": ("lastErrorClass", "lastErrorDetailCode", "preservedEvidenceDigest"),
    }
    selected = {field for group in groups[state] for field in members[group]}
    return canonical_json(OrderedDict((key, values[key]) for key in MASTER_KEYS if key in selected))


def mar_slot(slot_index: int, state: int, payload: bytes) -> bytes:
    if len(payload) > 32_768:
        raise ValueError("MAR payload capacity")
    slot = bytearray(MAR_SLOT_SIZE)
    slot[0:16] = bytes.fromhex("4d4152534c4f54310000000000000000")
    write_u16(slot, 16, 1)
    write_u16(slot, 18, 1)
    slot[20], slot[21] = slot_index, state
    write_u32(slot, 24, 136 + len(payload))
    write_u32(slot, 28, len(payload))
    for offset in (32, 40, 48, 56, 64):
        write_u64(slot, offset, 1)
    slot[72:104] = digest(payload)
    slot[136 : 136 + len(payload)] = payload
    covered = bytearray(slot[: 136 + len(payload)])
    covered[104:136] = b"\0" * 32
    slot[104:136] = digest(bytes(covered))
    return bytes(slot)


def refresh_mar_integrity(file_bytes: bytearray) -> None:
    """Recompute MAR payload/slot checksums after a semantic negative mutation."""
    start = MAR_HEADER_SIZE
    length = int.from_bytes(file_bytes[start + 24 : start + 28], "little")
    payload = bytes(file_bytes[start + 136 : start + length])
    file_bytes[start + 72 : start + 104] = digest(payload)
    covered = bytearray(file_bytes[start : start + length])
    covered[104:136] = b"\0" * 32
    file_bytes[start + 104 : start + 136] = digest(bytes(covered))


MAR_NEGATIVE_CASES = {
    "MARV1-N-GROUP-01": ("forbidden T key in ACTIVE_INTENT", "forbidden-key", "checksums-repaired"),
    "MARV1-N-GROUP-02": ("partial X optional group in ACTIVE_APPLY_ARMED", "partial-group", "checksums-repaired"),
    "MARV1-N-GROUP-03": ("partial E group in RECOVERY_REQUIRED", "partial-E", "checksums-repaired"),
    "MARV1-N-JSON-01": ("JSON null scalar", "null", "checksums-repaired"),
    "MARV1-N-JSON-02": ("JSON number scalar", "number", "checksums-repaired"),
    "MARV1-N-JSON-03": ("escaped canonical scalar", "escape", "checksums-repaired"),
    "MARV1-N-JSON-04": ("JSON whitespace", "whitespace", "checksums-repaired"),
    "MARV1-N-JSON-05": ("master-key order inversion", "order", "checksums-repaired"),
    "MARV1-N-JSON-06": ("duplicate JSON key", "duplicate-key", "checksums-repaired"),
    "MARV1-N-JSON-07": ("unknown JSON key", "unknown-key", "checksums-repaired"),
    "MARV1-N-JSON-08": ("trailing JSON whitespace", "trailing-whitespace", "checksums-repaired"),
    "MARV1-N-ENUM-01": ("unknown recordState enum", "unknown-record-state", "checksums-repaired"),
    "MARV1-N-ENUM-02": ("unknown OwnerWalLinkState enum", "unknown-owner-wal-state", "checksums-repaired"),
    "MARV1-N-ENUM-03": ("unknown OperationResult enum", "unknown-result", "checksums-repaired"),
    "MARV1-N-ENUM-04": ("unknown critical evidence detail", "unknown-detail", "checksums-repaired"),
    "MARV1-N-FENCE-01": ("zero leaseVersion", "zero-lease", "checksums-repaired"),
    "MARV1-N-FENCE-02": ("non-current leaseVersion", "old-lease", "checksums-repaired"),
    "MARV1-N-FENCE-03": ("watchdog instance/process binding mismatch", "actor-mismatch", "checksums-repaired"),
    "MARV1-N-FENCE-04": ("slot A bytes copied to physical B", "slot-copy", "checksums-repaired"),
    "MARV1-N-PROVISION-01": ("old direct TERMINAL_CLEAN sentinel with zero bootId", "old-direct-sentinel", "checksums-repaired"),
    "MARV1-N-PROVISION-02": ("hybrid sentinel with zero epoch", "hybrid-sentinel", "checksums-repaired"),
    "MARV1-N-PROVISION-03": ("terminal completion carrying nonterminal result", "wrong-completion-variant", "checksums-repaired"),
    "MARV1-N-PROVISION-04": ("critical E group with zero required outer digest", "zero-required-digest", "checksums-repaired"),
    "MARV1-N-BIND-01": ("operationCompletion Q without complete P group", "q-without-p", "checksums-repaired"),
    "MARV1-N-BIND-02": ("completion carries a wrong tagged-variant key", "wrong-variant-key", "checksums-repaired"),
    "MARV1-N-BIND-03": ("intent schema literal mismatch", "p-schema-mismatch", "checksums-repaired"),
    "MARV1-N-BIND-04": ("completion kind differs from P/top-level kind", "q-kind-mismatch", "checksums-repaired"),
    "MARV1-N-BIND-05": ("completion nonce differs from P/top-level nonce", "q-nonce-mismatch", "checksums-repaired"),
    "MARV1-N-BIND-06": ("top-level P nonce differs from nested P nonce", "top-p-nonce-mismatch", "checksums-repaired"),
    "MARV1-N-BIND-07": ("completion actor differs from required historical actor", "q-actor-mismatch", "checksums-repaired"),
    "MARV1-N-BIND-08": ("completion result has malformed u16 width", "malformed-result", "checksums-repaired"),
    "MARV1-N-BIND-09": ("required tagged completion digest is zero", "zero-completion-digest", "checksums-repaired"),
    "MARV1-N-D02-01": ("known detail has wrong required error class", "d02-class-mismatch", "checksums-repaired"),
    "MARV1-N-D02-02": ("known detail has wrong required evidence kind/digest", "d02-evidence-kind-mismatch", "checksums-repaired"),
    "MARV1-N-D02-03": ("ordinary record is illegally elevated into critical E evidence", "d02-ordinary-elevation", "checksums-repaired"),
    "MARV1-N-STRUCT-01": ("payloadLength does not equal canonical payload bytes", "payload-length", "checksum unavailable"),
    "MARV1-N-STRUCT-02": ("payload contains non-ASCII byte", "payload-nonascii", "checksum repaired"),
    "MARV1-N-STRUCT-03": ("dedicated slot checksum mismatch", "slot-checksum", "checksum intentionally invalid"),
    "MARV1-N-BIND-10": ("completion schema literal mismatch", "q-schema-mismatch", "checksums-repaired"),
    "MARV1-N-BIND-11": ("completion kind uses uppercase hex", "q-kind-uppercase", "checksums-repaired"),
    "MARV1-N-BIND-12": ("completion nonce uses uppercase hex", "q-nonce-uppercase", "checksums-repaired"),
    "MARV1-N-JSON-09": ("JSON boolean scalar", "boolean", "checksums-repaired"),
    "MARV1-N-JSON-10": ("JSON array scalar", "array", "checksums-repaired"),
}


def mar_file(state: int, payload: bytes, *, slots: dict[int, bytes] | None = None) -> bytes:
    header = envelope_header(bytes.fromhex("44444d41525631000000000000000000"), MAR_SLOT_SIZE, MAR_FILE_LENGTH)
    return record_file(header, MAR_SLOT_SIZE, {0: mar_slot(0, state, payload)} if slots is None else slots)


def mar_negative_payload(vector_id: str) -> tuple[int, bytes]:
    """Table-driven, syntactically observable MAR negative payload mutation."""
    if vector_id == "MARV1-N-GROUP-01":
        value = json.loads(machine_payload(1), object_pairs_hook=OrderedDict)
        value["ownerTerminalDigest"] = hex_digest("forbidden-terminal")
        return 1, canonical_json(OrderedDict((key, value[key]) for key in MASTER_KEYS if key in value))
    if vector_id == "MARV1-N-GROUP-02":
        value = json.loads(machine_payload(4), object_pairs_hook=OrderedDict)
        value.pop("workerProcessIdentity")
        return 4, canonical_json(value)
    if vector_id == "MARV1-N-GROUP-03":
        value = json.loads(machine_payload(6), object_pairs_hook=OrderedDict)
        value.pop("preservedEvidenceDigest")
        return 6, canonical_json(value)
    if vector_id.startswith("MARV1-N-JSON-"):
        number = int(vector_id[-2:])
        value = json.loads(machine_payload(1), object_pairs_hook=OrderedDict)
        if number == 1:
            value["ownerWalGeneration"] = None
            return 1, canonical_json(value)
        if number == 2:
            value["ownerSessionId"] = 1
            return 1, canonical_json(value)
        if number == 9:
            return 1, canonical_json(value).replace(b'"ownerSessionId":"00000001"', b'"ownerSessionId":true')
        if number == 10:
            return 1, canonical_json(value).replace(b'"ownerSessionId":"00000001"', b'"ownerSessionId":[]')
        canonical = canonical_json(value)
        if number == 3:
            return 1, canonical.replace(b'"binaryVersion":"1.0.0"', b'"binaryVersion":"1.0\\u002e0"')
        if number == 4:
            return 1, canonical.replace(b"{", b"{ ", 1)
        if number == 5:
            reordered = OrderedDict((("updatedWallClock", value.pop("updatedWallClock")), *value.items()))
            return 1, canonical_json(reordered)
        if number == 6:
            prefix = b'"bootId":"' + value["bootId"].encode("ascii") + b'",'
            return 1, canonical.replace(prefix, prefix + prefix, 1)
        if number == 7:
            value["unexpected"] = "0000"
            return 1, canonical_json(value)
        return 1, canonical + b" "
    if vector_id == "MARV1-N-ENUM-02":
        value = json.loads(machine_payload(1), object_pairs_hook=OrderedDict)
        value["ownerWalState"] = "0015"
        return 1, canonical_json(value)
    if vector_id == "MARV1-N-ENUM-03":
        value = json.loads(machine_payload(12, "fc-01"), object_pairs_hook=OrderedDict)
        value["operationCompletion"]["result"] = "ffff"
        return 12, canonical_json(value)
    if vector_id == "MARV1-N-ENUM-04":
        value = json.loads(machine_payload(0), object_pairs_hook=OrderedDict)
        value["lastErrorDetailCode"] = "ffff"
        value["preservedEvidenceDigest"] = critical_evidence_digest(1, int(value["lastErrorClass"], 16), 0xffff)
        return 0, canonical_json(value)
    if vector_id == "MARV1-N-FENCE-03":
        value = json.loads(machine_payload(1), object_pairs_hook=OrderedDict)
        value["watchdogInstanceId"] = hex_id("wrong-watchdog-instance")
        return 1, canonical_json(value)
    if vector_id == "MARV1-N-PROVISION-01":
        value = json.loads(machine_payload(9), object_pairs_hook=OrderedDict)
        value["bootId"] = ZERO32
        value["operationKind"] = "0000"
        value["operationIntent"]["kind"] = "0000"
        value["operationCompletion"]["kind"] = "0000"
        return 9, canonical_json(value)
    if vector_id == "MARV1-N-PROVISION-03":
        value = json.loads(machine_payload(9, "provisioned-clean"), object_pairs_hook=OrderedDict)
        completion_value = value["operationCompletion"]
        completion_value.pop("provisionCheckpointDigest")
        completion_value["tombstoneDigest"] = hex_digest("wrong-variant")
        return 9, canonical_json(value)
    if vector_id == "MARV1-N-PROVISION-04":
        value = json.loads(machine_payload(6), object_pairs_hook=OrderedDict)
        value["preservedEvidenceDigest"] = ZERO32
        return 6, canonical_json(value)
    if vector_id.startswith("MARV1-N-BIND-"):
        number = int(vector_id[-2:])
        value = json.loads(machine_payload(12, "fc-01"), object_pairs_hook=OrderedDict)
        if number == 1:
            for key in ("operationKind", "operationNonce", "operationIntent"):
                value.pop(key)
        elif number == 2:
            value["operationCompletion"].pop("completionEvidenceDigest")
            value["operationCompletion"]["tombstoneDigest"] = hex_digest("wrong-variant-bind")
        elif number == 3:
            value["operationIntent"]["schema"] = "0002"
        elif number == 4:
            value["operationCompletion"]["kind"] = "0002"
        elif number == 5:
            value["operationCompletion"]["operationNonce"] = hex_id("wrong-q-nonce")
        elif number == 6:
            value["operationNonce"] = hex_id("wrong-top-nonce")
        elif number == 7:
            value["operationCompletion"]["actor"] = actor_ref("wrong-actor", 2)
        elif number == 8:
            value["operationCompletion"]["result"] = "1"
        elif number == 9:
            value["operationCompletion"]["completionEvidenceDigest"] = ZERO32
        elif number == 10:
            value["operationCompletion"]["schema"] = "0002"
        elif number == 11:
            value["operationCompletion"]["kind"] = "000A"
        else:
            value["operationCompletion"]["operationNonce"] = value["operationCompletion"]["operationNonce"].upper()
        return 12, canonical_json(value)
    if vector_id.startswith("MARV1-N-D02-"):
        number = int(vector_id[-2:])
        if number == 3:
            value = json.loads(machine_payload(1), object_pairs_hook=OrderedDict)
            value["lastErrorClass"], value["lastErrorDetailCode"], value["preservedEvidenceDigest"] = "0002", "0001", critical_evidence_digest(1, 2, 1)
            return 1, canonical_json(OrderedDict((key, value[key]) for key in MASTER_KEYS if key in value))
        value = json.loads(machine_payload(6), object_pairs_hook=OrderedDict)
        if number == 1:
            value["lastErrorDetailCode"] = "0006"
            value["lastErrorClass"] = "0001"
            value["preservedEvidenceDigest"] = critical_evidence_digest(8, 1, 6)
        else:
            value["preservedEvidenceDigest"] = critical_evidence_digest(1, 7, 6)
        return 6, canonical_json(value)
    raise ValueError(f"unhandled MAR negative {vector_id}")


def mar_bytes(vector_id: str) -> bytes:
    if "-N-" not in vector_id:
        state = int(vector_id[-2:], 16) if "P-STATE-" in vector_id else (12 if "P-FC-" in vector_id else 9)
        variant = "fc-" + vector_id[-2:] if "P-FC-" in vector_id else ("provisioned-clean" if "PROVISIONED-CLEAN" in vector_id else "normal")
        return mar_file(state, machine_payload(state, variant))
    if vector_id == "MARV1-N-ENUM-01":
        base = bytearray(mar_file(1, machine_payload(1)))
        base[MAR_HEADER_SIZE + 21] = 0x7f
        refresh_mar_integrity(base)
        return bytes(base)
    if vector_id == "MARV1-N-FENCE-01" or vector_id == "MARV1-N-FENCE-02":
        base = bytearray(mar_file(1, machine_payload(1)))
        # N-02 observes current lease=2 but carries older lease=1.
        write_u64(base, MAR_HEADER_SIZE + 48, 0 if vector_id.endswith("01") else 1)
        if vector_id.endswith("02"):
            write_u64(base, MAR_HEADER_SIZE + 32, 2)
        refresh_mar_integrity(base)
        return bytes(base)
    if vector_id == "MARV1-N-PROVISION-02":
        base = bytearray(mar_file(9, machine_payload(9, "provisioned-clean")))
        write_u64(base, MAR_HEADER_SIZE + 40, 0)
        refresh_mar_integrity(base)
        return bytes(base)
    if vector_id == "MARV1-N-PROVISION-01":
        state, payload = mar_negative_payload(vector_id)
        base = bytearray(mar_file(state, payload))
        write_u64(base, MAR_HEADER_SIZE + 40, 0)
        write_u64(base, MAR_HEADER_SIZE + 48, 0)
        refresh_mar_integrity(base)
        return bytes(base)
    if vector_id == "MARV1-N-FENCE-04":
        slot_a = mar_slot(0, 1, machine_payload(1))
        return mar_file(1, machine_payload(1), slots={0: slot_a, 1: slot_a})
    if vector_id == "MARV1-N-STRUCT-01":
        base = bytearray(mar_file(1, machine_payload(1)))
        write_u32(base, MAR_HEADER_SIZE + 28, int.from_bytes(base[MAR_HEADER_SIZE + 28 : MAR_HEADER_SIZE + 32], "little") + 1)
        return bytes(base)
    if vector_id == "MARV1-N-STRUCT-02":
        base = bytearray(mar_file(1, machine_payload(1)))
        base[MAR_HEADER_SIZE + 136] = 0x80
        refresh_mar_integrity(base)
        return bytes(base)
    if vector_id == "MARV1-N-STRUCT-03":
        base = bytearray(mar_file(1, machine_payload(1)))
        base[MAR_HEADER_SIZE + 104] ^= 1
        return bytes(base)
    state, payload = mar_negative_payload(vector_id)
    return mar_file(state, payload)


MAP_MASTER_KEYS = (
    "provisionRecordPathDigest machineActorPathDigest directoryAnchorDigest provisionRecordDaclDigest "
    "machineActorDaclDigest provisionRecordAttributeStreamDigest machineActorAttributeStreamDigest "
    "installerManifestDigest designatedOwnerSidDigest creatorLane provisionActor previousSlotDigest "
    "machineActorHeaderDigest machineActorSlotDigest machineActorRecordState packageCompletionDigest "
    "retentionMode failureClass failureEvidenceDigest"
).split()


def map_payload(state: int, previous_slot_digest: str = ZERO32, linked_header_digest: str = ZERO32, linked_slot_digest: str = ZERO32) -> bytes:
    base: dict[str, Any] = {key: hex_digest(key) for key in MAP_MASTER_KEYS if key.endswith("Digest")}
    base.update({
        "creatorLane": "0001", "provisionActor": actor_ref("system-maintenance", 4),
        "machineActorRecordState": "0009" if state >= 3 else "0000", "retentionMode": "0001",
        "failureClass": "0000", "failureEvidenceDigest": ZERO32,
    })
    base["designatedOwnerSidDigest"] = owner_sid_digest()
    base["previousSlotDigest"] = previous_slot_digest
    base["machineActorHeaderDigest"] = linked_header_digest
    base["machineActorSlotDigest"] = linked_slot_digest
    if state == 1:
        for key in ("previousSlotDigest", "machineActorHeaderDigest", "machineActorSlotDigest", "packageCompletionDigest", "failureEvidenceDigest"):
            base[key] = ZERO32
    elif state == 2:
        for key in ("machineActorHeaderDigest", "machineActorSlotDigest", "packageCompletionDigest", "failureEvidenceDigest"):
            base[key] = ZERO32
    elif state in (3, 4, 5):
        base["packageCompletionDigest"], base["failureEvidenceDigest"] = ZERO32, ZERO32
    elif state == 6:
        base["failureEvidenceDigest"] = ZERO32
    elif state == 7:
        base["failureClass"], base["failureEvidenceDigest"] = "0002", hex_digest("map-failure")
    return canonical_json(OrderedDict((key, base[key]) for key in MAP_MASTER_KEYS))


def linked_machine_actor(state: int) -> tuple[str, str, int]:
    record_state = {3: 10, 4: 11, 5: 9, 6: 9}[state]
    fixture_id = {3: "MARV1-P-STATE-0A", 4: "MARV1-P-STATE-0B", 5: "MARV1-P-PROVISIONED-CLEAN", 6: "MARV1-P-PROVISIONED-CLEAN"}[state]
    fixture = mar_bytes(fixture_id)
    return digest(fixture[:MAR_HEADER_SIZE]).hex(), digest(fixture[MAR_HEADER_SIZE : MAR_HEADER_SIZE + MAR_SLOT_SIZE]).hex(), record_state


def map_slot(slot_index: int, state: int, payload: bytes) -> bytes:
    slot = bytearray(MAP_SLOT_SIZE)
    slot[0:16] = bytes.fromhex("4d415052534c4f543100000000000000")
    write_u16(slot, 16, 1)
    write_u16(slot, 18, 1)
    slot[20], slot[21] = slot_index, state
    write_u32(slot, 24, 224 + len(payload))
    write_u32(slot, 28, len(payload))
    write_u64(slot, 32, state)
    write_u64(slot, 40, 1)
    write_u64(slot, 48, state)
    write_u64(slot, 56, 0x1122334455667788)
    slot[64:80] = bytes.fromhex(hex_id("provision-file"))
    write_u64(slot, 80, 0 if state == 1 else 0x1122334455667788)
    slot[88:104] = bytes.fromhex(hex_id("machine-file")) if state >= 2 else b"\0" * 16
    write_u64(slot, 104, MAR_FILE_LENGTH)
    write_u64(slot, 112, 0 if state <= 2 else MAR_FILE_LENGTH)
    write_u64(slot, 120, 0 if state < 3 else state - 2)
    write_u64(slot, 128, 0 if state < 3 else 1)
    write_u64(slot, 136, 0 if state < 3 else 1)
    slot[144:160] = bytes.fromhex(hex_id("provision-nonce"))
    slot[160:192] = digest(payload)
    slot[224 : 224 + len(payload)] = payload
    covered = bytearray(slot[: 224 + len(payload)])
    covered[192:224] = b"\0" * 32
    slot[192:224] = digest(bytes(covered))
    return bytes(slot)


def refresh_map_slot_integrity(file_bytes: bytearray, start: int) -> None:
    length = int.from_bytes(file_bytes[start + 24 : start + 28], "little")
    payload = bytes(file_bytes[start + 224 : start + length])
    file_bytes[start + 160 : start + 192] = digest(payload)
    covered = bytearray(file_bytes[start : start + length])
    covered[192:224] = b"\0" * 32
    file_bytes[start + 192 : start + 224] = digest(bytes(covered))


def map_negative_semantic_slot(number: int) -> bytes:
    payload = json.loads(map_payload(1), object_pairs_hook=OrderedDict)
    if number == 2:
        payload["machineActorPathDigest"] = "f" * 64
    elif number == 3:
        payload["machineActorDaclDigest"] = "e" * 64
    elif number == 4:
        payload["directoryAnchorDigest"] = "d" * 64
    elif number == 5:
        payload["machineActorAttributeStreamDigest"] = "c" * 64
    elif number == 7:
        payload["retentionMode"] = "0002"
    elif number == 8:
        return map_slot(0, 7, map_payload(7))
    return map_slot(0, 1, canonical_json(payload))


def map_bytes(vector_id: str) -> bytes:
    header = envelope_header(bytes.fromhex("44444d41505256310000000000000000"), MAP_SLOT_SIZE, MAP_FILE_LENGTH)
    if vector_id == "MAPRV1-P-000":
        return record_file(header, MAP_SLOT_SIZE, {})
    if "P-STATE-" in vector_id:
        end_state = int(vector_id[-2:])
        slots: dict[int, bytes] = {}
        previous = ZERO32
        for state in range(1, end_state + 1):
            index = (state - 1) % 2
            header_digest, slot_digest, record_state = (ZERO32, ZERO32, 0) if state < 3 else linked_machine_actor(state)
            payload = map_payload(state, previous, header_digest, slot_digest)
            if state >= 3:
                payload_object = json.loads(payload, object_pairs_hook=OrderedDict)
                payload_object["machineActorRecordState"] = f"{record_state:04x}"
                payload = canonical_json(payload_object)
            slot = map_slot(index, state, payload)
            slots[index] = slot
            previous = digest(slot).hex()
        return record_file(header, MAP_SLOT_SIZE, slots)
    base = bytearray(record_file(header, MAP_SLOT_SIZE, {0: map_slot(0, 1, map_payload(1))}))
    number = int(vector_id.rsplit("-", 1)[1])
    if number == 1:
        return bytes(base)
    if number in (2, 3, 4, 5, 7, 8):
        base = bytearray(record_file(header, MAP_SLOT_SIZE, {0: map_negative_semantic_slot(number)}))
    elif number == 6:
        base[MAP_HEADER_SIZE + 21] = 0x7F
    elif number == 9:
        base[MAP_HEADER_SIZE + 192] ^= 0x01
    if number == 6:
        refresh_map_slot_integrity(base, MAP_HEADER_SIZE)
    return bytes(base)


def worker_bytes(vector_id: str) -> bytes:
    baseline_process = process_identity("worker", 3)
    baseline = OrderedDict((
        ("instanceId", hex_id("worker-instance")),
        ("processIdentity", baseline_process),
        ("role", "0003"),
        ("operationKind", "0001"),
        ("operationNonce", hex_id("operation")),
        ("leaseVersion", "0000000000000002"),
        ("oldProcessSignaled", "true"),
        ("frameSequence", ["HELLO", "GO", "TERMINAL"]),
    ))
    received = copy.deepcopy(baseline)
    cases: dict[str, tuple[str, str]] = {
        "WOSV1-N-001": ("same-process-different-instance", "WORKER_INSTANCE_ROTATE_REJECTED"),
        "WOSV1-N-002": ("same-instance-different-role", "WORKER_ROLE_MISMATCH"),
        "WOSV1-N-003": ("same-instance-different-operation", "WORKER_OPERATION_MISMATCH"),
        "WOSV1-N-004": ("same-instance-different-nonce", "WORKER_NONCE_MISMATCH"),
        "WOSV1-N-005": ("pid-reuse-different-creation-time", "WORKER_PROCESS_IDENTITY_MISMATCH"),
        "WOSV1-N-006": ("old-process-not-signaled", "OLD_WORKER_NOT_QUIESCENT"),
        "WOSV1-N-007": ("old-lease", "WORKER_LEASE_MISMATCH"),
        "WOSV1-N-008": ("go-replay", "WORKER_GO_REPLAY_REJECTED"),
        "WOSV1-N-009": ("terminal-after-frame", "WORKER_TERMINAL_AFTER_FRAME_REJECTED"),
        "WOSV1-N-010": ("same-worker-different-pid", "WORKER_PROCESS_IDENTITY_MISMATCH"),
        "WOSV1-N-011": ("same-worker-different-signed-image", "WORKER_PROCESS_IDENTITY_MISMATCH"),
        "WOSV1-N-012": ("same-worker-different-process-nonce", "WORKER_PROCESS_IDENTITY_MISMATCH"),
    }
    case, reject_reason = cases[vector_id]
    if vector_id == "WOSV1-N-001":
        received["instanceId"] = hex_id("worker-instance-rotated")
    elif vector_id == "WOSV1-N-002":
        received["role"] = "0002"
        received["processIdentity"] = process_identity("worker", 2)
    elif vector_id == "WOSV1-N-003":
        received["operationKind"] = "0002"
    elif vector_id == "WOSV1-N-004":
        received["operationNonce"] = hex_id("operation-other")
    elif vector_id == "WOSV1-N-005":
        reused = process_identity("worker", 3)
        reused["processCreationTime"] = "01d0000000000001"
        received["processIdentity"] = reused
    elif vector_id == "WOSV1-N-006":
        received["oldProcessSignaled"] = "false"
    elif vector_id == "WOSV1-N-007":
        received["leaseVersion"] = "0000000000000001"
    elif vector_id == "WOSV1-N-008":
        received["frameSequence"] = ["HELLO", "GO", "GO", "TERMINAL"]
    elif vector_id == "WOSV1-N-009":
        received["frameSequence"] = ["HELLO", "GO", "TERMINAL", "AFTER_TERMINAL"]
    elif vector_id == "WOSV1-N-010":
        received["processIdentity"]["pid"] = "00001001"
    elif vector_id == "WOSV1-N-011":
        received["processIdentity"]["signedImageIdentity"] = hex_digest("wrong-image")
    else:
        received["processIdentity"]["processNonce"] = hex_id("wrong-process-nonce")
    descriptor = OrderedDict((
        ("schema", "0001"), ("oracle", "WORKER_ONESHOT"), ("vectorId", vector_id), ("expected", "REJECT"),
        ("inputCase", case), ("rejectReason", reject_reason),
        ("baseline", baseline), ("received", received),
        ("sideEffects", OrderedDict((("displayMutation", 0), ("processLaunch", 0), ("fileCreate", 0), ("fileDelete", 0), ("fileTruncate", 0), ("fileWrite", 0)))),
    ))
    return canonical_json(descriptor)


def vector_catalog() -> list[str]:
    ids = [f"DJV1-P-{index:03d}" for index in range(1, 5)] + [f"DJV1-N-{index:03d}" for index in range(1, 14)]
    ids += [f"MARV1-P-STATE-{index:02X}" for index in range(13)]
    ids += [f"MARV1-P-FC-{index:02d}" for index in range(1, 4)] + ["MARV1-P-PROVISIONED-CLEAN"]
    for category, count in (("GROUP", 3), ("JSON", 10), ("ENUM", 4), ("FENCE", 4), ("PROVISION", 4), ("BIND", 12), ("D02", 3), ("STRUCT", 3)):
        ids += [f"MARV1-N-{category}-{index:02d}" for index in range(1, count + 1)]
    ids += ["MAPRV1-P-000"] + [f"MAPRV1-P-STATE-{index:02d}" for index in range(1, 7)]
    ids += [f"MAPRV1-N-{index:03d}" for index in range(1, 10)] + ["BOOTIDV1-P-001"]
    ids += [f"D02V1-P-{index:02X}" for index in range(1, 11)]
    ids += [f"D02V1-N-{index:03d}" for index in range(1, 4)]
    ids += [binding[0] for binding in MAR_D02_BINDINGS.values()]
    ids += [f"OWNWALV1-P-{code:04X}" for code in OWNER_WAL_CODES]
    ids += [f"OWNWALV1-N-{name}" for name in ("RESERVED-GAP", "RESERVED-TERMINAL", "UNKNOWN")]
    ids += ["MAPSCNV1-P-CREATE-ABSENT", "MAPSCNV1-P-RESUME-01", "MAPSCNV1-P-RESUME-02", "MAPSCNV1-P-MAP03-INTENT", "MAPSCNV1-P-MAP04-ACTIVE"] + [f"MAPSCNV1-P-STATE-{index:02d}" for index in range(3, 7)] + ["MAPSCNV1-P-TERMINAL-READONLY"]
    ids += [f"MAPSCNV1-N-{index:03d}" for index in range(10, 27)]
    ids += bytewise_sorted(D04_ALLOWED_TUPLES)
    ids += list(D04_D02_EVIDENCE_IDS) + [D04_READINESS_ID]
    ids += list(D04_NEGATIVE_IDS)
    ids += [f"OWNERSIDV1-P-{index:03d}" for index in range(1, 4)] + [f"OWNERSIDV1-N-{index:03d}" for index in range(1, 8)]
    ids += [f"WOSV1-N-{index:03d}" for index in range(1, 13)]
    if len(ids) != 590 or len(set(ids)) != len(ids):
        raise AssertionError("candidate catalog must contain 590 distinct vectors")
    return ids


def family_for(vector_id: str) -> str:
    return vector_id.split("-", 1)[0]


def vector_bytes(vector_id: str) -> bytes:
    if vector_id == "DJV1-N-006":
        base = dj_bytes("DJV1-P-001")
        return canonical_json(OrderedDict((("schema", "DecisionJournalPostCreateCheckpointV1"), ("vectorId", vector_id), ("recordSha256", digest(base).hex()), ("checkpointPresent", False), ("classification", "REJECT_UNBOUND_FRESH_FILE"), ("actionCounts", OrderedDict((("fileWrite", 0), ("displayMutation", 0)))))))
    if vector_id == "MAPRV1-N-001":
        base = map_bytes("MAPRV1-P-STATE-01")
        return canonical_json(OrderedDict((("schema", "ProvisionPostCreateCheckpointV1"), ("vectorId", vector_id), ("recordSha256", digest(base).hex()), ("checkpointPresent", False), ("classification", "REJECT_PRECHECKPOINT_EXISTING_FILE"), ("actionCounts", OrderedDict((("resume", 0), ("delete", 0), ("recreate", 0), ("fileWrite", 0), ("displayMutation", 0)))))))
    if family_for(vector_id) in {"D02V1", "D02MARV1"}:
        return d02_vector_bytes(vector_id)
    if family_for(vector_id) == "OWNWALV1":
        return owner_wal_vector_bytes(vector_id)
    if family_for(vector_id) == "MAPSCNV1":
        return map_scenario_bytes(vector_id)
    if family_for(vector_id) == "D04TUPV1":
        return d04_tuple_bytes(vector_id) if vector_id in D04_ALLOWED_TUPLES else d04_negative_bytes(vector_id)
    if family_for(vector_id) == "D04D02V1":
        return d04_d02_bytes(vector_id)
    if vector_id == D04_READINESS_ID:
        return d04_readiness_bytes()
    if family_for(vector_id) == "BOOTIDV1":
        return bootid_bytes()
    if family_for(vector_id) == "OWNERSIDV1":
        return owner_sid_vector_bytes(vector_id)
    return {"DJV1": dj_bytes, "MARV1": mar_bytes, "MAPRV1": map_bytes}.get(family_for(vector_id), worker_bytes)(vector_id)


def linked_vectors_for(vector_id: str) -> list[str]:
    links: dict[str, list[str]] = {
        "DJV1-N-006": ["DJV1-P-001"],
        "MAPRV1-N-001": ["MAPRV1-P-STATE-01"],
        "OWNERSIDV1-P-001": ["DJV1-P-002", "MAPRV1-P-STATE-03", "MARV1-P-STATE-01"],
        "MAPRV1-P-STATE-03": ["MARV1-P-STATE-0A"],
        "MAPRV1-P-STATE-04": ["MARV1-P-STATE-0A", "MARV1-P-STATE-0B"],
        "MAPRV1-P-STATE-05": ["MARV1-P-STATE-0A", "MARV1-P-STATE-0B", "MARV1-P-PROVISIONED-CLEAN"],
        "MAPRV1-P-STATE-06": ["MARV1-P-STATE-0A", "MARV1-P-STATE-0B", "MARV1-P-PROVISIONED-CLEAN"],
    }
    if vector_id.startswith("MAPSCNV1-"):
        config = MAPSCN_POSITIVE.get(vector_id, MAPSCN_POSITIVE["MAPSCNV1-P-RESUME-02"])
        stored_state, current_state, candidate_state, mar_id, *_ = config
        if vector_id == "MAPSCNV1-N-023": stored_state, current_state, candidate_state = 1, 2, 4
        if vector_id in {"MAPSCNV1-N-024", "MAPSCNV1-N-025", "MAPSCNV1-N-026"}: stored_state, current_state, candidate_state = 1, 1, 0
        links = {f"MAPRV1-P-STATE-{state:02d}" for state in (stored_state, current_state, candidate_state) if state}
        if vector_id != "MAPSCNV1-P-CREATE-ABSENT":
            links.add("MAPRV1-P-STATE-02")
        if vector_id == "MAPSCNV1-N-022": links.add("MARV1-P-FC-01")
        elif mar_id is not None: links.add(mar_id)
        return bytewise_sorted(links)
    if vector_id in D04_ALLOWED_TUPLES:
        kind, result, owner, _state, variant = D04_ALLOWED_TUPLES[vector_id]
        links = {f"OWNWALV1-P-{owner:04X}"}
        links.add(d04_machine_actor_fixture(_state, variant))
        if variant == "TERMINAL_CLEAN":
            links.add(d04_tuple_id(kind, result, owner, 8, "TERMINALIZING"))
        if _state == 12:
            links.add(d04_evidence_id(vector_id))
        if variant == "PROVISION_STRUCTURAL":
            links.update(("MAPRV1-P-STATE-05", "MARV1-P-PROVISIONED-CLEAN"))
        return bytewise_sorted(links)
    if vector_id in D04_D02_EVIDENCE_IDS:
        bound_id, _kind, _result, owner = D04_EVIDENCE_BINDINGS[vector_id]
        evidence_links = {bound_id, f"OWNWALV1-P-{owner:04X}"}
        if bound_id in D04_ALLOWED_TUPLES:
            _kind, _result, _owner, state, variant = D04_ALLOWED_TUPLES[bound_id]
            evidence_links.add(d04_machine_actor_fixture(state, variant))
        return bytewise_sorted(evidence_links)
    if vector_id == D04_READINESS_ID:
        return bytewise_sorted((d04_tuple_id(7, 1, 0, 9, "PROVISION_STRUCTURAL"), "MAPRV1-P-STATE-05", "MAPRV1-P-STATE-06", "MARV1-P-PROVISIONED-CLEAN"))
    if vector_id.startswith("D04TUPV1-N-"):
        if vector_id in D04_NEGATIVE_EVIDENCE_BOUNDARIES:
            _kind, _result, owner = D04_NEGATIVE_EVIDENCE_BOUNDARIES[vector_id]
            links = {d04_negative_evidence_id(vector_id), f"OWNWALV1-P-{owner:04X}", "MARV1-P-FC-01"}
        elif vector_id == "D04TUPV1-N-UNCERTAINTY-EVIDENCE-MISSING":
            links = set()
            links.update((d04_tuple_id(1, 6, 0x0002, 12, "UNCERTAINTY"), d04_evidence_id(d04_tuple_id(1, 6, 0x0002, 12, "UNCERTAINTY")), "OWNWALV1-P-0002"))
        elif vector_id.startswith("D04TUPV1-N-PROVISION"):
            links = set()
            links.update((d04_tuple_id(7, 1, 0, 9, "PROVISION_STRUCTURAL"), "MAPRV1-P-STATE-05", "MAPRV1-P-STATE-06", "MARV1-P-PROVISIONED-CLEAN"))
        elif vector_id == "D04TUPV1-N-UNINSTALL-NOT-ADMITTED":
            links = set()
            links.add("OWNWALV1-P-0020")
        elif vector_id == "D04TUPV1-N-NORMAL-PAIR-TUPLE-CHANGE":
            links = {
                d04_tuple_id(1, 3, 0x21, 9, "TERMINAL_CLEAN"),
                d04_tuple_id(1, 4, 0x22, 8, "TERMINALIZING"),
            }
        else:
            links = {d04_tuple_id(1, 3, 0x21, 9, "TERMINAL_CLEAN")}
        return bytewise_sorted(links)
    if vector_id in MAR_D02_BINDINGS:
        return [MAR_D02_BINDINGS[vector_id][0]]
    for mar_id, binding in MAR_D02_BINDINGS.items():
        if binding[0] == vector_id:
            return [mar_id]
    return bytewise_sorted(links.get(vector_id, []))


def exact_parse_rule(vector_id: str) -> str:
    """Per-vector oracle contract; never a placeholder for a product parser."""
    family = family_for(vector_id)
    if vector_id.startswith("DJV1-P-"):
        if vector_id == "DJV1-P-001":
            return "UNQUALIFIED_RAW_FRESH: exact zero-slot DJ bytes require separately bound durable post-create checkpoint; bytes alone grant no authority."
        return f"ACCEPT {vector_id}: exact DJV1 header, two fixed slots, owner-SID digest and slot/header SHA-256; classify this named chain only."
    if vector_id.startswith("DJV1-N-"):
        return f"REJECT: {DJ_NEGATIVE_CASES[vector_id][0]}; no decision authority and no mutation."
    if vector_id.startswith("MARV1-P-"):
        return f"ACCEPT {vector_id}: exact MARV1 header/slot integrity, canonical state payload and D01/D02/D03/D04 bindings; inspection only."
    if vector_id.startswith("MARV1-N-"):
        return f"REJECT: {MAR_NEGATIVE_CASES[vector_id][0]}; no recovery or mutation authority."
    if vector_id.startswith("D02V1-P-"):
        return f"ACCEPT: exact D02 known-answer detail={vector_id[-2:].lower()}, inner source/domain and outer digest all rehash exactly; no authority grant."
    if vector_id.startswith("D02MARV1-P-"):
        return f"ACCEPT {vector_id}: exact context-bound D02 source/preimage/digest; bound MAR E-group must match byte-for-byte."
    if vector_id.startswith("D02V1-N-"):
        return f"REJECT {vector_id}: exact D02 inner-source/outer-digest boundary; all action counters zero."
    if vector_id in D04_ALLOWED_TUPLES:
        kind, result, owner, state, variant = D04_ALLOWED_TUPLES[vector_id]
        return (
            f"ACCEPT {vector_id}: approved D04 R01-R04 semantic tuple witness "
            f"K={kind:04x} R={result:04x} W={owner:04x} S={state:04x} V={variant}; "
            "tupleProjection is the tuple-specific source; linked MAR is envelope/state layout only "
            "and never supplies P/Q/nonce/owner; evidence only."
        )
    if vector_id.startswith("D04TUPV1-N-"):
        return f"REJECT {vector_id}: exact prohibited D04 R01-R04 binding; no authority or mutation."
    if vector_id in D04_D02_EVIDENCE_IDS:
        bound_id = D04_EVIDENCE_BINDINGS[vector_id][0]
        return f"ACCEPT {vector_id}: exact context-bound D02 source, preimage, and digest for {bound_id}; evidence only."
    if vector_id == D04_READINESS_ID:
        return "ACCEPT D04 readiness companion: exact structural state-5 witness plus MAP state-6 durable package evidence; runtime authorization remains false."
    if vector_id.startswith("OWNWALV1-P-"):
        return f"ACCEPT: exact OwnerWalLinkStateV1 code={vector_id[-4:].lower()} two-byte LE round-trip with sentinel rule."
    if vector_id.startswith("OWNWALV1-N-"):
        return f"REJECT {vector_id}: reserved or unknown OwnerWalLinkStateV1 code; never coerce or encode a WAL frame."
    if vector_id.startswith("MAPSCNV1-"):
        return f"READ_ONLY {vector_id}: exact stored/observed fileId/DACL/anchor/attribute comparison and requested-action outcome; all action counters zero."
    if vector_id.startswith("MAPRV1-P-"):
        if vector_id == "MAPRV1-P-STATE-01":
            return "UNQUALIFIED_RAW_CREATE_INTENT: exact bytes require separately bound post-create checkpoint before any resume; no write authority."
        return f"ACCEPT {vector_id}: exact MAPRV1 fixed-slot/checkpoint predicate; never authorizes a write."
    if vector_id.startswith("MAPRV1-N-"):
        return f"REJECT {vector_id}: exact named MAP mismatch/action predicate; requested resume/delete/recreate/cleanup count remains zero."
    if vector_id.startswith("OWNERSIDV1-P-"):
        return f"ACCEPT {vector_id}: revision=1, NT authority=5, actualLength=8+4*subAuthorityCount, zero unused tail; digest actual bytes only."
    if vector_id.startswith("OWNERSIDV1-N-"):
        return f"REJECT {vector_id}: exact SidValueV1 boundary/preimage violation; do not derive owner authority."
    if vector_id.startswith("WOSV1-N-"):
        descriptor = json.loads(worker_bytes(vector_id).decode("ascii"))
        return f"REJECT {vector_id}: {descriptor['inputCase']} -> {descriptor['rejectReason']}; no process launch or display/file mutation."
    if vector_id == "BOOTIDV1-P-001":
        return "ACCEPT: exact static BootIdV1 domain-separated preimage digest only; no same-boot runtime authority."
    raise ValueError(vector_id)


def d02_parameters_for(vector_id: str) -> tuple[int, int, int, str] | None:
    table = {
        "MARV1-P-STATE-00": (1, 2, 1, "mar-0-normal"), "MARV1-P-STATE-06": (4, 4, 4, "mar-6-normal"),
        "MARV1-P-FC-01": (1, 2, 1, "mar-12-fc-01"), "MARV1-P-FC-02": (8, 7, 6, "mar-12-fc-02"), "MARV1-P-FC-03": (1, 3, 3, "mar-12-fc-03"),
        "MARV1-N-D02-01": (8, 1, 6, "standalone"), "MARV1-N-D02-02": (1, 7, 6, "standalone"), "MARV1-N-D02-03": (1, 2, 1, "standalone"),
    }
    return table.get(vector_id)


def canonicalization_rule_for(vector_id: str) -> str:
    base = "ASCII UTF-8 canonical JSON; unsigned little-endian binary envelope; canonical key order; exact declared SHA-256 coverage"
    if vector_id.startswith("D04TUPV1-") or vector_id == D04_READINESS_ID:
        return (
            "ASCII UTF-8 canonical JSON descriptor; canonical key order; "
            "embedded tupleProjection is the tuple-specific preimage and its SHA-256 must rehash exactly; "
            "referenced D04/MAR/MAP/OwnerWal/D02 binary or descriptor artifacts are bound by exact SHA-256 "
            "and artifact-index links, not embedded as a binary envelope; linked MAR is state/envelope/checksum layout only"
        )
    if vector_id.startswith("MAPSCNV1-"):
        return (
            "ASCII UTF-8 canonical JSON descriptor; canonical key order; "
            "referenced MAPRV1/MARV1 binary artifacts are bound by exact SHA-256 and artifact-index links, "
            "not embedded as a binary envelope"
        )
    if vector_id.startswith("D02V1-P-"):
        detail = int(vector_id[-2:], 16)
        error_class, evidence_kind = DETAIL_MATRIX[detail]
        inputs = critical_evidence_inputs(evidence_kind, error_class, detail)
        return base + "; D02 exact subjectDomain=" + inputs["subjectDomainSeparatorHex"] + "; subjectSource=" + inputs["subjectSourceHex"] + "; observedDomain=" + inputs["observedDomainSeparatorHex"] + "; observedSource=" + inputs["observedSourceHex"] + "; outerDomain=" + inputs["domainSeparatorHex"]
    if vector_id.startswith("D02MARV1-P-"):
        mar_id = next(mar for mar, binding in MAR_D02_BINDINGS.items() if binding[0] == vector_id)
        _, evidence_kind, error_class, detail, context = MAR_D02_BINDINGS[mar_id]
        inputs = critical_evidence_inputs(evidence_kind, error_class, detail, context)
        return base + "; D02 exact subjectDomain=" + inputs["subjectDomainSeparatorHex"] + "; subjectSource=" + inputs["subjectSourceHex"] + "; observedDomain=" + inputs["observedDomainSeparatorHex"] + "; observedSource=" + inputs["observedSourceHex"] + "; outerDomain=" + inputs["domainSeparatorHex"]
    if vector_id in D04_D02_EVIDENCE_IDS:
        bound_id, _kind, result, _owner = D04_EVIDENCE_BINDINGS[vector_id]
        evidence_kind, error_class, detail = d04_evidence_parameters(result)
        inputs = critical_evidence_inputs(evidence_kind, error_class, detail, f"d04-{bound_id}")
        return (
            "ASCII UTF-8 canonical JSON descriptor; canonical key order; "
            f"boundWitness={bound_id}; D02 exact subjectDomain={inputs['subjectDomainSeparatorHex']}; "
            f"subjectSource={inputs['subjectSourceHex']}; observedDomain={inputs['observedDomainSeparatorHex']}; "
            f"observedSource={inputs['observedSourceHex']}; outerDomain={inputs['domainSeparatorHex']}"
        )
    params = d02_parameters_for(vector_id)
    if params is not None:
        evidence_kind, error_class, detail, context = params
        inputs = critical_evidence_inputs(evidence_kind, error_class, detail, context)
        return base + "; D02 exact subjectDomain=" + inputs["subjectDomainSeparatorHex"] + "; subjectSource=" + inputs["subjectSourceHex"] + "; observedDomain=" + inputs["observedDomainSeparatorHex"] + "; observedSource=" + inputs["observedSourceHex"] + "; outerDomain=" + inputs["domainSeparatorHex"]
    return base


def manifest_for(vector_id: str, fixture: bytes) -> OrderedDict[str, Any]:
    positive, family = "-P-" in vector_id, family_for(vector_id)
    classification = "CANDIDATE_ACCEPT" if positive else "REJECT_FAIL_CLOSED"
    if vector_id in ("DJV1-P-001", "MAPRV1-P-000"):
        classification = "FRESH_UNINITIALIZED"
    if vector_id == "BOOTIDV1-P-001":
        classification = "STATIC_BOOT_ID_DIGEST_MATCH"
    if vector_id.startswith("D02V1-P-"):
        classification = "CRITICAL_EVIDENCE_EXACT_MATCH"
    if vector_id.startswith("D02MARV1-P-"):
        classification = "CRITICAL_EVIDENCE_EXACT_MATCH"
    if vector_id.startswith("D02V1-N-"):
        classification = "CRITICAL_EVIDENCE_REJECT"
    if vector_id.startswith("D04TUPV1-"):
        classification = "D04_EXACT_ALLOWED_TUPLE" if vector_id in D04_ALLOWED_TUPLES else "D04_REJECT_FAIL_CLOSED"
    if vector_id in D04_D02_EVIDENCE_IDS:
        classification = "D04_CRITICAL_EVIDENCE_EXACT"
    if vector_id == D04_READINESS_ID:
        classification = "D04_R04_READINESS_EVIDENCE_ONLY"
    if vector_id == "OWNWALV1-P-0000":
        classification = "ABSENT_EXPECTED_MACHINE_LINK_ONLY"
    if vector_id.startswith("OWNWALV1-P-") and vector_id != "OWNWALV1-P-0000":
        classification = "OWNER_WAL_CODE_ROUND_TRIP"
    if vector_id.startswith("MAPSCNV1-P-"):
        classification = "READ_ONLY_RESUME_CLASSIFICATION"
    if vector_id.startswith("MAPSCNV1-N-"):
        classification = "READ_ONLY_ACTION_REJECT"
    if family == "WOSV1":
        classification = "REJECT_NO_WORKER_REUSE"
    side_effects = OrderedDict((("displayMutation", 0), ("processLaunch", 0), ("fileCreate", 0), ("fileDelete", 0), ("fileTruncate", 0), ("fileWrite", 0)))
    if vector_id == "DJV1-P-001":
        profile, mutation = "fresh header plus zero slots; external DECISION_JOURNAL_POST_CREATE_CHECKPOINT prerequisite is exact and not encoded in this file", "none"
    elif vector_id in DJ_NEGATIVE_CASES:
        profile, mutation = DJ_NEGATIVE_CASES[vector_id]
    elif vector_id == "MAPRV1-N-001":
        profile, mutation = "valid fresh provision-record bytes with required external POST_CREATE_CHECKPOINT prerequisite deliberately absent", "external prerequisite missing; no byte mutation"
    elif vector_id.startswith("MAPRV1-N-"):
        map_cases = {
            "MAPRV1-N-002": "MachineActor identity mismatch",
            "MAPRV1-N-003": "DACL profile mismatch",
            "MAPRV1-N-004": "directory anchor mismatch",
            "MAPRV1-N-005": "attribute profile mismatch",
            "MAPRV1-N-006": "unknown provision state",
            "MAPRV1-N-007": "premature resume/delete/recreate",
            "MAPRV1-N-008": "FAILED_CLOSED cleanup attempt",
            "MAPRV1-N-009": "checksum mismatch dedicated vector",
        }
        profile, mutation = map_cases[vector_id], map_cases[vector_id]
    elif vector_id == "BOOTIDV1-P-001":
        profile, mutation = "static BootIdV1 preimage tuple plus expected SHA-256; no runtime tick or tolerance authority", "none"
    elif vector_id == "OWNERSIDV1-P-001":
        profile, mutation = "actual canonical SID bytes with u32le actual length; exact digest cross-link to DJV1-P-002, MARV1-P-STATE-01, MAPRV1-P-STATE-03, and lock-name input", "none"
    elif family == "WOSV1":
        descriptor = json.loads(worker_bytes(vector_id).decode("ascii"))
        profile, mutation = f"{descriptor['inputCase']}; concrete baseline/received identity and frame data are in fixture", descriptor["rejectReason"]
    elif vector_id.startswith("MARV1-P-FC-") or vector_id in {"MARV1-P-STATE-00", "MARV1-P-STATE-06"}:
        state = 12 if vector_id.startswith("MARV1-P-FC-") else int(vector_id[-2:], 16)
        variant = "fc-" + vector_id[-2:] if vector_id.startswith("MARV1-P-FC-") else "normal"
        payload = json.loads(machine_payload(state, variant).decode("ascii"), object_pairs_hook=OrderedDict)
        detail = int(payload["lastErrorDetailCode"], 16)
        evidence_kind = {1: 1, 3: 1, 4: 4, 6: 8}[detail]
        context = d02_parameters_for(vector_id)[3] if d02_parameters_for(vector_id) else "standalone"
        profile = f"{vector_id} critical evidence inputs={canonical_json(critical_evidence_inputs(evidence_kind, int(payload['lastErrorClass'],16), detail, context)).decode('ascii')}"
        mutation = "none"
    elif vector_id in D04_D02_EVIDENCE_IDS:
        bound_id = D04_EVIDENCE_BINDINGS[vector_id][0]
        profile = f"{vector_id} context-bound critical evidence for {bound_id}; exact canonical inputs are embedded in the fixture and canonicalizationRule"
        mutation = "none"
    elif vector_id in MAR_NEGATIVE_CASES:
        profile, mutation, integrity = MAR_NEGATIVE_CASES[vector_id]
        profile = f"{profile}; integrity={integrity}; semantic base is the state-specific canonical payload named by this vector"
        mutation = f"{mutation}; {integrity}"
    elif positive:
        profile, mutation = f"{vector_id} canonical positive candidate profile", "none"
    else:
        profile, mutation = f"{vector_id} isolated negative candidate profile", f"{vector_id} single exact negative mutation; integrity recomputed unless this is a checksum/length/trailing vector"
    return OrderedDict((
        ("vectorId", vector_id), ("family", family), ("positiveOrNegative", "positive" if positive else "negative"),
        ("requiredDecisionSet", ["DD-FR-002-D01", "DD-FR-002-D02", "DD-FR-002-D03", "DD-FR-002-D04", "DD-FR-002-D05", "DD-FR-002-D06", "DD-FR-002-D07", "DD-FR-002-D08"]),
        ("semanticInputProfile", profile),
        ("canonicalizationRule", canonicalization_rule_for(vector_id)),
        ("mutationDescriptor", mutation),
        ("expectedParse", exact_parse_rule(vector_id)),
        ("expectedClassification", classification),
        ("expectedRecoveryDisposition", "READ_ONLY_CANDIDATE_OBSERVATION" if positive else "NO_MUTATION_FAIL_CLOSED"),
        ("expectedSideEffects", side_effects), ("byteFixtureStatus", "FULL_BYTES_CREATED"), ("sha256Status", "SHA256_CREATED"),
    ))


def static_d08_known_answer() -> OrderedDict[str, Any]:
    utc, major, minor, build = 133_700_000_000_000_000, 10, 0, 26_190
    preimage = b"DisplayDeck.BootId.V1\0" + utc.to_bytes(8, "little") + major.to_bytes(4, "little") + minor.to_bytes(4, "little") + build.to_bytes(4, "little")
    return OrderedDict((
        ("artifact", "BootIdV1-static-known-answer"), ("scope", "STATIC_PREIMAGE_ONLY"),
        ("lastBootUtcFileTime", f"{utc:016x}"), ("versionMajor", f"{major:08x}"), ("versionMinor", f"{minor:08x}"), ("versionBuild", f"{build:08x}"),
        ("bootIdSha256", digest(preimage).hex()), ("runtimeAcceptance", "UNSET_EVIDENCE_PENDING"),
        ("excluded", ["tick samples", "UTC sample span", "host name", "wall-clock capture time", "WMI timeout", "tolerance"]),
    ))


def bootid_bytes() -> bytes:
    utc, major, minor, build = 133_700_000_000_000_000, 10, 0, 26_190
    preimage = b"DisplayDeck.BootId.V1\0" + utc.to_bytes(8, "little") + major.to_bytes(4, "little") + minor.to_bytes(4, "little") + build.to_bytes(4, "little")
    return canonical_json(OrderedDict((
        ("schema", "BootIdV1-static-known-answer"),
        ("scope", "STATIC_PREIMAGE_ONLY"),
        ("lastBootUtcFileTime", f"{utc:016x}"),
        ("versionMajor", f"{major:08x}"),
        ("versionMinor", f"{minor:08x}"),
        ("versionBuild", f"{build:08x}"),
        ("expectedBootIdSha256", digest(preimage).hex()),
        ("runtimeAcceptance", "UNSET_EVIDENCE_PENDING"),
    )))


def bytewise_sorted(values: Iterable[str]) -> list[str]:
    return sorted(values, key=lambda value: value.encode("utf-8"))


def aggregate_fixture_set_sha256(semantic_sha256: str, entries: list[OrderedDict[str, Any]]) -> str:
    material = bytearray(b"DisplayDeck.DD-FR-002.ArtifactSet.V1\0")
    material.extend(PROFILE.encode("utf-8"))
    material.extend(bytes.fromhex(semantic_sha256))
    for entry in entries:
        vector_id = entry["vectorId"].encode("utf-8")
        relative_path = entry["relativePath"].encode("utf-8")
        material.extend(len(vector_id).to_bytes(4, "little"))
        material.extend(vector_id)
        material.extend(len(relative_path).to_bytes(4, "little"))
        material.extend(relative_path)
        material.extend(int(entry["byteLength"], 16).to_bytes(8, "little"))
        material.extend(bytes.fromhex(entry["fixtureSha256"]))
        links = entry["linkedVectorIds"]
        hashes = entry["linkedFixtureSha256s"]
        material.extend(len(links).to_bytes(4, "little"))
        for linked_id, linked_hash in zip(links, hashes, strict=True):
            encoded_link = linked_id.encode("utf-8")
            material.extend(len(encoded_link).to_bytes(4, "little"))
            material.extend(encoded_link)
            material.extend(bytes.fromhex(linked_hash))
    return digest(bytes(material)).hex()


def expected_semantic_manifest() -> bytes:
    return b"".join(canonical_json(manifest_for(vector_id, vector_bytes(vector_id))) + b"\n" for vector_id in bytewise_sorted(vector_catalog()))


def expected_entries(output: Path) -> list[OrderedDict[str, Any]]:
    rows: list[OrderedDict[str, Any]] = []
    for vector_id in bytewise_sorted(vector_catalog()):
        contents = (output / "bytes" / f"{vector_id}.bin").read_bytes()
        linked = linked_vectors_for(vector_id)
        rows.append(OrderedDict((
            ("vectorId", vector_id),
            ("relativePath", f"bytes/{vector_id}.bin"),
            ("byteLength", f"{len(contents):016x}"),
            ("fixtureSha256", digest(contents).hex()),
            ("linkedVectorIds", linked),
            ("linkedFixtureSha256s", [digest((output / "bytes" / f"{linked_id}.bin").read_bytes()).hex() for linked_id in linked]),
        )))
    return rows


def all_artifact_files(root: Path) -> list[tuple[str, bytes]]:
    return [
        (path.relative_to(root).as_posix(), path.read_bytes())
        for path in sorted(root.rglob("*"))
        if path.is_file() and path.name != "SHA256SUMS"
    ]


def check_header(data: bytes, magic_hex: str, slot_size: int, file_length: int) -> bool:
    if len(data) != file_length or data[0:16] != bytes.fromhex(magic_hex):
        return False
    if int.from_bytes(data[16:18], "little") != 1 or int.from_bytes(data[18:20], "little") != 1:
        return False
    if int.from_bytes(data[20:24], "little") != DJ_HEADER_SIZE or int.from_bytes(data[24:28], "little") != slot_size:
        return False
    if int.from_bytes(data[28:32], "little") != file_length or data[32] != 2 or any(data[33:64]) or any(data[96:DJ_HEADER_SIZE]):
        return False
    covered = bytearray(data[:DJ_HEADER_SIZE])
    covered[64:96] = b"\0" * 32
    return data[64:96] == digest(bytes(covered))


def check_dj_slot(slot: bytes, slot_index: int) -> bool:
    if not any(slot):
        return True
    if slot[0:16] != bytes.fromhex("444a534c4f5456310000000000000000") or slot[20] != slot_index:
        return False
    if int.from_bytes(slot[16:18], "little") != 1 or int.from_bytes(slot[18:20], "little") != 1 or int.from_bytes(slot[24:28], "little") != 440:
        return False
    if any(slot[22:24]) or any(slot[28:32]) or any(slot[440:]):
        return False
    covered = bytearray(slot[:440])
    covered[408:440] = b"\0" * 32
    return slot[176:208] == bytes.fromhex(owner_sid_digest()) and slot[376:408] == digest(slot[64:376]) and slot[408:440] == digest(bytes(covered))


def check_dj_negative(vector_id: str, data: bytes) -> bool:
    """Independent structural predicate for each DJ negative boundary."""
    if vector_id == "DJV1-N-001":
        return len(data) == DJ_FILE_LENGTH - 1
    if vector_id == "DJV1-N-006":
        try:
            parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
            return parsed["recordSha256"] == digest(dj_bytes("DJV1-P-001")).hex() and parsed["checkpointPresent"] is False and parsed["classification"] == "REJECT_UNBOUND_FRESH_FILE" and all(value == 0 for value in parsed["actionCounts"].values())
        except (UnicodeDecodeError, json.JSONDecodeError, KeyError):
            return False
    if len(data) != DJ_FILE_LENGTH:
        return False
    header_ok = check_header(data, "4444444a563100000000000000000000", DJ_SLOT_SIZE, DJ_FILE_LENGTH)
    slot = data[DJ_HEADER_SIZE : DJ_HEADER_SIZE + DJ_SLOT_SIZE]
    if vector_id == "DJV1-N-002":
        return not header_ok and slot == dj_slot(0, 1)
    if not header_ok:
        return False
    covered = bytearray(slot[:440])
    covered[408:440] = b"\0" * 32
    integrity_ok = slot[376:408] == digest(slot[64:376]) and slot[408:440] == digest(bytes(covered))
    if vector_id == "DJV1-N-003":
        return integrity_ok and slot[20] == 1
    if vector_id == "DJV1-N-004":
        return integrity_ok and int.from_bytes(slot[32:40], "little") == 1 and int.from_bytes(slot[40:48], "little") == 1
    if vector_id == "DJV1-N-005":
        slot_b = data[DJ_HEADER_SIZE + DJ_SLOT_SIZE : DJ_HEADER_SIZE + 2 * DJ_SLOT_SIZE]
        covered_b = bytearray(slot_b[:440]); covered_b[408:440] = b"\0" * 32
        return integrity_ok and slot_b[176:208] != slot[176:208] and slot_b[376:408] == digest(slot_b[64:376]) and slot_b[408:440] == digest(bytes(covered_b))
    if vector_id == "DJV1-N-006":
        return not any(slot) and not any(data[DJ_HEADER_SIZE + DJ_SLOT_SIZE :])
    if vector_id == "DJV1-N-007":
        return integrity_ok and slot[21] == 0x7F
    if vector_id == "DJV1-N-008":
        return integrity_ok and slot[22] == 1
    if vector_id == "DJV1-N-009":
        return integrity_ok and slot[440] == 1
    if vector_id == "DJV1-N-010":
        return integrity_ok and data[DJ_HEADER_SIZE + DJ_SLOT_SIZE + 64 : DJ_HEADER_SIZE + DJ_SLOT_SIZE + 80] != slot[64:80]
    if vector_id == "DJV1-N-011":
        return integrity_ok and data[DJ_HEADER_SIZE + DJ_SLOT_SIZE + 80 : DJ_HEADER_SIZE + DJ_SLOT_SIZE + 112] != slot[80:112]
    if vector_id == "DJV1-N-012":
        return integrity_ok and data[DJ_HEADER_SIZE + DJ_SLOT_SIZE + 144 : DJ_HEADER_SIZE + DJ_SLOT_SIZE + 176] != slot[144:176]
    if vector_id == "DJV1-N-013":
        return integrity_ok and int.from_bytes(data[DJ_HEADER_SIZE + DJ_SLOT_SIZE + 56 : DJ_HEADER_SIZE + DJ_SLOT_SIZE + 64], "little") != int.from_bytes(slot[56:64], "little")
    return False


def mar_expected_keys(state: int, payload: OrderedDict[str, Any]) -> set[str]:
    groups: dict[int, tuple[str, ...]] = {
        0: ("H", "E"), 1: ("H", "O", "C", "W", "P"), 2: ("H", "O", "C", "W", "P"),
        3: ("H", "O", "C", "W", "P"), 4: ("H", "O", "C", "W", "X", "P"),
        5: ("H", "O", "C", "W", "P"), 6: ("H", "O", "W", "P", "E"),
        7: ("H", "O", "W", "P"), 8: ("H", "O", "W", "T", "P", "Q"),
        9: ("H", "O", "T", "P", "Q"), 10: ("H", "T", "P"), 11: ("H", "T", "P"), 12: ("H", "E"),
    }
    if state == 12 and "operationCompletion" in payload and "ownerTerminalDigest" in payload:
        groups[12] = ("H", "O", "T", "P", "Q", "E")
    elif state == 12 and "operationCompletion" in payload:
        groups[12] = ("H", "O", "P", "Q", "E")
    members = {
        "H": ("bootId", "binaryVersion", "recoveryBinaryVersion", "createdWallClock", "updatedWallClock"),
        "O": ("activeDisplayId", "ownerSid", "ownerLogonId", "ownerSessionId", "ownerWalPathDigest", "ownerWalGeneration", "ownerWalState"),
        "C": ("controllerInstanceId", "controllerProcessIdentity"), "W": ("watchdogInstanceId", "watchdogProcessIdentity"),
        "X": ("workerInstanceId", "workerProcessIdentity"), "T": ("ownerTerminalDigest", "terminalGeneration"),
        "P": ("operationKind", "operationNonce", "operationIntent"), "Q": ("operationCompletion",),
        "E": ("lastErrorClass", "lastErrorDetailCode", "preservedEvidenceDigest"),
    }
    return {field for group in groups[state] for field in members[group]}


def check_mar_slot(slot: bytes, slot_index: int, expected_state: int | None = None) -> bool:
    if not any(slot):
        return True
    if slot[0:16] != bytes.fromhex("4d4152534c4f54310000000000000000") or slot[20] != slot_index:
        return False
    if expected_state is not None and slot[21] != expected_state:
        return False
    length, payload_length = int.from_bytes(slot[24:28], "little"), int.from_bytes(slot[28:32], "little")
    if length != 136 + payload_length or not 1 <= payload_length <= 32768 or any(slot[length:]):
        return False
    payload = slot[136:length]
    covered = bytearray(slot[:length])
    covered[104:136] = b"\0" * 32
    if slot[72:104] != digest(payload) or slot[104:136] != digest(bytes(covered)):
        return False
    try:
        parsed = json.loads(payload.decode("ascii"), object_pairs_hook=OrderedDict)
    except (UnicodeDecodeError, json.JSONDecodeError):
        return False
    if list(parsed) != [key for key in MASTER_KEYS if key in parsed] or set(parsed) != mar_expected_keys(slot[21], parsed):
        return False
    if slot[21] in (0, 6, 12):
        try:
            error_class, detail = int(parsed["lastErrorClass"], 16), int(parsed["lastErrorDetailCode"], 16)
            evidence_kind = {1: 1, 2: 1, 3: 1, 4: 4, 6: 8}.get(detail)
        except (KeyError, ValueError):
            return False
        variant = "normal"
        if slot[21] == 12 and "operationCompletion" in parsed:
            variant = "fc-01" if parsed.get("ownerWalState") == "0028" else ("fc-02" if parsed.get("ownerWalState") == "0011" else "fc-03")
        elif slot[21] == 12 and detail == 3:
            variant = "fc-03"
        context = f"mar-{slot[21]}-{variant}"
        if evidence_kind is None or parsed.get("preservedEvidenceDigest") != critical_evidence_digest(evidence_kind, error_class, detail, context):
            return False
        inputs = critical_evidence_inputs(evidence_kind, error_class, detail, context)
        if digest(bytes.fromhex(inputs["subjectDomainSeparatorHex"]) + bytes.fromhex(inputs["subjectSourceHex"])).hex() != inputs["subjectIdentityDigest"]:
            return False
        if digest(bytes.fromhex(inputs["observedDomainSeparatorHex"]) + bytes.fromhex(inputs["observedSourceHex"])).hex() != inputs["observedEvidenceDigest"]:
            return False
        if digest(bytes.fromhex(inputs["outerPreimageHex"])).hex() != inputs["preservedEvidenceDigest"]:
            return False
    if "ownerSid" in parsed:
        try:
            actual_sid_bytes(parsed["ownerSid"])
        except (TypeError, ValueError):
            return False
    # P/Q/top-level exact binding.  This is an artifact oracle, not runtime
    # parsing: every positive P or Q record must prove the candidate contract.
    if "operationIntent" in parsed:
        p, q = parsed["operationIntent"], parsed.get("operationCompletion")
        if list(p)[:8] != ["schema", "kind", "operationNonce", "actor", "expectedRecordStateVersion", "targetDigest", "planDigest", "detailsDigest"]:
            return False
        if p.get("schema") != "0001" or p.get("kind") != parsed.get("operationKind") or p.get("operationNonce") != parsed.get("operationNonce"):
            return False
        if "watchdogInstanceId" in parsed:
            required_actor = OrderedDict((("instanceId", parsed["watchdogInstanceId"]), ("process", parsed["watchdogProcessIdentity"])))
            if p.get("actor") != required_actor:
                return False
        if q is not None:
            if list(q)[:5] != ["schema", "kind", "operationNonce", "result", "actor"]:
                return False
            if q.get("schema") != "0001" or q.get("kind") != parsed.get("operationKind") or q.get("operationNonce") != parsed.get("operationNonce"):
                return False
            if q.get("actor") != p.get("actor") or q.get("result") not in {f"{n:04x}" for n in range(1, 8)}:
                return False
            kind = int(parsed["operationKind"], 16)
            required_q = {1: ["completionEvidenceDigest"], 2: ["completionEvidenceDigest"], 3: ["binarySetDigest", "signatureEvidenceDigest", "recoveryReaderDigest", "retentionDigest"], 4: ["binarySetDigest", "signatureEvidenceDigest", "recoveryReaderDigest", "retentionDigest"], 5: ["binarySetDigest", "signatureEvidenceDigest", "recoveryReaderDigest", "retentionDigest"], 6: ["tombstoneDigest", "retentionDigest"], 7: ["provisionCheckpointDigest"]}[kind]
            if list(q)[5:] != required_q or any(q[key] == ZERO32 for key in required_q):
                return False
    if slot[21] == 1 and (parsed.get("ownerWalGeneration") != ZERO8 or parsed.get("ownerWalState") != "0000"):
        return False
    if slot[21] == 12 and "operationCompletion" in parsed:
        if "ownerTerminalDigest" in parsed:
            if parsed.get("ownerWalState") not in {"0024", "0025", "0026", "0027", "0028"} or parsed.get("ownerWalState") != "0028" or parsed["operationCompletion"].get("result") != "0007":
                return False
        elif parsed.get("ownerWalState") != "0011" or parsed.get("lastErrorClass") != "0007" or parsed.get("lastErrorDetailCode") != "0006" or parsed["operationCompletion"].get("result") != "0006":
            return False
    return True


def mar_slot_integrity(slot: bytes, slot_index: int) -> tuple[bool, bytes]:
    """Validate only envelope/integrity, intentionally not semantic acceptance."""
    if slot[0:16] != bytes.fromhex("4d4152534c4f54310000000000000000") or slot[20] != slot_index:
        return False, b""
    length, payload_length = int.from_bytes(slot[24:28], "little"), int.from_bytes(slot[28:32], "little")
    if length != 136 + payload_length or not 1 <= payload_length <= 32768 or any(slot[length:]):
        return False, b""
    payload = slot[136:length]
    covered = bytearray(slot[:length])
    covered[104:136] = b"\0" * 32
    return slot[72:104] == digest(payload) and slot[104:136] == digest(bytes(covered)), payload


def check_mar_negative(vector_id: str, data: bytes) -> bool:
    """Reject predicate checks the specified semantic violation without regenerating bytes."""
    if not check_header(data, "44444d41525631000000000000000000", MAR_SLOT_SIZE, MAR_FILE_LENGTH):
        return False
    slot_a = data[MAR_HEADER_SIZE : MAR_HEADER_SIZE + MAR_SLOT_SIZE]
    if vector_id == "MARV1-N-STRUCT-01":
        return int.from_bytes(slot_a[28:32], "little") + 136 != int.from_bytes(slot_a[24:28], "little")
    if vector_id == "MARV1-N-STRUCT-03":
        return slot_a[72:104] == digest(slot_a[136 : int.from_bytes(slot_a[24:28], "little")]) and slot_a[104:136] != digest(bytes(bytearray(slot_a[:104]) + b"\0" * 32 + bytearray(slot_a[136 : int.from_bytes(slot_a[24:28], "little")])) )
    ok, payload = mar_slot_integrity(slot_a, 0)
    if not ok:
        return False
    if vector_id == "MARV1-N-STRUCT-02":
        return payload[:1] == b"\x80"
    if vector_id == "MARV1-N-FENCE-04":
        slot_b = data[MAR_HEADER_SIZE + MAR_SLOT_SIZE : MAR_HEADER_SIZE + 2 * MAR_SLOT_SIZE]
        return slot_b == slot_a and slot_b[20] == 0
    if vector_id == "MARV1-N-ENUM-01":
        return slot_a[21] == 0x7F
    if vector_id == "MARV1-N-FENCE-01":
        return int.from_bytes(slot_a[48:56], "little") == 0
    if vector_id == "MARV1-N-FENCE-02":
        return int.from_bytes(slot_a[48:56], "little") == 1
    if vector_id == "MARV1-N-JSON-03":
        return b"\\u002e" in payload
    if vector_id == "MARV1-N-JSON-04":
        return payload.startswith(b"{ ")
    if vector_id == "MARV1-N-JSON-05":
        return payload.startswith(b'{"updatedWallClock"')
    if vector_id == "MARV1-N-JSON-06":
        return payload.count(b'"bootId"') == 2
    if vector_id == "MARV1-N-JSON-08":
        return payload.endswith(b" ")
    try:
        parsed = json.loads(payload.decode("ascii"), object_pairs_hook=OrderedDict)
    except (UnicodeDecodeError, json.JSONDecodeError):
        return False
    if vector_id == "MARV1-N-GROUP-01":
        return slot_a[21] == 1 and "ownerTerminalDigest" in parsed
    if vector_id == "MARV1-N-GROUP-02":
        return slot_a[21] == 4 and "workerInstanceId" in parsed and "workerProcessIdentity" not in parsed
    if vector_id == "MARV1-N-GROUP-03":
        return slot_a[21] == 6 and "lastErrorClass" in parsed and "preservedEvidenceDigest" not in parsed
    if vector_id == "MARV1-N-JSON-01":
        return parsed.get("ownerWalGeneration") is None
    if vector_id == "MARV1-N-JSON-02":
        return isinstance(parsed.get("ownerSessionId"), int)
    if vector_id == "MARV1-N-JSON-07":
        return parsed.get("unexpected") == "0000"
    if vector_id == "MARV1-N-JSON-09":
        return parsed.get("ownerSessionId") is True
    if vector_id == "MARV1-N-JSON-10":
        return parsed.get("ownerSessionId") == []
    if vector_id == "MARV1-N-ENUM-02":
        return parsed.get("ownerWalState") == "0015"
    if vector_id == "MARV1-N-ENUM-03":
        return parsed.get("operationCompletion", {}).get("result") == "ffff"
    if vector_id == "MARV1-N-ENUM-04":
        return parsed.get("lastErrorDetailCode") == "ffff"
    if vector_id == "MARV1-N-FENCE-03":
        return parsed.get("watchdogInstanceId") != hex_id("watchdog") and parsed.get("watchdogProcessIdentity") == process_identity("watchdog", 2)
    if vector_id == "MARV1-N-PROVISION-01":
        return slot_a[21] == 9 and parsed.get("bootId") == ZERO32 and int.from_bytes(slot_a[40:48], "little") == 0 and int.from_bytes(slot_a[48:56], "little") == 0
    if vector_id == "MARV1-N-PROVISION-02":
        return slot_a[21] == 9 and int.from_bytes(slot_a[40:48], "little") == 0 and parsed.get("bootId") != ZERO32 and parsed.get("operationKind") == "0007"
    if vector_id == "MARV1-N-PROVISION-03":
        completion_value = parsed.get("operationCompletion", {})
        return parsed.get("ownerTerminalDigest") is not None and "tombstoneDigest" in completion_value and "completionEvidenceDigest" not in completion_value
    if vector_id == "MARV1-N-PROVISION-04":
        return parsed.get("preservedEvidenceDigest") == ZERO32
    if vector_id == "MARV1-N-BIND-01":
        return "operationCompletion" in parsed and "operationIntent" not in parsed and "operationKind" not in parsed
    if vector_id == "MARV1-N-BIND-02":
        return "tombstoneDigest" in parsed.get("operationCompletion", {}) and "completionEvidenceDigest" not in parsed.get("operationCompletion", {})
    if vector_id == "MARV1-N-BIND-03":
        return parsed.get("operationIntent", {}).get("schema") == "0002"
    if vector_id == "MARV1-N-BIND-04":
        return parsed.get("operationCompletion", {}).get("kind") == "0002"
    if vector_id == "MARV1-N-BIND-05":
        return parsed.get("operationCompletion", {}).get("operationNonce") != parsed.get("operationNonce")
    if vector_id == "MARV1-N-BIND-06":
        return parsed.get("operationIntent", {}).get("operationNonce") != parsed.get("operationNonce")
    if vector_id == "MARV1-N-BIND-07":
        return parsed.get("operationCompletion", {}).get("actor") != parsed.get("operationIntent", {}).get("actor")
    if vector_id == "MARV1-N-BIND-08":
        return parsed.get("operationCompletion", {}).get("result") == "1"
    if vector_id == "MARV1-N-BIND-09":
        return parsed.get("operationCompletion", {}).get("completionEvidenceDigest") == ZERO32
    if vector_id == "MARV1-N-BIND-10":
        return parsed.get("operationCompletion", {}).get("schema") == "0002"
    if vector_id == "MARV1-N-BIND-11":
        return parsed.get("operationCompletion", {}).get("kind") == "000A"
    if vector_id == "MARV1-N-BIND-12":
        return parsed.get("operationCompletion", {}).get("operationNonce", "").isupper()
    if vector_id == "MARV1-N-D02-01":
        return slot_a[21] == 6 and parsed.get("lastErrorClass") == "0001" and parsed.get("lastErrorDetailCode") == "0006"
    if vector_id == "MARV1-N-D02-02":
        return slot_a[21] == 6 and parsed.get("preservedEvidenceDigest") == critical_evidence_digest(1, 7, 6)
    if vector_id == "MARV1-N-D02-03":
        return slot_a[21] == 1 and "preservedEvidenceDigest" in parsed
    return False


def check_map_slot(slot: bytes, slot_index: int, expected_state: int | None = None) -> tuple[bool, OrderedDict[str, Any] | None]:
    if not any(slot):
        return True, None
    if slot[0:16] != bytes.fromhex("4d415052534c4f543100000000000000") or slot[20] != slot_index:
        return False, None
    if expected_state is not None and slot[21] != expected_state:
        return False, None
    length, payload_length = int.from_bytes(slot[24:28], "little"), int.from_bytes(slot[28:32], "little")
    if length != 224 + payload_length or not 1 <= payload_length <= 3872 or any(slot[length:]):
        return False, None
    payload = slot[224:length]
    covered = bytearray(slot[:length])
    covered[192:224] = b"\0" * 32
    if slot[160:192] != digest(payload) or slot[192:224] != digest(bytes(covered)):
        return False, None
    try:
        parsed = json.loads(payload.decode("ascii"), object_pairs_hook=OrderedDict)
    except (UnicodeDecodeError, json.JSONDecodeError):
        return False, None
    state = slot[21]
    if list(parsed) != MAP_MASTER_KEYS:
        return False, None
    if int.from_bytes(slot[32:40], "little") != state or int.from_bytes(slot[48:56], "little") != state:
        return False, None
    if state == 1:
        if any(slot[80:104]) or int.from_bytes(slot[112:120], "little") != 0 or parsed["previousSlotDigest"] != ZERO32:
            return False, None
    elif state == 2:
        if not any(slot[88:104]) or int.from_bytes(slot[112:120], "little") != 0:
            return False, None
    elif 3 <= state <= 6:
        header_digest, slot_digest, record_state = linked_machine_actor(state)
        if parsed["machineActorHeaderDigest"] != header_digest or parsed["machineActorSlotDigest"] != slot_digest or parsed["machineActorRecordState"] != f"{record_state:04x}":
            return False, None
        if int.from_bytes(slot[112:120], "little") != MAR_FILE_LENGTH or int.from_bytes(slot[120:128], "little") != state - 2:
            return False, None
    if parsed["designatedOwnerSidDigest"] != owner_sid_digest():
        return False, None
    return True, parsed


def check_map_negative(vector_id: str, data: bytes) -> bool:
    """MAP negatives 002..008 deliberately retain valid checksums; 009 does not."""
    if vector_id == "MAPRV1-N-001":
        try:
            parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
            return parsed["recordSha256"] == digest(map_bytes("MAPRV1-P-STATE-01")).hex() and parsed["checkpointPresent"] is False and parsed["classification"] == "REJECT_PRECHECKPOINT_EXISTING_FILE" and all(value == 0 for value in parsed["actionCounts"].values())
        except (UnicodeDecodeError, json.JSONDecodeError, KeyError):
            return False
    if not check_header(data, "44444d41505256310000000000000000", MAP_SLOT_SIZE, MAP_FILE_LENGTH):
        return False
    slot = data[MAP_HEADER_SIZE : MAP_HEADER_SIZE + MAP_SLOT_SIZE]
    length = int.from_bytes(slot[24:28], "little")
    payload = slot[224:length]
    covered = bytearray(slot[:length])
    covered[192:224] = b"\0" * 32
    integrity = slot[160:192] == digest(payload) and slot[192:224] == digest(bytes(covered))
    if vector_id == "MAPRV1-N-009":
        return not integrity and slot[160:192] == digest(payload)
    if not integrity:
        return False
    try:
        parsed = json.loads(payload.decode("ascii"), object_pairs_hook=OrderedDict)
    except (UnicodeDecodeError, json.JSONDecodeError):
        return False
    if vector_id == "MAPRV1-N-001":
        return slot[21] == 1 and parsed["previousSlotDigest"] == ZERO32
    if vector_id == "MAPRV1-N-002":
        return parsed["machineActorPathDigest"] == "f" * 64
    if vector_id == "MAPRV1-N-003":
        return parsed["machineActorDaclDigest"] == "e" * 64
    if vector_id == "MAPRV1-N-004":
        return parsed["directoryAnchorDigest"] == "d" * 64
    if vector_id == "MAPRV1-N-005":
        return parsed["machineActorAttributeStreamDigest"] == "c" * 64
    if vector_id == "MAPRV1-N-006":
        return slot[21] == 0x7F
    if vector_id == "MAPRV1-N-007":
        return parsed["retentionMode"] == "0002"
    if vector_id == "MAPRV1-N-008":
        return slot[21] == 7 and parsed["failureClass"] == "0002"
    return False


def check_map_scenario(vector_id: str, data: bytes) -> bool:
    """Generator self-consistency and semantic oracle for bounded 19.8 evidence."""
    # This in-generator gate is intentionally byte-exact so a second,
    # unrelated discrepancy cannot hide behind the intended negative.  It is
    # not the independent review oracle; independent review recomputes the
    # referenced artifacts and 19.8 relation outside this generator.
    if data != map_scenario_bytes(vector_id):
        return False
    try:
        parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
        stored, observed, counts = parsed["stored"], parsed["observed"], parsed["actionCounts"]
    except (UnicodeDecodeError, json.JSONDecodeError, KeyError):
        return False
    fact_keys = ["volumeSerial", "fileId", "daclDigest", "anchorDigest", "attributeDigest"]
    if parsed.get("schema") != "MachineActorProvisionReadOnlyScenarioV2" or list(stored) != fact_keys or list(observed) != fact_keys:
        return False
    if any(value != 0 for value in counts.values()):
        return False
    if parsed.get("d07DirectoryAnchorEvidence") != "UNPROVEN" or parsed.get("runtimeWriteAuthorized") is not False:
        return False
    checkpoint = map_bytes("MAPRV1-P-STATE-02")
    if parsed.get("checkpointMapRecordSha256") != digest(checkpoint).hex() or parsed.get("checkpointLinkState") != "0002":
        return False
    relaxed_context = {"MAPSCNV1-N-017", "MAPSCNV1-N-018", "MAPSCNV1-N-019"}
    if vector_id not in relaxed_context and (parsed.get("provisionActor") != actor_ref("system-maintenance", 4) or parsed.get("provisionNonce") != hex_id("provision-nonce") or parsed.get("installerManifestDigest") != hex_digest("installerManifestDigest") or parsed.get("designatedOwnerSidDigest") != owner_sid_digest()):
        return False
    if parsed["stored"].get("volumeSerial") != "1122334455667788":
        return False
    if vector_id == "MAPSCNV1-P-CREATE-ABSENT":
        if parsed["observed"].get("volumeSerial") != "0000000000000000":
            return False
    elif parsed["observed"].get("volumeSerial") != "1122334455667788":
        return False

    config = MAPSCN_POSITIVE.get(vector_id, MAPSCN_POSITIVE["MAPSCNV1-P-RESUME-02"])
    stored_state, current_state, candidate_state, mar_id, mar_class, classification, write_target, next_write = config
    if vector_id == "MAPSCNV1-N-023":
        stored_state, current_state, candidate_state = 1, 2, 4
    elif vector_id in {"MAPSCNV1-N-024", "MAPSCNV1-N-025", "MAPSCNV1-N-026"}:
        stored_state, current_state, candidate_state = 1, 1, 0
    if parsed.get("storedMapState") != f"{stored_state:04x}" or parsed.get("currentMapState") != f"{current_state:04x}" or parsed.get("candidateNextMapState") != f"{candidate_state:04x}":
        return False
    if (
        parsed.get("storedMapRecordSha256") != digest(map_bytes(f"MAPRV1-P-STATE-{stored_state:02d}")).hex()
        or parsed.get("currentMapRecordSha256") != digest(map_bytes(f"MAPRV1-P-STATE-{current_state:02d}")).hex()
        or parsed.get("candidateNextMapRecordSha256") != (ZERO32 if candidate_state == 0 else digest(map_bytes(f"MAPRV1-P-STATE-{candidate_state:02d}")).hex())
    ):
        return False

    # Exact expected positive observation includes the full MAR bytes/hash.
    if mar_id is None:
        expected_bytes = b"" if vector_id in {"MAPSCNV1-P-CREATE-ABSENT", "MAPSCNV1-P-RESUME-01"} else fresh_machine_actor_bytes()
        expected_state, expected_epoch, expected_lease = "0000", ZERO8, ZERO8
    else:
        expected_bytes = mar_bytes(mar_id)
        expected_state, expected_epoch, expected_lease = f"{int(mar_id[-2:], 16) if 'STATE' in mar_id else 9:04x}", "0000000000000001", "0000000000000001"
    exact_observation = (
        parsed.get("observedByteLength") == f"{len(expected_bytes):016x}"
        and parsed.get("observedMachineActorClassification") == mar_class
        and parsed.get("observedMachineActorFullSha256") == (ZERO32 if not expected_bytes else digest(expected_bytes).hex())
        and parsed.get("observedHeaderDigest") == (ZERO32 if not expected_bytes else digest(expected_bytes[:MAR_HEADER_SIZE]).hex())
        and parsed.get("observedSlotDigest") == (ZERO32 if not expected_bytes else digest(expected_bytes[MAR_HEADER_SIZE:MAR_HEADER_SIZE + MAR_SLOT_SIZE]).hex())
        and parsed.get("observedRecordState") == expected_state and parsed.get("observedEpoch") == expected_epoch and parsed.get("observedLease") == expected_lease
    )
    if vector_id in MAPSCN_POSITIVE:
        if vector_id == "MAPSCNV1-P-CREATE-ABSENT":
            observed_relation_ok = (
                parsed.get("observedPresence") is False
                and parsed.get("observedIdentityStatus") == "ABSENT_EXPECTED"
                and all(value in {"0000000000000000", "00000000000000000000000000000000", ZERO32} for value in observed.values())
            )
        else:
            observed_relation_ok = (
                parsed.get("observedPresence") is True
                and parsed.get("observedIdentityStatus") == "PRESENT_EXACT"
                and stored == observed
            )
        package_ok = True
        if current_state == 6 or candidate_state == 6:
            current_map = map_bytes("MAPRV1-P-STATE-06")
            # State 6 resides in parity B for the 1..6 alternating chain.
            slot = current_map[MAP_HEADER_SIZE + MAP_SLOT_SIZE:MAP_HEADER_SIZE + 2 * MAP_SLOT_SIZE]
            payload_length = int.from_bytes(slot[28:32], "little")
            payload = json.loads(slot[224:224 + payload_length].decode("ascii"), object_pairs_hook=OrderedDict)
            package_ok = parsed.get("packageCompletionDigest") == payload["packageCompletionDigest"] and parsed.get("packageCompletionEvidenceStatus") == "DURABLE_READBACK_EXACT"
        else:
            package_ok = parsed.get("packageCompletionDigest") == ZERO32 and parsed.get("packageCompletionEvidenceStatus") == "NOT_REQUIRED"
        return observed_relation_ok and exact_observation and package_ok and parsed.get("requestedAction") == "READ_ONLY_CLASSIFY" and parsed.get("classification") == classification and parsed.get("soleNextWriteTarget") == write_target and parsed.get("soleNextWrite") == next_write

    expected_negative = {
        "MAPSCNV1-N-010": ("RESUME", "REJECT_DIFFERENT_FILE_ID"), "MAPSCNV1-N-011": ("DELETE_RECREATE", "REJECT_DACL_MISMATCH"),
        "MAPSCNV1-N-012": ("RESUME", "REJECT_ANCHOR_MISMATCH"), "MAPSCNV1-N-013": ("RESUME", "REJECT_ATTRIBUTE_MISMATCH"),
        "MAPSCNV1-N-014": ("RESUME", "REJECT_PARTIAL_HEADER"), "MAPSCNV1-N-015": ("DELETE_RECREATE", "REJECT_PARTIAL_SLOT"),
        "MAPSCNV1-N-016": ("CLEANUP", "REJECT_CHECKSUM_MISMATCH"), "MAPSCNV1-N-017": ("RESUME", "REJECT_PROVISION_ACTOR_MISMATCH"),
        "MAPSCNV1-N-018": ("RESUME", "REJECT_PROVISION_NONCE_MISMATCH"), "MAPSCNV1-N-019": ("RESUME", "REJECT_MANIFEST_MISMATCH"),
        "MAPSCNV1-N-020": ("RESUME", "REJECT_EPOCH_MISMATCH"), "MAPSCNV1-N-021": ("RESUME", "REJECT_LEASE_MISMATCH"),
        "MAPSCNV1-N-022": ("CLEANUP", "REJECT_FAILED_CLOSED_CLEANUP"), "MAPSCNV1-N-023": ("RESUME", "REJECT_TWO_STEP_AHEAD"),
        "MAPSCNV1-N-024": ("RESUME", "REJECT_PRECHECKPOINT_EXISTING_FILE"), "MAPSCNV1-N-025": ("DELETE_RECREATE", "REJECT_PRECHECKPOINT_EXISTING_FILE"), "MAPSCNV1-N-026": ("RECREATE", "REJECT_PRECHECKPOINT_EXISTING_FILE"),
    }
    if vector_id not in expected_negative or parsed.get("requestedAction") != expected_negative[vector_id][0] or parsed.get("classification") != expected_negative[vector_id][1] or parsed.get("soleNextWriteTarget") != "NONE" or parsed.get("soleNextWrite") != "NONE":
        return False
    # One precise bounded discrepancy per negative scenario.
    if vector_id == "MAPSCNV1-N-010": return observed["fileId"] != stored["fileId"] and exact_observation
    if vector_id == "MAPSCNV1-N-011": return observed["daclDigest"] != stored["daclDigest"] and exact_observation
    if vector_id == "MAPSCNV1-N-012": return observed["anchorDigest"] != stored["anchorDigest"] and exact_observation
    if vector_id == "MAPSCNV1-N-013": return observed["attributeDigest"] != stored["attributeDigest"] and exact_observation
    if vector_id == "MAPSCNV1-N-014": return parsed.get("observedByteLength") == "0000000000001000" and parsed.get("observedHeaderDigest") == hex_digest("partial-header-bytes")
    if vector_id == "MAPSCNV1-N-015": return parsed.get("observedByteLength") == "0000000000011000" and parsed.get("observedSlotDigest") == hex_digest("partial-slot-bytes")
    if vector_id == "MAPSCNV1-N-016": return parsed.get("observedSlotDigest") == hex_digest("checksum-mismatch-slot")
    if vector_id == "MAPSCNV1-N-017": return parsed.get("provisionActor", {}).get("instanceId") == hex_id("wrong-provision-actor") and exact_observation
    if vector_id == "MAPSCNV1-N-018": return parsed.get("provisionNonce") == hex_id("wrong-provision-nonce") and exact_observation
    if vector_id == "MAPSCNV1-N-019": return parsed.get("installerManifestDigest") == hex_digest("wrong-manifest") and exact_observation
    if vector_id == "MAPSCNV1-N-020": return parsed.get("observedEpoch") == "0000000000000002" and exact_observation is False
    if vector_id == "MAPSCNV1-N-021": return parsed.get("observedLease") == "0000000000000002" and exact_observation is False
    if vector_id == "MAPSCNV1-N-022":
        failed = mar_bytes("MARV1-P-FC-01")
        return parsed.get("observedMachineActorClassification") == "FAILED_CLOSED" and parsed.get("observedRecordState") == "000c" and parsed.get("observedEpoch") == "0000000000000001" and parsed.get("observedLease") == "0000000000000001" and parsed.get("observedMachineActorFullSha256") == digest(failed).hex() and parsed.get("observedHeaderDigest") == digest(failed[:MAR_HEADER_SIZE]).hex() and parsed.get("observedSlotDigest") == digest(failed[MAR_HEADER_SIZE:MAR_HEADER_SIZE + MAR_SLOT_SIZE]).hex()
    if vector_id == "MAPSCNV1-N-023": return True
    return True


def check_worker_oracle(vector_id: str, data: bytes) -> bool:
    try:
        parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
        baseline, received = parsed["baseline"], parsed["received"]
    except (UnicodeDecodeError, json.JSONDecodeError, KeyError):
        return False
    reason = parsed.get("rejectReason")
    same_process = received["processIdentity"] == baseline["processIdentity"]
    if vector_id == "WOSV1-N-001":
        return same_process and received["instanceId"] != baseline["instanceId"] and reason == "WORKER_INSTANCE_ROTATE_REJECTED"
    if vector_id == "WOSV1-N-002":
        return received["instanceId"] == baseline["instanceId"] and received["role"] != baseline["role"] and received["processIdentity"]["role"] != baseline["processIdentity"]["role"] and reason == "WORKER_ROLE_MISMATCH"
    if vector_id == "WOSV1-N-003":
        return received["operationKind"] != baseline["operationKind"] and reason == "WORKER_OPERATION_MISMATCH"
    if vector_id == "WOSV1-N-004":
        return received["operationNonce"] != baseline["operationNonce"] and reason == "WORKER_NONCE_MISMATCH"
    if vector_id == "WOSV1-N-005":
        return received["processIdentity"]["pid"] == baseline["processIdentity"]["pid"] and received["processIdentity"]["processCreationTime"] != baseline["processIdentity"]["processCreationTime"] and reason == "WORKER_PROCESS_IDENTITY_MISMATCH"
    if vector_id == "WOSV1-N-006":
        return received["oldProcessSignaled"] == "false" and reason == "OLD_WORKER_NOT_QUIESCENT"
    if vector_id == "WOSV1-N-007":
        return received["leaseVersion"] != baseline["leaseVersion"] and reason == "WORKER_LEASE_MISMATCH"
    if vector_id == "WOSV1-N-008":
        frames = received["frameSequence"]
        return frames.count("GO") == 2 and frames[-1] == "TERMINAL" and reason == "WORKER_GO_REPLAY_REJECTED"
    if vector_id == "WOSV1-N-009":
        frames = received["frameSequence"]
        return frames.count("GO") == 1 and frames[-2:] == ["TERMINAL", "AFTER_TERMINAL"] and reason == "WORKER_TERMINAL_AFTER_FRAME_REJECTED"
    if vector_id == "WOSV1-N-010":
        return received["processIdentity"]["pid"] != baseline["processIdentity"]["pid"] and reason == "WORKER_PROCESS_IDENTITY_MISMATCH"
    if vector_id == "WOSV1-N-011":
        return received["processIdentity"]["signedImageIdentity"] != baseline["processIdentity"]["signedImageIdentity"] and reason == "WORKER_PROCESS_IDENTITY_MISMATCH"
    if vector_id == "WOSV1-N-012":
        return received["processIdentity"]["processNonce"] != baseline["processIdentity"]["processNonce"] and reason == "WORKER_PROCESS_IDENTITY_MISMATCH"
    return False


def check_owner_sid_negative(vector_id: str, data: bytes) -> bool:
    try:
        parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
    except (UnicodeDecodeError, json.JSONDecodeError):
        return False
    if vector_id == "OWNERSIDV1-N-005":
        return parsed.get("sid") == "S-1-5-0" and parsed.get("reason") == "STRING_SID_FORBIDDEN"
    if vector_id == "OWNERSIDV1-N-006":
        return parsed.get("sid") == "account-name" and parsed.get("reason") == "ACCOUNT_NAME_FORBIDDEN"
    if vector_id == "OWNERSIDV1-N-007":
        try:
            sid = parsed["sid"]
            capacity = bytes.fromhex(sid["bytes"])
            wrong = digest(b"DisplayDeck.OwnerSidDigest.V1\0" + (68).to_bytes(4, "little") + capacity).hex()
            return parsed.get("reason") == "FIXED_TAIL_IN_PREIMAGE" and parsed.get("wrongPreimageDigest") == wrong and wrong != owner_sid_digest(sid)
        except (KeyError, ValueError):
            return False
    try:
        actual_sid_bytes(parsed["sid"])
    except (ValueError, KeyError, TypeError):
        return True
    return False


def check_positive_vector(vector_id: str, data: bytes) -> bool:
    family = family_for(vector_id)
    if family == "D04TUPV1":
        return check_d04_tuple(vector_id, data)
    if family == "D04D02V1":
        return check_d04_d02(vector_id, data)
    if vector_id == D04_READINESS_ID:
        return check_d04_readiness(data)
    if family in {"D02V1", "D02MARV1"}:
        if vector_id.startswith("D02V1-N-"):
            try:
                parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
                inputs = parsed["inputs"]
                if any(value != 0 for value in parsed["actionCounts"].values()):
                    return False
                if vector_id == "D02V1-N-001":
                    return digest(bytes.fromhex(inputs["subjectDomainSeparatorHex"]) + bytes.fromhex(inputs["subjectSourceHex"])).hex() != inputs["subjectIdentityDigest"]
                if vector_id == "D02V1-N-002":
                    return digest(bytes.fromhex(inputs["outerPreimageHex"])).hex() != inputs["preservedEvidenceDigest"]
                return inputs["subjectIdentityDigest"] == ZERO32 and digest(bytes.fromhex(inputs["outerPreimageHex"])).hex() == inputs["preservedEvidenceDigest"]
            except (UnicodeDecodeError, json.JSONDecodeError, KeyError, ValueError):
                return False
        try:
            if family == "D02MARV1":
                parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
                bound = parsed["boundMarVectorId"]
                binding = MAR_D02_BINDINGS[bound]
                inputs = parsed["inputs"]
                return (
                    parsed["vectorId"] == binding[0]
                    and parsed["context"] == binding[4]
                    and parsed["detail"] == f"{binding[3]:04x}"
                    and parsed["requiredErrorClass"] == f"{binding[2]:04x}"
                    and parsed["requiredEvidenceKind"] == f"{binding[1]:04x}"
                    and digest(bytes.fromhex(inputs["subjectDomainSeparatorHex"]) + bytes.fromhex(inputs["subjectSourceHex"])).hex() == inputs["subjectIdentityDigest"]
                    and digest(bytes.fromhex(inputs["observedDomainSeparatorHex"]) + bytes.fromhex(inputs["observedSourceHex"])).hex() == inputs["observedEvidenceDigest"]
                    and digest(bytes.fromhex(inputs["outerPreimageHex"])).hex() == inputs["preservedEvidenceDigest"]
                )
            parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
            inputs = parsed["inputs"]
            return (
                parsed["vectorId"] == vector_id
                and digest(bytes.fromhex(inputs["subjectDomainSeparatorHex"]) + bytes.fromhex(inputs["subjectSourceHex"])).hex() == inputs["subjectIdentityDigest"]
                and digest(bytes.fromhex(inputs["observedDomainSeparatorHex"]) + bytes.fromhex(inputs["observedSourceHex"])).hex() == inputs["observedEvidenceDigest"]
                and digest(bytes.fromhex(inputs["outerPreimageHex"])).hex() == inputs["preservedEvidenceDigest"]
            )
        except (UnicodeDecodeError, json.JSONDecodeError, KeyError, ValueError):
            return False
    if family == "OWNWALV1":
        try:
            parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
            code = int(parsed["ownerWalState"], 16)
            return code in OWNER_WAL_CODES and parsed["wireBytesLe"] == code.to_bytes(2, "little").hex() and parsed["ownerWalFrameEncodable"] is (code != 0)
        except (UnicodeDecodeError, json.JSONDecodeError, KeyError, ValueError):
            return False
    if family == "MAPSCNV1":
        return check_map_scenario(vector_id, data)
    if family == "OWNERSIDV1":
        try:
            parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
            sid_digest = owner_sid_digest(parsed["sid"])
        except (UnicodeDecodeError, json.JSONDecodeError, KeyError, ValueError):
            return False
        if vector_id == "OWNERSIDV1-P-001":
            return data == owner_sid_cross_link_bytes() and all(parsed.get(key) == sid_digest for key in ("ownerSidDigest", "decisionJournalOwnerSidDigest", "machineActorOwnerSidDigest", "lockNameOwnerSidDigest"))
        return parsed.get("ownerSidDigest") == sid_digest and int(parsed["sid"]["byteLength"], 16) in {8, 68}
    if family == "BOOTIDV1":
        try:
            parsed = json.loads(data.decode("ascii"), object_pairs_hook=OrderedDict)
        except (UnicodeDecodeError, json.JSONDecodeError):
            return False
        try:
            preimage = (
                b"DisplayDeck.BootId.V1\0"
                + int(parsed["lastBootUtcFileTime"], 16).to_bytes(8, "little")
                + int(parsed["versionMajor"], 16).to_bytes(4, "little")
                + int(parsed["versionMinor"], 16).to_bytes(4, "little")
                + int(parsed["versionBuild"], 16).to_bytes(4, "little")
            )
        except (KeyError, ValueError, OverflowError):
            return False
        return data == bootid_bytes() and parsed.get("scope") == "STATIC_PREIMAGE_ONLY" and parsed.get("runtimeAcceptance") == "UNSET_EVIDENCE_PENDING" and parsed.get("expectedBootIdSha256") == digest(preimage).hex()
    if family == "DJV1":
        if not check_header(data, "4444444a563100000000000000000000", DJ_SLOT_SIZE, DJ_FILE_LENGTH):
            return False
        return all(check_dj_slot(data[DJ_HEADER_SIZE + index * DJ_SLOT_SIZE : DJ_HEADER_SIZE + (index + 1) * DJ_SLOT_SIZE], index) for index in range(2))
    if family == "MARV1":
        if not check_header(data, "44444d41525631000000000000000000", MAR_SLOT_SIZE, MAR_FILE_LENGTH):
            return False
        return all(check_mar_slot(data[MAR_HEADER_SIZE + index * MAR_SLOT_SIZE : MAR_HEADER_SIZE + (index + 1) * MAR_SLOT_SIZE], index) for index in range(2))
    if family == "MAPRV1":
        if not check_header(data, "44444d41505256310000000000000000", MAP_SLOT_SIZE, MAP_FILE_LENGTH):
            return False
        states: list[tuple[int, bytes, OrderedDict[str, Any]]] = []
        for index in range(2):
            slot = data[MAP_HEADER_SIZE + index * MAP_SLOT_SIZE : MAP_HEADER_SIZE + (index + 1) * MAP_SLOT_SIZE]
            valid, payload = check_map_slot(slot, index)
            if not valid:
                return False
            if payload is not None:
                states.append((slot[21], slot, payload))
        if vector_id == "MAPRV1-P-000":
            return not states
        current = max(states, key=lambda item: item[0])
        if current[0] < 2:
            return True
        predecessor = next((item for item in states if item[0] == current[0] - 1), None)
        return predecessor is not None and current[2]["previousSlotDigest"] == digest(predecessor[1]).hex()
    return False


def generate(output: Path) -> None:
    staging = output.with_name(output.name + ".staging")
    if staging.exists():
        shutil.rmtree(staging)
    (staging / "bytes").mkdir(parents=True)
    for vector_id in vector_catalog():
        contents = vector_bytes(vector_id)
        (staging / "bytes" / f"{vector_id}.bin").write_bytes(contents)
    semantic = expected_semantic_manifest()
    (staging / "semantic-manifest.jsonl").write_bytes(semantic)
    semantic_sha256 = digest(semantic).hex()
    entries = expected_entries(staging)
    index = OrderedDict((
        ("indexSchema", "DD-FR-002-ARTIFACT-INDEX-V1"),
        ("profileId", PROFILE),
        ("semanticManifestSha256", semantic_sha256),
        ("entries", entries),
    ))
    (staging / "artifact-index.json").write_bytes(canonical_json(index) + b"\n")
    metadata = OrderedDict((
        ("metadataSchema", "DD-FR-002-FREEZE-METADATA-V1"),
        ("artifactId", ARTIFACT_ID),
        ("generator", TOOL_ID),
        ("notProductImplementation", True),
        ("vectorCount", len(vector_catalog())),
        ("staticD08Scope", "STATIC_PREIMAGE_ONLY; runtime tick/UTC tolerance remains UNSET_EVIDENCE_PENDING"),
        ("allDeclaredSideEffectsZero", True),
        ("semanticManifestSha256", semantic_sha256),
        ("artifactIndexSha256", digest((staging / "artifact-index.json").read_bytes()).hex()),
        ("aggregateFixtureSetSha256", aggregate_fixture_set_sha256(semantic_sha256, entries)),
    ))
    (staging / "freeze-candidate-metadata.json").write_bytes(canonical_json(metadata) + b"\n")
    checksum_rows = all_artifact_files(staging)
    (staging / "SHA256SUMS").write_text("\n".join(f"{digest(contents).hex()}  {relative}" for relative, contents in checksum_rows) + "\n", encoding="ascii", newline="\n")
    if output.exists():
        shutil.rmtree(output)
    staging.rename(output)


def verify(output: Path) -> list[str]:
    errors: list[str] = []
    index_path, sums_path = output / "artifact-index.json", output / "SHA256SUMS"
    semantic_path, metadata_path = output / "semantic-manifest.jsonl", output / "freeze-candidate-metadata.json"
    if not output.is_dir():
        return ["artifact directory missing"]
    if not index_path.is_file() or not sums_path.is_file() or not semantic_path.is_file() or not metadata_path.is_file():
        return ["artifact index, metadata, semantic manifest, or SHA256SUMS missing"]
    try:
        index = json.loads(index_path.read_text(encoding="ascii"), object_pairs_hook=OrderedDict)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        return [f"artifact index invalid: {error}"]
    if list(index) != ["indexSchema", "profileId", "semanticManifestSha256", "entries"]:
        errors.append("artifact index key order/schema mismatch")
    if index.get("indexSchema") != "DD-FR-002-ARTIFACT-INDEX-V1" or index.get("profileId") != PROFILE:
        errors.append("artifact identity mismatch")
    semantic = semantic_path.read_bytes()
    if semantic != expected_semantic_manifest():
        errors.append("semantic manifest bytes mismatch")
    if index.get("semanticManifestSha256") != digest(semantic).hex():
        errors.append("semantic manifest digest mismatch")
    try:
        metadata = json.loads(metadata_path.read_text(encoding="ascii"), object_pairs_hook=OrderedDict)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        return [f"freeze metadata invalid: {error}"]
    if metadata.get("artifactId") != ARTIFACT_ID or metadata.get("generator") != TOOL_ID or metadata.get("vectorCount") != len(vector_catalog()):
        errors.append("freeze metadata identity/count mismatch")
    if metadata.get("staticD08Scope") != "STATIC_PREIMAGE_ONLY; runtime tick/UTC tolerance remains UNSET_EVIDENCE_PENDING":
        errors.append("D08 static scope mismatch")
    if metadata.get("allDeclaredSideEffectsZero") is not True:
        errors.append("metadata side-effect declaration mismatch")
    entries = expected_entries(output)
    if index.get("entries") != entries:
        errors.append("artifact index entries mismatch")
    if metadata.get("artifactIndexSha256") != digest(index_path.read_bytes()).hex():
        errors.append("artifact index sidecar digest mismatch")
    if metadata.get("aggregateFixtureSetSha256") != aggregate_fixture_set_sha256(digest(semantic).hex(), entries):
        errors.append("aggregate fixture set digest mismatch")
    checksum_entries: dict[str, str] = {}
    for line in sums_path.read_text(encoding="ascii").splitlines():
        try:
            sha, relative = line.split("  ", 1)
            checksum_entries[relative] = sha
        except ValueError:
            errors.append("invalid SHA256SUMS line")
    for relative, contents in all_artifact_files(output):
        if checksum_entries.get(relative) != digest(contents).hex():
            errors.append(f"SHA256SUMS mismatch: {relative}")
    if len(checksum_entries) != len(all_artifact_files(output)):
        errors.append("SHA256SUMS entry count mismatch")
    for vector_id in vector_catalog():
        fixture = output / "bytes" / f"{vector_id}.bin"
        if not fixture.is_file():
            errors.append(f"vector missing: {vector_id}")
            continue
        if fixture.read_bytes() != vector_bytes(vector_id):
            errors.append(f"vector bytes mismatch: {vector_id}")
        if "-P-" in vector_id and not check_positive_vector(vector_id, fixture.read_bytes()):
            errors.append(f"positive structural validation failed: {vector_id}")
        if vector_id.startswith("DJV1-N-") and not check_dj_negative(vector_id, fixture.read_bytes()):
            errors.append(f"DJ negative oracle failed: {vector_id}")
        if vector_id.startswith("MARV1-N-") and not check_mar_negative(vector_id, fixture.read_bytes()):
            errors.append(f"MAR negative oracle failed: {vector_id}")
        if vector_id.startswith("MAPRV1-N-") and not check_map_negative(vector_id, fixture.read_bytes()):
            errors.append(f"MAP negative oracle failed: {vector_id}")
        if vector_id.startswith("MAPSCNV1-") and not check_map_scenario(vector_id, fixture.read_bytes()):
            errors.append(f"MAP read-only scenario oracle failed: {vector_id}")
        if family_for(vector_id) == "WOSV1" and not check_worker_oracle(vector_id, fixture.read_bytes()):
            errors.append(f"worker oracle validation failed: {vector_id}")
        if family_for(vector_id) == "D04TUPV1" and not check_d04_tuple(vector_id, fixture.read_bytes()):
            errors.append(f"D04 tuple oracle failed: {vector_id}")
        if vector_id.startswith("OWNERSIDV1-N-") and not check_owner_sid_negative(vector_id, fixture.read_bytes()):
            errors.append(f"owner SID negative oracle failed: {vector_id}")
    # Context-bound D02 artifacts must exactly rehash to the E group embedded
    # in their linked MAR fixture.  This catches a stale context, stale digest,
    # or an index link that merely shares a detail number.
    for mar_id, (d02_id, evidence_kind, error_class, detail, context) in MAR_D02_BINDINGS.items():
        try:
            mar = (output / "bytes" / f"{mar_id}.bin").read_bytes()
            slot = mar[MAR_HEADER_SIZE : MAR_HEADER_SIZE + MAR_SLOT_SIZE]
            payload_length = int.from_bytes(slot[28:32], "little")
            payload = json.loads(slot[136 : 136 + payload_length].decode("ascii"), object_pairs_hook=OrderedDict)
            d02 = json.loads((output / "bytes" / f"{d02_id}.bin").read_text(encoding="ascii"), object_pairs_hook=OrderedDict)
            expected = critical_evidence_inputs(evidence_kind, error_class, detail, context)
            if (
                d02.get("boundMarVectorId") != mar_id
                or d02.get("inputs") != expected
                or payload.get("preservedEvidenceDigest") != expected["preservedEvidenceDigest"]
                or d02_id not in linked_vectors_for(mar_id)
                or mar_id not in linked_vectors_for(d02_id)
            ):
                errors.append(f"MAR/D02 context binding mismatch: {mar_id}/{d02_id}")
        except (OSError, UnicodeDecodeError, json.JSONDecodeError, KeyError, ValueError):
            errors.append(f"MAR/D02 context evidence unavailable: {mar_id}/{d02_id}")
    try:
        owner_cross_link = json.loads((output / "bytes" / "OWNERSIDV1-P-001.bin").read_text(encoding="ascii"), object_pairs_hook=OrderedDict)
        expected_owner_digest = owner_sid_digest(owner_cross_link["sid"])
        if any(owner_cross_link.get(key) != expected_owner_digest for key in ("ownerSidDigest", "decisionJournalOwnerSidDigest", "machineActorOwnerSidDigest", "lockNameOwnerSidDigest")):
            errors.append("owner SID cross-link vector mismatch")
        dj = (output / "bytes" / "DJV1-P-002.bin").read_bytes()
        mar = (output / "bytes" / "MARV1-P-STATE-01.bin").read_bytes()
        map_fixture = (output / "bytes" / "MAPRV1-P-STATE-03.bin").read_bytes()
        mar_payload_length = int.from_bytes(mar[MAR_HEADER_SIZE + 28 : MAR_HEADER_SIZE + 32], "little")
        mar_payload = json.loads(mar[MAR_HEADER_SIZE + 136 : MAR_HEADER_SIZE + 136 + mar_payload_length].decode("ascii"))
        map_payload_length = int.from_bytes(map_fixture[MAP_HEADER_SIZE + 28 : MAP_HEADER_SIZE + 32], "little")
        map_payload_object = json.loads(map_fixture[MAP_HEADER_SIZE + 224 : MAP_HEADER_SIZE + 224 + map_payload_length].decode("ascii"))
        if dj[DJ_HEADER_SIZE + 176 : DJ_HEADER_SIZE + 208].hex() != expected_owner_digest or owner_sid_digest(mar_payload["ownerSid"]) != expected_owner_digest or map_payload_object["designatedOwnerSidDigest"] != expected_owner_digest:
            errors.append("DJ/MAR/MAP/lock-name owner SID digest cross-link mismatch")
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, KeyError, ValueError):
        errors.append("owner SID cross-link evidence unavailable")
    negative_digests: dict[str, str] = {}
    positive_digests = {digest((output / "bytes" / f"{vector_id}.bin").read_bytes()).hex() for vector_id in vector_catalog() if "-P-" in vector_id}
    for vector_id in vector_catalog():
        if "-N-" not in vector_id:
            continue
        fixture_digest = digest((output / "bytes" / f"{vector_id}.bin").read_bytes()).hex()
        if fixture_digest in positive_digests and vector_id not in {"DJV1-N-006", "MAPRV1-N-001"}:
            errors.append(f"negative equals positive fixture: {vector_id}")
        other = negative_digests.get(fixture_digest)
        if other is not None:
            errors.append(f"negative fixture collision: {other}/{vector_id}")
        negative_digests[fixture_digest] = vector_id
    for entry in entries:
        if list(entry) != ["vectorId", "relativePath", "byteLength", "fixtureSha256", "linkedVectorIds", "linkedFixtureSha256s"]:
            errors.append(f"index entry key order mismatch: {entry.get('vectorId', '?')}")
        if entry["linkedVectorIds"] != bytewise_sorted(set(entry["linkedVectorIds"])) or entry["vectorId"] in entry["linkedVectorIds"]:
            errors.append(f"invalid index links: {entry['vectorId']}")
        linked_hashes = [next(candidate["fixtureSha256"] for candidate in entries if candidate["vectorId"] == linked_id) for linked_id in entry["linkedVectorIds"]]
        if entry["linkedFixtureSha256s"] != linked_hashes:
            errors.append(f"linked fixture hash mismatch: {entry['vectorId']}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate or verify DD-FR-002 candidate freeze evidence.")
    parser.add_argument("command", choices=("generate", "verify"))
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    if args.command == "generate":
        generate(args.output)
        errors = verify(args.output)
        if errors:
            raise SystemExit("generation verification failed: " + "; ".join(errors))
        print(f"generated vectors={len(vector_catalog())} side-effects=0 verification=pass")
        return 0
    errors = verify(args.output)
    if errors:
        print("verification=fail " + "; ".join(errors))
        return 1
    print(f"verification=pass vectors={len(vector_catalog())} side-effects=0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
