# DD-FR-002 candidate freeze evidence tool

`dd_fr_002_freeze.py` is an offline Python-standard-library-only generator for
`DD-FR-002-WIRE-PROFILE-V1-CANDIDATE-04`. It is not DisplayDeck product code,
a Phase 2A serializer, a watchdog/worker, or a Windows API implementation.

It deterministically materializes full-byte candidate fixtures,
`semantic-manifest.jsonl`, the separately-scoped canonical artifact index,
approval-binding metadata, and SHA-256 checksums. Verification checks the
candidate artifact's deterministic bytes, catalogue, semantic manifest, index,
known-answer scope, and zero side-effect declarations. It does not certify a
future product parser or Windows runtime evidence.

`D04TUPV1` files are canonical JSON semantic-tuple witnesses, not duplicated
MachineActor wire files. Each carries an explicit canonical `tupleProjection`
and its SHA-256. A linked `MARV1` file is marked layout-only and demonstrates
only the binary envelope, slot/checksum, and record-state layout; its P/Q,
nonce, and owner fields are never treated as the D04 tuple source.

Run from the repository root:

```text
python3 tools/dd-fr-002-freeze/dd_fr_002_freeze.py generate
python3 tools/dd-fr-002-freeze/dd_fr_002_freeze.py verify
```

The generated material contains no host name, capture time, or absolute path.
D08 is restricted to a static `BootIdV1` preimage known-answer; runtime
tick/UTC tolerance remains `UNSET_EVIDENCE_PENDING`.
