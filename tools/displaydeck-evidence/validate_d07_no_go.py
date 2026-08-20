#!/usr/bin/env python3
"""Dependency-free validator for the fixed D07 no-go evidence predicate."""

import argparse
import json
import tempfile
from pathlib import Path

TOP = [
    "schemaVersion", "evidenceStatus", "directoryAnchorClassification", "windowsProbeStatus",
    "requiredProofs", "requiredSideEffects", "decision",
]
PROOFS = [
    "raceResistantDirectoryAnchorAlgorithm", "reparsePointRejection",
    "fileIdDaclAttributeReadback", "localFixedNtfsAndPersistentAcls",
]
SIDE_EFFECTS = [
    "provisionCreate", "provisionStorageWrite", "storageWrite", "fileDelete",
    "fileTruncate", "displayMutation", "processLaunch",
]


def fail(message):
    raise ValueError(message)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            fail("duplicate JSON key: " + key)
        result[key] = value
    return result


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle, object_pairs_hook=unique_object)


def exact_keys(value, keys, name):
    if not isinstance(value, dict) or list(value) != keys:
        fail(name + " key order or key set is not canonical")


def validate(value):
    exact_keys(value, TOP, "D07 predicate")
    if value["schemaVersion"] != "D07-NO-GO-PREDICATE-V1":
        fail("unsupported schemaVersion")
    if value["evidenceStatus"] != "NO_GO_RECORDED":
        fail("D07 must retain the recorded no-go result")
    if value["directoryAnchorClassification"] != "DIRECTORY_ANCHOR_UNPROVEN":
        fail("directory anchor must remain unproven")
    if value["windowsProbeStatus"] != "NOT_RUN_DIRECTORY_ANCHOR_UNPROVEN":
        fail("D07 Windows writes/probe must remain stopped at the unproven anchor predicate")
    exact_keys(value["requiredProofs"], PROOFS, "requiredProofs")
    if any(value["requiredProofs"][key] != "UNPROVEN" for key in PROOFS):
        fail("all D07 proofs must remain UNPROVEN")
    exact_keys(value["requiredSideEffects"], SIDE_EFFECTS, "requiredSideEffects")
    for key in SIDE_EFFECTS:
        count = value["requiredSideEffects"][key]
        if isinstance(count, bool) or not isinstance(count, int) or count != 0:
            fail("D07 side effect must be exactly zero: " + key)
    if value["decision"] != "NO_GO":
        fail("D07 must remain NO_GO")


def self_test():
    here = Path(__file__).resolve().parent
    template = load_json(here / "d07-no-go-predicate.template.json")
    validate(template)
    bad = dict(template)
    bad["decision"] = "GO"
    try:
        validate(bad)
    except ValueError:
        pass
    else:
        fail("self-test did not reject GO")
    with tempfile.TemporaryDirectory() as temporary:
        duplicate = Path(temporary) / "duplicate.json"
        duplicate.write_text('{"schemaVersion":"x","schemaVersion":"y"}', encoding="utf-8")
        try:
            load_json(duplicate)
        except ValueError:
            pass
        else:
            fail("self-test did not reject duplicate JSON key")
    print("self-test: pass")


def main():
    parser = argparse.ArgumentParser(description="Validate the fixed D07 no-go predicate")
    parser.add_argument("predicate", type=Path, nargs="?")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        if args.predicate is not None:
            parser.error("--self-test does not take a predicate")
        self_test()
        return
    if args.predicate is None:
        parser.error("predicate is required")
    validate(load_json(args.predicate))
    print("valid: " + str(args.predicate))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        raise SystemExit("invalid: " + str(error))
