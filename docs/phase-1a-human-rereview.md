# DisplayDeck Phase 1A Technical Revision V2 Independent Human Re-review

## 1. Review metadata

| Field | Value |
| --- | --- |
| Review date | 2026-08-08 |
| Review type | Independent technical re-review of Phase 1A Technical Revision V2 |
| Repository | `/Users/koichi/project/DisplayDeck` |
| Primary subject | `docs/phase-1a-execution-record.md` |
| Prior review | `docs/phase-1a-human-review.md` |
| Execution Record input SHA-256 | `64d738ca16cd78c0dd0e7c1008f69d9c6d4feed0e1c10fbc9e2b6db5cca4cf34` |
| Prior Human Review SHA-256 | `0c08e65c9b73fe0c4ccb585b488bad4f73a1c11989b5ef73d4f493e503d74836` |
| Human approval represented by this document | No |
| Reviewer/Approver decision entered | No; all remain `PENDING` |
| Cargo/source/build/test/static audit/API execution performed | No |
| Phase authorization represented by this document | No |

This document checks whether the technical findings in the prior review were resolved by Technical Revision V2. It is not an approval record, does not mark any API row or policy `APPROVED`, and does not authorize source creation, Cargo creation, compilation, static-audit execution, evidence collection, Windows API execution, or any phase.

Review methods were limited to document inspection, read-only repository/baseline checks, deterministic row/function/count checks, and official documentation review. No future audit evidence was simulated or treated as already collected.

Primary external references:

- [Microsoft GetLengthSid contract](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-getlengthsid)
- [Microsoft IsValidSid contract](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-isvalidsid)
- [Microsoft GetSidSubAuthorityCount contract](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-getsidsubauthoritycount)
- [`windows` 0.62.2 Cargo metadata](https://docs.rs/crate/windows/0.62.2/source/Cargo.toml.orig)
- [`windows` generated Win32 documentation](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/)

## 2. Prior review result

Prior final decision: `NEEDS_TECHNICAL_REVISION`.

The prior review did not reject Rust 1.97.1, `windows` 0.62.2, or the seven proposed Cargo features. It required technical correction before human decisions because SID validation, binding inventory, bypass controls, exact dependency auditing, PE import provenance, first-cell evidence scope, and four redaction fields were incomplete or inaccurate.

| Area | Previous issue | V2 result |
| --- | --- | --- |
| SID validity | missing | `VERIFIED_RESOLVED` |
| GetLengthSid contract | unsafe/inaccurate | `VERIFIED_RESOLVED` |
| windows binding inventory | incomplete | `COMPLETE` |
| manual FFI bypass | missing | `VERIFIED_RESOLVED` |
| dynamic resolution bypass | missing | `VERIFIED_RESOLVED` |
| build/link bypass | missing | `VERIFIED_RESOLVED` |
| dependency tree audit | incorrect authorization surface | `VERIFIED_RESOLVED` |
| source + PE audit | insufficient | `VERIFIED_RESOLVED` |
| PE provenance | insufficient | `VERIFIED_RESOLVED` |
| evidence scope | excessive | `VERIFIED_RESOLVED` |
| redaction 4 fields | change recommended | `VERIFIED_RESOLVED` |

## 3. V2 revision summary

| Artifact | Reviewed version | Review result | Human approval |
| --- | --- | --- | --- |
| API allowlist | `PHASE1A-ALLOWLIST-V2` | 38 unique proposed rows; internally consistent | `PENDING`; approved rows 0 |
| Forbidden-call policy | `PHASE1A-FORBIDDEN-V2` | Prior omissions closed without weakening V1 safety classes | `PENDING` |
| Static audit plan | `PHASE1A-STATIC-AUDIT-V2` | Exact authorized graph and source-plus-PE review defined | `PENDING`; all audits `NOT RUN` |
| PE import provenance | `PHASE1A-PE-IMPORT-PROVENANCE-V1` | Required classification and failure rules defined | `PENDING`; `NOT RUN` |
| Evidence plan | `PHASE1A-EVIDENCE-V2` | First-cell mandatory scope corrected | `PENDING` |
| Redaction policy | `PHASE1A-REDACTION-V2` | Four recommended changes incorporated | `PENDING` |

Technical Revision V2 adds one API row, `P1A-SEC-015`, to the existing 37 rows. It does not add another API family, Cargo feature, phase, implementation component, mutation path, or broader target cell. Policy expansions are audit and bypass controls, not executable scope.

## 4. SID safety closure

Decision: **`VERIFIED_RESOLVED`**.

### 4.1 Exact validation row

`P1A-SEC-015` exists as an exact `PROPOSED` row with:

- DLL: `Advapi32.dll`.
- Rust module: `windows::Win32::Security`.
- Function: `IsValidSid`.
- Input sources limited to successful, bounded SEC-003 `TokenUser`, SEC-004 `TokenGroups`, and SEC-008 `TokenIntegrityLevel` buffers.
- Null, external, arbitrary caller-provided, and out-of-buffer SID pointers forbidden.
- Pointer-in-containing-buffer and minimum SID-header range proof before the call.
- Retry 0 and `TIMEBOX-P1A-01`.
- Raw SID `DROP`; validation result `KEEP`; hashing permitted only after validation.

Microsoft documents that `IsValidSid` requires a non-null pointer, returns zero for an invalid SID, and has no extended error information. V2's rule therefore does not authorize a `GetLastError` call for `IsValidSid`.

### 4.2 GetLengthSid correction

SEC-010 now states that `GetLengthSid`:

- is called only after SEC-015 returned TRUE;
- obtains the bounded serialized length of an already validated SID;
- is not a validation API;
- must not be called with an invalid SID;
- must not use an invalid-SID return value as a validity test; and
- is followed by proof that the complete SID byte range fits in the containing token buffer.

This matches the Microsoft contract: `GetLengthSid` assumes a valid SID, and its result is undefined for an invalid SID.

### 4.3 Per-source chains

| Source | Required chain | V2 result |
| --- | --- | --- |
| SEC-003 `TokenUser` | successful bounded token result → structure/pointer/header bounds → SEC-015 → SEC-010 → complete range bounds → hash | Explicit; invalid optional SID becomes unavailable evidence and is not parsed or hashed |
| SEC-004 `TokenGroups` | checked group-count arithmetic → each structure/SID pointer/header bounds → SEC-015 → SEC-010 → complete range bounds → attributes → SEC-011 | Explicit; any invalid group SID makes the allowed privilege profile unprovable and fails closed |
| SEC-008 `TokenIntegrityLevel` | structure/pointer/header bounds → SEC-015 → SEC-010 → complete range bounds → SEC-012 → checked nonzero count → SEC-013 and returned-DWORD bounds | Explicit; invalid SID, zero count, or out-of-buffer result fails closed |

SEC-011/012/013 accept only validated, fully bounded SIDs. An invalid SID must not reach any of SEC-010/011/012/013. Pointer/handle/union escape is also a MUST item in Static Audit V2, so the eventual source must prove buffer lifetime and pointer confinement as an unsafe-block/FFI-boundary invariant before it can pass; no source exists yet and no such evidence is claimed here.

## 5. Allowlist V2 review

Decision: **`VERIFIED_CONSISTENT`**.

Read-only parsing of the call-contract table produced 38 row IDs and no duplicate ID.

| Classification | Count | Relationship to prior review |
| --- | ---: | --- |
| Required candidate | 23 | Prior 22 Required rows plus new P1A-SEC-015 |
| Optional candidate | 5 | Same five rows as prior review |
| Deferred / removed candidate | 10 | Same ten rows as prior review |
| Total | 38 | Existing 37 plus one technical SID-safety row |
| Human-approved | 0 | No row became approved |

The ten Deferred rows remain P1A-GDI-005/007, P1A-CCD-003/004/007/009, P1A-SES-001/002, and P1A-SYS-003/004. The first seven are future read-only exploratory candidates; SES-001/002 and direct SYS-004 remain first-cell removal candidates.

No scope creep was found. The only new callable row is the validation prerequisite requested by the prior review. Existing deferred rows remain visible for individual human decisions but are not silently moved into the first-cell mandatory set.

## 6. windows crate inventory review

Decision: **`COMPLETE`**.

The V2 inventory contains 21 distinct functions. For every entry it records function name, `windows` module, relevant V2 row IDs, first-cell classification, binding availability, and feature. The module/row/classification relationships agree with the allowlist:

| Module | Functions accounted for | Feature |
| --- | ---: | --- |
| `Win32::Graphics::Gdi` | 2 | `Win32_Graphics_Gdi` |
| `Win32::Devices::Display` | 3 | `Win32_Devices_Display` |
| `Win32::Security` | 7 | `Win32_Security` |
| `Win32::Foundation` | 2 | `Win32_Foundation` |
| `Win32::System::Threading` | 3 | `Win32_System_Threading` |
| `Win32::System::RemoteDesktop` | 1 | `Win32_System_RemoteDesktop` |
| `Win32::System::SystemInformation` | 3 | `Win32_System_SystemInformation` |
| Total | 21 | Seven exact features |

`IsValidSid` is present in `Win32::Security`, so its addition reuses `Win32_Security` and requires no eighth feature. The record correctly labels availability as documentation-level evidence rather than compile evidence. Compilation and resolved dependency evidence remain future gated work and were not performed during this re-review.

## 7. Cargo proposal review

Decision: **`VERIFIED_CONSISTENT`**; status remains `PROPOSED`, Human decision `PENDING`.

The document-only proposal is exactly:

```toml
windows = { version = "=0.62.2", default-features = true, features = [
  "Win32_Devices_Display",
  "Win32_Foundation",
  "Win32_Graphics_Gdi",
  "Win32_Security",
  "Win32_System_RemoteDesktop",
  "Win32_System_SystemInformation",
  "Win32_System_Threading",
] }
```

Official crate metadata identifies version 0.62.2, declares Rust 1.82 as its minimum Rust version, defines default as `["std"]`, and contains all seven feature names. The Rust 1.97.1 proposal is therefore not in version conflict. No unnecessary feature was added for SEC-015.

No `Cargo.toml`, `Cargo.lock`, Rust source, or root `build.rs` exists in the repository at this review point. No Cargo command or dependency resolution was run.

## 8. Forbidden V2 review

Decision: **`VERIFIED_RESOLVED`**.

### 8.1 Manual FFI and alternate bindings

V2 explicitly forbids manual `extern "system"`, manual `extern "stdcall"`, `#[link(...)]`, hand-written Win32 FFI, raw function-pointer tables, alternate Win32 binding crates, `windows-sys` bypass, bindgen/equivalent generation, and generated wrappers not tied to an individually approved row. Approved rows must use the reviewed `windows` 0.62.2 binding.

### 8.2 Dynamic resolution

V2 explicitly forbids GetModuleHandleA/W, GetModuleHandleExA/W, the LoadLibrary family, GetProcAddress, LdrLoadDll, LdrGetProcedureAddress, equivalent loader APIs, dynamic-loader crates such as libloading, manual PE export lookup, and delay/manual function resolution.

### 8.3 Build/link/code generation

V2 explicitly controls an unapproved root `build.rs`, custom linker wrapper, custom native library, `.cargo/config*`, rustflags, `-C link-arg`, extra `.lib`/`.dll` injection, custom linker scripts, proc-macro Win32-call generation, and code-generated unapproved FFI. Transitive build scripts require package/version/hash/purpose/emitted-output inventory and human review rather than silent acceptance.

### 8.4 V1 safety preservation

The V1 prohibitions remain: display mutation and mutation flags, registry/file persistence, profile writes, process spawn/termination/job control, named writable synchronization objects, recovery/WAL/lock operations, watchdog/worker/UI/Tauri/WebView work, shell/script hosts, elevation/token/identity mutation, scaling/HDR/orientation/profile/virtual-display mutation. Runtime provenance is not source authorization, and display-mutation imports are not exempted.

## 9. Static Audit V2 review

Decision: **`VERIFIED_RESOLVED`**. Execution status remains `NOT RUN` for every item.

The authorization-pass dependency graph is bound to committed `Cargo.toml`, committed `Cargo.lock`, exact approved default-feature decision, exact seven `windows` features, and exact `x86_64-pc-windows-msvc` target under the approved build conditions. The planned command is:

```text
cargo tree --locked -e features --target x86_64-pc-windows-msvc
```

`cargo tree --locked --all-features` is explicitly excluded from pass evidence. If separately authorized, it is labeled `NON_AUTHORIZATION_EXPLORATORY_AUDIT` and separated from the execution-artifact authorization evidence.

The plan contains 30 MUST audits covering:

- exact Win32 callsites, forbidden names, and mutation flags;
- unsafe blocks and FFI boundaries;
- manual extern/`#[link]` and alternate bindings;
- exact Cargo/default features;
- filesystem, registry, spawn, termination, named object, Tauri/WebView, and shell paths;
- dynamic loading, GetModuleHandle/Ldr families, and assembly;
- root and transitive build scripts, `.cargo/config*`, rustflags/link arguments, and proc macros;
- exact dependency graph, source hashes, and Cargo.lock review;
- final PE hash/import table/provenance; and
- source-to-binary binding.

Source scanning and final-PE import review are independently MUST and neither substitutes for the other. Byte-for-byte reproducibility is OPTIONAL, while exact source, lockfile, toolchain, target, feature set, and build command remain MUST evidence.

## 10. PE Import Provenance review

Decision: **`VERIFIED_RESOLVED`**.

`PHASE1A-PE-IMPORT-PROVENANCE-V1` defines all required classes:

- `APPROVED_DIRECT`
- `WINDOWS_BINDING_RUNTIME`
- `RUST_RUNTIME`
- `TRANSITIVE_DEPENDENCY`
- `TOOLCHAIN_RUNTIME`
- `UNEXPLAINED`

Any final execution-PE import with display-mutation capability, including ChangeDisplaySettings, SetDisplayConfig, DisplayConfigSetDeviceInfo, DDC/CI setters, or HDR/WCG/display setters, causes `STATIC_AUDIT_FAIL` regardless of provenance.

Another unexpected import such as runtime WriteFile or GetLastError is neither automatically allowed nor automatically rejected. The rule requires proof of the owner artifact, originating package/runtime, dependency path, absence of a direct source call, approved dataflow role, and lack of source reachability to a forbidden operation. Failure to prove all six produces `UNEXPLAINED` and `STATIC_AUDIT_FAIL`. This closes the generated/runtime-import gap without granting a silent waiver.

## 11. Evidence V2 review

Decision: **`VERIFIED_RESOLVED`**.

The first-cell MUST set retains the evidence necessary to bind authorization and execution:

- evidence/policy/authorization and approved machine identity;
- exact OS/KB/ESU, hardware, display, connection, console/session, and privilege cell;
- current resolution, rational refresh, orientation, and bpp;
- repository/source/manifest/lock/dependency/toolchain/build identity;
- API row, sequence, flags/query, result/error, retry/count/cap, elapsed/timebox trace;
- bounded stdout/stderr/exit evidence;
- GDI adapter/monitor/current/normal-mode results;
- GDI-to-CCD exact/ambiguous/unmapped relation and active CCD topology; and
- every exclusion/failure/cap/hotplug/access-denied and most-severe result.

The following are explicitly removed from first-cell mandatory results: registry/persisted mode, raw modes, database topology, database topology ID, preferred mode, advanced-color/HDR runtime-API result, PID export, and global UAC policy. Scale, token elevation type, scoped identity hashes, raw absolute tick, UTC correlation, and sanitized stderr body remain useful/restricted rather than mandatory.

The plan did not over-delete the primary identity or safety evidence. The 8 MiB output cap, enumeration caps, CCD allocation cap, token buffer cap, bounded retries, failure preservation, and no-label-similarity mapping rule remain intact. A manually recorded deferred environment fact must identify its collection method and cannot masquerade as approved runtime-API evidence.

## 12. Redaction V2 review

Decision: **`VERIFIED_RESOLVED`**.

| Field | Required V2 treatment | Observed V2 treatment |
| --- | --- | --- |
| Computer name | `DROP`; restricted scoped correlation only by Human decision | Match |
| GDI `DeviceString` | `HUMAN_REVIEW`; model-only may be kept, custom/serial/user-assigned content masked or dropped | Match |
| PID | `DROP`; normalize provenance to `CURRENT_PROCESS` | Match |
| Time fields | elapsed/exit `KEEP`; raw absolute tick restricted review/shared `DROP`; UTC review/coarsening | Match |

Bundle-local, domain-separated equality protection remains for user SID, authentication LUID, adapter LUID, GDI/monitor/device paths, and other device identity fields. Source/target IDs remain meaningful only with the hashed adapter LUID. The four changes therefore reduce unnecessary disclosure without breaking the GDI/CCD, user, or logon equality tests.

## 13. Authorization boundary

Decision: **`VERIFIED_INTACT`**.

| Item | Current state |
| --- | --- |
| Rust toolchain | `PROPOSED`; Human `PENDING` |
| `windows` crate | `PROPOSED`; Human `PENDING` |
| Cargo features/default feature | `PROPOSED`; Human `PENDING` |
| API rows | 38 `PROPOSED`; 0 approved |
| Forbidden policy | `READY_FOR_HUMAN_REVIEW`; Human `PENDING` |
| Static audit | `READY_FOR_HUMAN_REVIEW`; Human `PENDING`; all `NOT RUN` |
| PE provenance | `READY_FOR_HUMAN_REVIEW`; Human `PENDING`; `NOT RUN` |
| Evidence policy | `READY_FOR_HUMAN_REVIEW`; Human `PENDING` |
| Redaction policy | `READY_FOR_HUMAN_REVIEW`; Human `PENDING` |
| Authorization state | `NOT AUTHORIZED` |

Phase states remain:

| Phase | Status |
| --- | --- |
| Phase 1A record | `INCOMPLETE` |
| Phase 1A execution | `NOT EXECUTABLE` |
| Phase 2A | `NOT EXECUTABLE` |
| Phase 1B | `NOT EXECUTABLE` |
| Phase 2B | `NOT EXECUTABLE` |

`TECHNICAL_PREFLIGHT_COMPLETE` and `TECHNICAL_REVISION_V2_READY_FOR_REVIEW` describe technical document readiness only. They are not Human approval or execution permission.

## 14. Repository/Design baseline

Decision: **`VERIFIED_UNCHANGED`**.

| Baseline field | Verified value |
| --- | --- |
| Repository Baseline ID | `P1A-REPO-09be6a3` |
| Branch | `main` |
| Baseline HEAD | `09be6a3e05651b9587d526c2d57e542823ec9297` |
| Design Baseline SHA V1 | `d3764f3e0cafa9c7b0a89468e59b7452f627885483de90e90f68dedafabb015e` |

The current branch and HEAD match the frozen values. Recalculation of the 14-file Design Baseline V1 aggregate produced the same digest, and no Design Baseline file had a working-tree difference from HEAD.

At review input, `docs/phase-1a-execution-record.md` was modified and `docs/phase-1a-human-review.md` was untracked; neither was staged. This re-review adds only `docs/phase-1a-human-rereview.md` as another uncommitted review artifact. That working-tree state is not a re-review blocker and does not change the frozen repository/design baseline.

Before one byte of Phase 1A source is created, human inputs, per-row/policy decisions, and phase-specific authorization must be completed and the final authorization/Execution Record state must be committed to an immutable commit SHA. Later source/Cargo/lock/binary evidence must be frozen separately and must not be conflated with the authorization-record commit.

## 15. New findings

No new reportable technical finding was found.

| Severity | Count | Result |
| --- | ---: | --- |
| Critical | 0 | None |
| High | 0 | None |
| Medium | 0 | None |
| Low | 0 | None |

The new-issue search covered unsafe SID handling, token-buffer pointer provenance and lifetime, containing-buffer bounds, allowlist scope, Cargo feature consistency, manual/dynamic/build FFI bypasses, runtime import provenance, evidence/authorization binding, source-versus-binary auditing, and the Human approval boundary.

The following are pending verification gates, not current findings:

- The future implementation must keep each token buffer alive, unmoved, and unmodified for the complete SID validation/read sequence and record that invariant for every relevant unsafe block. V2 already requires buffer-rooted pointers, no raw-pointer escape, complete-range proof, and unsafe/FFI-boundary audit.
- Documentation-level binding availability is not compile proof. Exact manifest resolution and compilation remain separately authorized future evidence.
- Static Audit V2 and PE provenance remain `NOT RUN`; readiness of their policy does not assert a passing artifact.

These gates cannot be waived by this re-review.

## 16. Remaining human decisions

Technical revision closure leaves Human decisions as the next major gate. At minimum, humans must:

1. Complete the 52 primary machine, privilege, people, evidence, execution, and authorization inputs without inference.
2. Decide the exact target cell, including Windows build/KB/ESU acceptance, hardware/display/connection, console/session state, and effective privilege profile.
3. Approve or reject Rust 1.97.1, the x64 MSVC/minimal profile, rustfmt/Clippy policy, `windows` 0.62.2, default `std`, and all seven features.
4. Approve or reject each of the 38 API rows individually, the Required/Optional/Deferred first-cell selection, and rejected flag decisions; no wildcard approval.
5. Approve exact versions/hashes of Forbidden V2, Static Audit V2, PE Import Provenance V1, Evidence V2, and Redaction V2.
6. Decide evidence identity, location, retention, access principals, capture method, and the external procedure for a Win32 query that never returns.
7. Bind the final decisions to the approved Machine ID and immutable authorization/Execution Record commit SHA before source creation.
8. Issue separate phase-specific authorization; record completion alone is insufficient.

## 17. Final decision

### `READY_FOR_HUMAN_DECISIONS`

Basis:

- Critical = 0.
- High = 0.
- Medium = 0.
- Low = 0.
- Every technical revision required by the prior `NEEDS_TECHNICAL_REVISION` decision is resolved.
- The V2 policies are internally consistent and sufficiently specified for humans to approve, reject, or condition each proposal.
- Human approval, immutable authorization binding, future source/static-audit evidence, and phase-specific authorization remain outstanding and are not supplied by this document.

This decision advances the record only to Human decisions. Phase 1A record status remains `INCOMPLETE`, Phase 1A execution remains `NOT EXECUTABLE`, and no later phase is executable.

Stop point: creation of `docs/phase-1a-human-rereview.md`. Do not amend the Execution Record or prior Human Review, enter Human decisions, commit, create Cargo/source, build/test, run static audit, execute Windows APIs, collect target-machine evidence, or begin a phase under this re-review.
