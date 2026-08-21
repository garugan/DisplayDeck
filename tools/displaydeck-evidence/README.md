# DisplayDeck evidence scaffold

This directory contains bounded, non-product evidence formats for the G1A review.
It is not application code, a Windows probe, a recovery implementation, or
authorization for Phase 2A or display mutation.

The G1A manifest validator is intentionally dependency-free and only validates
the evidence manifest and optionally hashes files that already exist. It never
creates, changes, truncates, or deletes files. The hard limits are:

- `phase2AAuthorized` must be `false`.
- `displayMutationAllowed` must be `false`.
- aggregate persisted artifact bytes must be at most 8 MiB (`8388608`);
- every formal artifact declares `truncation: false`. Oversize evidence makes
  the bundle invalid and belongs in a separately retained failed run.

The manifest also carries the minimum audit binding: creation time; evidence
owner/operator/reviewer; target-machine alias; redaction, retention, access,
and location references; repository/source/toolchain/build bindings; and typed
allowlist/call-trace/environment/observations/review/gaps statuses. A template
has all of those fields at `PENDING`, contains no artifacts, and cannot close
itself.

`bundleContentSha256` binds both that audit metadata and the complete artifact
inventory. Its canonical preimage is the manifest's canonical JSON object in
the required key order, compact ASCII JSON (`ensure_ascii`, no whitespace),
prefixed by `DisplayDeck.G1ABundleContent.V1` followed by one NUL byte. During
that calculation only, its own `bundleContentSha256` value is replaced with 64
lowercase ASCII `0` characters. This explicit zero-self convention avoids a
self-referential hash while preventing an artifact list or audit field from
being changed without invalidating the binding.

Validate a completed bundle from its containing directory:

```text
python3 validate_g1a_bundle.py path/to/g1a-bundle-manifest.json --artifact-root path/to/bundle
```

The validator rejects duplicate JSON keys, non-canonical IDs and paths,
out-of-order or duplicate artifact IDs/paths, path escape, artifact-root or
artifact symlinks, and values that violate the schema's critical type/pattern
constraints. It uses only the Python standard library.

Run its parser/path adversarial checks (temporary files only):

```text
python3 validate_g1a_bundle.py --self-test
```

`d07-no-go-predicate.template.json` is a deliberately fixed safety result, not
a partially completed probe. The authorized lane stopped before any Windows
write/probe because the directory-anchor algorithm is unproven. Validate that
it continues to state `UNPROVEN`, `NO_GO`, `NO_GO_RECORDED`, and zero for every
side effect:

```text
python3 validate_d07_no_go.py d07-no-go-predicate.template.json
python3 validate_d07_no_go.py --self-test
```

D07 remains a fail-closed No-Go unless a separately reviewed race-resistant
anchor algorithm exists. D08 is authorized only for bounded read-only Windows
capture; all acceptance thresholds remain `UNSET` until that capture and human
review occur.

Validate the D08 template and the static `BootIdV1` known answer together:

```text
python3 validate_d08_readonly_capture.py d08-readonly-capture.template.json
python3 validate_d08_readonly_capture.py --self-test
```

The D08 validator rejects duplicate JSON keys, non-canonical key ordering,
unbounded/raw non-ASCII values, threshold values other than `UNSET`, partial
captures, and any attempt to claim acceptance. It recomputes the static
preimage and SHA-256 with the exact `DisplayDeck.BootId.V1` domain plus one
NUL byte. A future Windows `CAPTURED` record remains evidence only: its result
must be one of the explicit rejection/absence-of-authority outcomes, never an
acceptance decision.

Create and validate one runtime sample from Windows PowerShell 5.1:

```powershell
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$capture = Join-Path $env:TEMP "displaydeck-d08-$stamp.json"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\displaydeck-evidence\capture_d08_readonly.ps1 -OutputPath $capture
py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py $capture
```

The helper reads only the two documented clock APIs and
`Win32_OperatingSystem`. It creates one new file, refuses overwrite, leaves all
thresholds `UNSET`, and never emits an acceptance result. Keep the raw capture
outside Git pending evidence-owner review.

Print the candidate `BootIdV1` digest for a validated runtime capture when
comparing pre/post reboot evidence:

```powershell
py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py --boot-id-only $capture
```

This is a diagnostic digest only. It does not issue same-boot authority or set
an acceptance threshold.
