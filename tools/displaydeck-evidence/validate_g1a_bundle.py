#!/usr/bin/env python3
"""Dependency-free, read-only validation for DisplayDeck G1A evidence bundles.

This is an evidence-format checker, not product code.  It never creates,
changes, truncates, or deletes a supplied bundle artifact.
"""

import argparse
import hashlib
import json
import os
import re
import tempfile
from pathlib import Path, PurePosixPath

MAX_BYTES = 8 * 1024 * 1024
ZERO_SHA256 = "0" * 64
STATUSES = {"PENDING", "CAPTURED", "VERIFIED", "REJECTED", "NOT_APPLICABLE"}
REFERENCES = re.compile(r"^(PENDING|[A-Z0-9][A-Z0-9._:/@-]{2,255})$")
UTC = re.compile(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
IDENTIFIER = re.compile(r"^[A-Z0-9][A-Z0-9._-]{2,127}$")
MEDIA_TYPE = re.compile(r"^[A-Za-z0-9.+-]+/[A-Za-z0-9.+-]+$")
RELATIVE_PATH = re.compile(
    r"^[A-Za-z0-9][A-Za-z0-9._-]*(?:/[A-Za-z0-9][A-Za-z0-9._-]*)*$"
)

TOP_LEVEL_KEYS = [
    "schemaVersion", "bundleStatus", "closureStatus", "bundleId", "createdAtUtc",
    "evidenceOwner", "operator", "reviewer", "targetMachineAlias",
    "redactionReference", "retentionReference", "accessReference", "locationReference",
    "repositoryBinding", "sourceBinding", "toolchainBinding", "buildBinding",
    "allowlistStatus", "callTraceStatus", "environmentStatus", "observationsStatus",
    "reviewStatus", "gapsStatus", "phase2AAuthorized", "displayMutationAllowed",
    "maxBundleBytes", "oversizeDisposition", "artifacts", "totalArtifactBytes",
    "bundleContentSha256",
]
ARTIFACT_KEYS = ["artifactId", "relativePath", "mediaType", "byteLength", "sha256", "truncation"]


def fail(message):
    raise ValueError(message)


def unique_object(pairs):
    """Reject duplicate JSON keys instead of silently retaining the last one."""
    result = {}
    for key, value in pairs:
        if key in result:
            fail("duplicate JSON key: " + key)
        result[key] = value
    return result


def load_json(path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle, object_pairs_hook=unique_object)


def sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require_string(value, field, pattern=None):
    if not isinstance(value, str) or (pattern is not None and not pattern.fullmatch(value)):
        fail(field + " has an invalid type or canonical form")


def require_reference(value, field):
    require_string(value, field, REFERENCES)


def require_status(value, field):
    if not isinstance(value, str) or value not in STATUSES:
        fail(field + " has an unknown status")


def require_exact_keys(value, expected, field):
    if not isinstance(value, dict) or list(value) != expected:
        fail(field + " key order or key set is not canonical")


def require_nonnegative_int(value, field, maximum):
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= maximum:
        fail(field + " must be a bounded integer")


def resolve_artifact_root(raw_root):
    if raw_root.is_symlink() or not raw_root.is_dir():
        fail("artifact root must be an existing non-symlink directory")
    return raw_root.resolve(strict=True)


def checked_artifact_path(root, relative_path):
    require_string(relative_path, "artifact.relativePath", RELATIVE_PATH)
    # The grammar is POSIX-only so a Windows separator cannot become a hidden path
    # separator when this format is consumed on Windows.
    pure = PurePosixPath(relative_path)
    if pure.is_absolute() or any(part in {"", ".", ".."} for part in pure.parts):
        fail("artifact relativePath is not normalized")
    path = root.joinpath(*pure.parts)
    current = root
    for part in pure.parts:
        current = current / part
        if current.is_symlink():
            fail("artifact path contains a symlink: " + relative_path)
    try:
        path.resolve(strict=False).relative_to(root)
    except ValueError:
        fail("artifact relativePath escapes artifact root")
    return path


def canonical_bundle_hash(manifest):
    """Bind all audit metadata and artifact inventory with a zeroed self-field."""
    bound = dict(manifest)
    bound["bundleContentSha256"] = ZERO_SHA256
    encoded = json.dumps(
        bound, ensure_ascii=True, separators=(",", ":"), sort_keys=False
    ).encode("ascii")
    return hashlib.sha256(b"DisplayDeck.G1ABundleContent.V1\0" + encoded).hexdigest()


def validate(manifest, artifact_root):
    require_exact_keys(manifest, TOP_LEVEL_KEYS, "manifest")
    if manifest["schemaVersion"] != "G1A-BUNDLE-MANIFEST-V1":
        fail("unsupported schemaVersion")
    if manifest["bundleStatus"] not in {"TEMPLATE", "EVIDENCE_COLLECTED", "REVIEW_READY"}:
        fail("unknown bundleStatus")
    if manifest["closureStatus"] not in {"PENDING", "HUMAN_REVIEW_REQUIRED"}:
        fail("a G1A evidence manifest cannot self-close")
    require_string(manifest["bundleId"], "bundleId", IDENTIFIER)
    if manifest["phase2AAuthorized"] is not False or manifest["displayMutationAllowed"] is not False:
        fail("Phase 2A and display mutation must remain unauthorized")
    if manifest["maxBundleBytes"] != MAX_BYTES:
        fail("maxBundleBytes must be exactly 8388608")
    if manifest["oversizeDisposition"] != "INVALID_BUNDLE_SEPARATE_FAILED_RUN":
        fail("oversize artifact disposition must reject rather than truncate")
    if manifest["createdAtUtc"] != "PENDING":
        require_string(manifest["createdAtUtc"], "createdAtUtc", UTC)
    for field in (
        "evidenceOwner", "operator", "reviewer", "targetMachineAlias", "redactionReference",
        "retentionReference", "accessReference", "locationReference",
    ):
        require_reference(manifest[field], field)
    for field in ("repositoryBinding", "sourceBinding", "toolchainBinding", "buildBinding"):
        binding = manifest[field]
        require_exact_keys(binding, ["status", "reference"], field)
        require_status(binding["status"], field + ".status")
        require_reference(binding["reference"], field + ".reference")
    for field in ("allowlistStatus", "callTraceStatus", "environmentStatus", "observationsStatus", "reviewStatus", "gapsStatus"):
        require_status(manifest[field], field)
    if not isinstance(manifest["artifacts"], list) or len(manifest["artifacts"]) > 256:
        fail("artifacts must be a bounded array")

    root = resolve_artifact_root(artifact_root)
    total = 0
    prior_id = prior_path = None
    artifact_ids, relative_paths = set(), set()
    for artifact in manifest["artifacts"]:
        require_exact_keys(artifact, ARTIFACT_KEYS, "artifact")
        artifact_id = artifact["artifactId"]
        relative_path = artifact["relativePath"]
        require_string(artifact_id, "artifactId", IDENTIFIER)
        checked_path = checked_artifact_path(root, relative_path)
        require_string(artifact["mediaType"], "mediaType", MEDIA_TYPE)
        require_nonnegative_int(artifact["byteLength"], "artifact.byteLength", MAX_BYTES)
        require_string(artifact["sha256"], "artifact.sha256", SHA256)
        if artifact["truncation"] is not False:
            fail("formal G1A artifacts must not be truncated")
        if artifact_id in artifact_ids or relative_path in relative_paths:
            fail("artifactId and relativePath must each be unique")
        if prior_id is not None and artifact_id.encode("ascii") <= prior_id.encode("ascii"):
            fail("artifactId order must be strictly bytewise ascending")
        if prior_path is not None and relative_path.encode("ascii") <= prior_path.encode("ascii"):
            fail("relativePath order must be strictly bytewise ascending")
        artifact_ids.add(artifact_id)
        relative_paths.add(relative_path)
        prior_id, prior_path = artifact_id, relative_path
        if manifest["bundleStatus"] != "TEMPLATE":
            if not checked_path.exists() or not checked_path.is_file() or checked_path.is_symlink():
                fail("artifact is not a regular non-symlink file: " + relative_path)
            if checked_path.stat().st_size != artifact["byteLength"]:
                fail("artifact byteLength mismatch: " + relative_path)
            if sha256_file(checked_path) != artifact["sha256"]:
                fail("artifact SHA-256 mismatch: " + relative_path)
        total += artifact["byteLength"]
    require_nonnegative_int(manifest["totalArtifactBytes"], "totalArtifactBytes", MAX_BYTES)
    if total != manifest["totalArtifactBytes"] or total > MAX_BYTES:
        fail("totalArtifactBytes mismatch or cap exceeded")
    require_string(manifest["bundleContentSha256"], "bundleContentSha256", SHA256)
    if manifest["bundleContentSha256"] != canonical_bundle_hash(manifest):
        fail("bundleContentSha256 must bind canonical metadata and artifact inventory")

    if manifest["bundleStatus"] == "TEMPLATE":
        pending_fields = (
            "createdAtUtc", "evidenceOwner", "operator", "reviewer", "targetMachineAlias",
            "redactionReference", "retentionReference", "accessReference", "locationReference",
            "allowlistStatus", "callTraceStatus", "environmentStatus", "observationsStatus",
            "reviewStatus", "gapsStatus",
        )
        if manifest["closureStatus"] != "PENDING" or any(manifest[field] != "PENDING" for field in pending_fields):
            fail("template metadata and closureStatus must remain PENDING")
        for binding in (manifest["repositoryBinding"], manifest["sourceBinding"], manifest["toolchainBinding"], manifest["buildBinding"]):
            if binding["status"] != "PENDING" or binding["reference"] != "PENDING":
                fail("template binding metadata must remain PENDING")
        if manifest["artifacts"] or manifest["totalArtifactBytes"] != 0:
            fail("template must contain no artifacts")


def self_test():
    """Adversarial parser/path tests; temporary files only, no product artifacts."""
    here = Path(__file__).resolve().parent
    template = here / "g1a-bundle-manifest.template.json"
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        manifest = load_json(template)
        manifest["bundleContentSha256"] = canonical_bundle_hash(manifest)
        valid = root / "valid.json"
        valid.write_text(json.dumps(manifest), encoding="utf-8")
        validate(load_json(valid), root)

        duplicate = root / "duplicate.json"
        duplicate.write_text('{"schemaVersion":"x","schemaVersion":"y"}', encoding="utf-8")
        try:
            load_json(duplicate)
        except ValueError:
            pass
        else:
            fail("self-test did not reject duplicate JSON keys")

        escaped = dict(manifest)
        escaped["bundleStatus"] = "EVIDENCE_COLLECTED"
        escaped["artifacts"] = [{"artifactId":"ART-001", "relativePath":"../outside.bin", "mediaType":"application/octet-stream", "byteLength":0, "sha256":hashlib.sha256(b"").hexdigest(), "truncation":False}]
        escaped["totalArtifactBytes"] = 0
        escaped["bundleContentSha256"] = canonical_bundle_hash(escaped)
        try:
            validate(escaped, root)
        except ValueError:
            pass
        else:
            fail("self-test did not reject path escape")

        symlink_target = root / "target"
        symlink_target.mkdir()
        symlink_root = root / "symlink-root"
        os.symlink(symlink_target, symlink_root, target_is_directory=True)
        try:
            validate(manifest, symlink_root)
        except ValueError:
            pass
        else:
            fail("self-test did not reject an artifact-root symlink")

        altered_metadata = dict(manifest)
        altered_metadata["bundleId"] = "G1A-ALTERED-METADATA-01"
        try:
            validate(altered_metadata, root)
        except ValueError:
            pass
        else:
            fail("self-test did not bind audit metadata into bundleContentSha256")
    print("self-test: pass")


def main():
    parser = argparse.ArgumentParser(description="Read-only G1A bundle manifest validator")
    parser.add_argument("manifest", type=Path, nargs="?")
    parser.add_argument("--artifact-root", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        if args.manifest is not None or args.artifact_root is not None:
            parser.error("--self-test does not take a manifest or artifact root")
        self_test()
        return
    if args.manifest is None or args.artifact_root is None:
        parser.error("manifest and --artifact-root are required")
    validate(load_json(args.manifest), args.artifact_root)
    print("valid: " + str(args.manifest))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        raise SystemExit("invalid: " + str(error))
