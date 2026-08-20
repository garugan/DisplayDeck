#!/usr/bin/env python3
"""Dependency-free validation for the bounded D08 read-only evidence format.

This is an offline evidence-format checker.  It does not call Windows APIs,
start a process, write a capture, or create same-boot acceptance authority.
"""

import argparse
import datetime
import hashlib
import json
import re
import tempfile
from pathlib import Path


CAPTURE_KEYS = [
    "schemaVersion", "captureStatus", "probeAuthorization", "lastBootUpTimeRaw",
    "versionRaw", "buildNumberRaw", "tickBeforeMs", "utcBeforeFileTime",
    "tickAfterMs", "utcAfterFileTime", "maxBootIdentitySampleSpanMs",
    "maxBootUtcDelta100ns", "clockJumpRule", "result",
]
STATIC_KEYS = [
    "schemaVersion", "vectorId", "purpose", "domainAsciiWithNul",
    "lastBootUtcFileTimeHex", "versionMajorHex", "versionMinorHex",
    "versionBuildHex", "preimageHex", "expectedSha256",
    "crossCheckThresholdStatus", "windowsReadOnlyProbeStatus", "phase2AAuthorized",
    "displayMutationAllowed",
]

HEX16 = re.compile(r"^[0-9a-f]{16}$")
HEX8 = re.compile(r"^[0-9a-f]{8}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
DMTF = re.compile(r"^[1-9][0-9]{3}[0-9]{10}\.[0-9]{6}[+-][0-9]{3}$")
VERSION = re.compile(r"^[1-9][0-9]{0,4}\.[0-9]{1,5}\.[0-9]{1,10}$")
BUILD = re.compile(r"^(0|[1-9][0-9]{0,9})$")
VECTOR_ID = re.compile(r"^[A-Z0-9][A-Z0-9.-]{2,127}$")

DOMAIN = b"DisplayDeck.BootId.V1\0"
UNSET_DATA_FIELDS = (
    "lastBootUpTimeRaw", "versionRaw", "buildNumberRaw", "tickBeforeMs",
    "utcBeforeFileTime", "tickAfterMs", "utcAfterFileTime",
)
THRESHOLD_FIELDS = (
    "maxBootIdentitySampleSpanMs", "maxBootUtcDelta100ns", "clockJumpRule",
)
REJECTION_RESULTS = {
    "REJECT_NO_AUTHORITY", "REJECT_CROSS_CHECK", "ACCEPTANCE_NOT_AUTHORIZED",
}


def fail(message):
    raise ValueError(message)


def unique_object(pairs):
    """Reject duplicate keys instead of silently retaining the last value."""
    output = {}
    for key, value in pairs:
        if key in output:
            fail("duplicate JSON key: " + key)
        output[key] = value
    return output


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle, object_pairs_hook=unique_object)


def exact_keys(value, keys, name):
    if not isinstance(value, dict) or list(value) != keys:
        fail(name + " key order or key set is not canonical")


def require_string(value, field, pattern=None):
    if not isinstance(value, str) or not value.isascii():
        fail(field + " must be an ASCII string")
    if pattern is not None and pattern.fullmatch(value) is None:
        fail(field + " has an invalid canonical form")


def require_unset_or(value, field, pattern):
    require_string(value, field)
    if value != "UNSET" and pattern.fullmatch(value) is None:
        fail(field + " must be UNSET or a bounded canonical value")


def validate_dmtf_timestamp(value):
    if value == "UNSET":
        return
    # DMTF's fixed-width timestamp has no leap-second representation.  Check
    # the calendar portion as well as the transport grammar before accepting
    # it as a captured raw observation.
    try:
        datetime.datetime.strptime(value[:14], "%Y%m%d%H%M%S")
    except ValueError:
        fail("lastBootUpTimeRaw has an invalid DMTF calendar value")


def validate_capture(capture):
    """Validate the transport format; no outcome can claim acceptance."""
    exact_keys(capture, CAPTURE_KEYS, "D08 capture")
    if capture["schemaVersion"] != "D08-READONLY-CAPTURE-V1":
        fail("unsupported D08 capture schemaVersion")
    if capture["captureStatus"] not in {"TEMPLATE", "PENDING", "CAPTURED"}:
        fail("unknown captureStatus")
    if capture["probeAuthorization"] not in {"NOT_AUTHORIZED", "READ_ONLY_AUTHORIZED"}:
        fail("unknown probeAuthorization")

    require_unset_or(capture["lastBootUpTimeRaw"], "lastBootUpTimeRaw", DMTF)
    validate_dmtf_timestamp(capture["lastBootUpTimeRaw"])
    require_unset_or(capture["versionRaw"], "versionRaw", VERSION)
    require_unset_or(capture["buildNumberRaw"], "buildNumberRaw", BUILD)
    for field in ("tickBeforeMs", "utcBeforeFileTime", "tickAfterMs", "utcAfterFileTime"):
        require_unset_or(capture[field], field, HEX16)
    for field in THRESHOLD_FIELDS:
        if capture[field] != "UNSET":
            fail(field + " must remain UNSET until a separate human freeze")

    result = capture["result"]
    if result not in {"PENDING", *REJECTION_RESULTS}:
        fail("D08 does not define an acceptance result")

    status = capture["captureStatus"]
    data_is_unset = all(capture[field] == "UNSET" for field in UNSET_DATA_FIELDS)
    data_is_complete = all(capture[field] != "UNSET" for field in UNSET_DATA_FIELDS)
    if status == "TEMPLATE":
        if capture["probeAuthorization"] != "READ_ONLY_AUTHORIZED":
            fail("the distributed template records the bounded read-only authorization")
        if not data_is_unset or result != "PENDING":
            fail("template observations and result must remain UNSET/PENDING")
    elif status == "PENDING":
        if not data_is_unset or result != "PENDING":
            fail("pending capture cannot contain partial observations or a conclusion")
    else:
        if capture["probeAuthorization"] != "READ_ONLY_AUTHORIZED":
            fail("captured D08 evidence requires READ_ONLY_AUTHORIZED")
        if not data_is_complete:
            fail("captured D08 evidence requires all seven raw observations")
        if result not in REJECTION_RESULTS:
            fail("captured D08 evidence is never same-boot acceptance authority")
        if int(capture["tickAfterMs"], 16) < int(capture["tickBeforeMs"], 16):
            fail("captured tick order is inconsistent; record REJECT_CROSS_CHECK instead")


def static_preimage(vector):
    return (
        DOMAIN
        + int(vector["lastBootUtcFileTimeHex"], 16).to_bytes(8, "little")
        + int(vector["versionMajorHex"], 16).to_bytes(4, "little")
        + int(vector["versionMinorHex"], 16).to_bytes(4, "little")
        + int(vector["versionBuildHex"], 16).to_bytes(4, "little")
    )


def validate_static_vector(vector):
    """Recompute the static BootId known answer, including exact NUL domain bytes."""
    exact_keys(vector, STATIC_KEYS, "D08 static vector")
    if vector["schemaVersion"] != "D08-BOOTID-STATIC-VECTOR-V1":
        fail("unsupported D08 static vector schemaVersion")
    require_string(vector["vectorId"], "vectorId", VECTOR_ID)
    require_string(vector["purpose"], "purpose")
    if vector["domainAsciiWithNul"] != DOMAIN.decode("ascii"):
        fail("BootId domain must be exactly DisplayDeck.BootId.V1 followed by one NUL")
    for field in ("lastBootUtcFileTimeHex",):
        require_string(vector[field], field, HEX16)
    for field in ("versionMajorHex", "versionMinorHex", "versionBuildHex"):
        require_string(vector[field], field, HEX8)
    require_string(vector["preimageHex"], "preimageHex", re.compile(r"^[0-9a-f]{84}$"))
    require_string(vector["expectedSha256"], "expectedSha256", HEX64)
    if vector["crossCheckThresholdStatus"] != "UNSET":
        fail("static vector cannot set a D08 cross-check threshold")
    if vector["windowsReadOnlyProbeStatus"] != "NOT_RUN_STATIC_VECTOR_ONLY":
        fail("static vector must record that no Windows probe was run")
    if vector["phase2AAuthorized"] is not False or vector["displayMutationAllowed"] is not False:
        fail("D08 static vector cannot authorize Phase 2A or display mutation")

    expected_preimage = static_preimage(vector)
    if vector["preimageHex"] != expected_preimage.hex():
        fail("static vector preimageHex is not the exact canonical BootId preimage")
    if vector["expectedSha256"] != hashlib.sha256(expected_preimage).hexdigest():
        fail("static vector expectedSha256 does not match the canonical preimage")


def rejects(action, description):
    try:
        action()
    except ValueError:
        return
    fail("self-test did not reject " + description)


def self_test():
    here = Path(__file__).resolve().parent
    capture = load_json(here / "d08-readonly-capture.template.json")
    vector = load_json(here / "d08-bootid-static-vector-v1.json")
    validate_capture(capture)
    validate_static_vector(vector)

    with tempfile.TemporaryDirectory() as temporary:
        duplicate = Path(temporary) / "duplicate.json"
        duplicate.write_text('{"schemaVersion":"x","schemaVersion":"y"}', encoding="utf-8")
        rejects(lambda: load_json(duplicate), "duplicate JSON key")

    false_acceptance = dict(capture)
    false_acceptance["captureStatus"] = "CAPTURED"
    for field, value in {
        "lastBootUpTimeRaw": "20260813010203.000000+000",
        "versionRaw": "10.0.26100",
        "buildNumberRaw": "26100",
        "tickBeforeMs": "0000000000000001",
        "utcBeforeFileTime": "01da3c457689c000",
        "tickAfterMs": "0000000000000002",
        "utcAfterFileTime": "01da3c45768a0000",
    }.items():
        false_acceptance[field] = value
    false_acceptance["result"] = "ACCEPT"
    rejects(lambda: validate_capture(false_acceptance), "false acceptance")

    altered_digest = dict(vector)
    altered_digest["expectedSha256"] = "0" * 64
    rejects(lambda: validate_static_vector(altered_digest), "altered static digest")
    print("self-test: pass")


def main():
    parser = argparse.ArgumentParser(description="Validate bounded D08 read-only evidence")
    parser.add_argument("capture", type=Path, nargs="?", help="D08 capture JSON")
    parser.add_argument(
        "--static-vector", type=Path,
        default=Path(__file__).resolve().parent / "d08-bootid-static-vector-v1.json",
        help="D08 static BootId known-answer JSON",
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        if args.capture is not None:
            parser.error("--self-test does not take a capture")
        self_test()
        return
    if args.capture is None:
        parser.error("capture is required")
    validate_capture(load_json(args.capture))
    validate_static_vector(load_json(args.static_vector))
    print("valid: " + str(args.capture))
    print("valid static vector: " + str(args.static_vector))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        raise SystemExit("invalid: " + str(error))
