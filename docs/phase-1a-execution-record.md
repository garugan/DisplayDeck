# DisplayDeck Phase 1A execution record

## 1. Record metadata

| Field | Value |
| --- | --- |
| Record purpose | Phase 1A Technical Preflight / Policy Freeze |
| Record schema | `PHASE1A-EXECUTION-RECORD-V1` |
| Policy preparation date | 2026-08-08 |
| Repository | `/Users/koichi/project/DisplayDeck` |
| Prepared scope | Read-only research and policy design only |
| Source/Cargo project/build/test/API execution performed | No |
| Human approval represented by this record | No |
| Technical preflight status | `TECHNICAL_PREFLIGHT_BLOCKED` |
| Blocking reason | The repository path is not currently recognized as a Git repository, so branch, full HEAD SHA, working tree, and index cannot be frozen. |
| Phase 1A record status | `INCOMPLETE` |
| Phase 1A execution | `NOT EXECUTABLE` |

Normative interpretation: `PROPOSED` and `READY_FOR_HUMAN_REVIEW` below are not approval. No row has human approval. This record does not authorize source creation, dependency installation, compilation, execution, display API calls, evidence collection, or any later phase.

## 2. Closure review

Source: [`docs/tauri-design-closure-review.md`](tauri-design-closure-review.md).

| Required condition | Observed result | Pass |
| --- | --- | --- |
| Final decision | `APPROVED_FOR_PHASE_1A_RECORD_COMPLETION` | Yes |
| DD-FR-001 | `VERIFIED_RESOLVED` | Yes |
| DD-FR-002 | `VERIFIED_PRE_PHASE_2A_FREEZE_CONDITION` | Yes |
| Critical | 0 | Yes |
| High | 0 | Yes |
| Medium | 0 | Yes |

The closure review permits completion of this record only. It explicitly does not authorize Phase 1A execution and leaves Phase 1A, Phase 2A, Phase 1B, and Phase 2B unapproved and unstarted. DD-CR-001 remains Low and is a pre-Phase 2A freeze condition, not a Phase 1A record blocker.

## 3. Human inputs

Existing human values were not present. No value in this table was inferred.

| Field | Value |
| --- | --- |
| Target Machine ID | `PENDING_HUMAN_INPUT` |
| Execution location | `PENDING_HUMAN_INPUT` |
| Phase 1A evidence ID | `PENDING_HUMAN_INPUT` |
| Evidence location | `PENDING_HUMAN_INPUT` |
| Evidence retention period | `PENDING_HUMAN_INPUT` |
| Evidence access principals | `PENDING_HUMAN_INPUT` |
| Operator | `PENDING_HUMAN_INPUT` |
| Evidence Owner | `PENDING_HUMAN_INPUT` |
| Reviewer | `PENDING_HUMAN_INPUT` |
| Approver | `PENDING_HUMAN_INPUT` |
| Planned execution date | `PENDING_HUMAN_INPUT` |
| Approval ID | `PENDING_HUMAN_INPUT` |
| Signature / record reference | `PENDING_HUMAN_INPUT` |

## 4. Target Machine

This is a one-cell record. All exact values other than the supplied ESU observation remain human or machine inputs and must be frozen before authorization.

| Field | Value |
| --- | --- |
| Machine ID | `PENDING_HUMAN_INPUT` |
| Windows edition | `PENDING_HUMAN_INPUT` |
| Windows version | `PENDING_HUMAN_INPUT` |
| OS build | `PENDING_HUMAN_INPUT` |
| Installed KB list and evidence | `PENDING_HUMAN_INPUT` |
| ESU status | `NOT CONFIRMED（登録済み表示なし）` |
| Approve this non-ESU-confirmed environment as the Phase 1A exploration cell | `PENDING_HUMAN_INPUT` |
| CPU architecture | `PENDING_HUMAN_INPUT` (planned target: x64) |
| GPU / exact model | `PENDING_HUMAN_INPUT` |
| GPU driver | `PENDING_HUMAN_INPUT` |
| Display / firmware | `PENDING_HUMAN_INPUT` |
| Connection | `PENDING_HUMAN_INPUT` |
| Physical port | `PENDING_HUMAN_INPUT`; allowed evidence value: `GPU-side port number: NOT EXPOSED` |
| Dock / adapter | `PENDING_HUMAN_INPUT` |
| Current resolution / refresh | `PENDING_MACHINE_OBSERVATION` |
| HDR / advanced color / scale | `PENDING_MACHINE_OBSERVATION` |
| Local console / RDP | `PENDING_MACHINE_OBSERVATION` |
| Multi-user / Fast User Switching state | `PENDING_MACHINE_OBSERVATION` |

OS edition/version/build/KB and ESU are environment evidence, not feature-presence authority. Phase 1A source must not use registry writes, process spawning, PowerShell, `cmd.exe`, or an undocumented version query. This record proposes only the Win32 rows below; any automated WMI/CIM or additional version/KB call requires a new row and human review. `Win32_QuickFixEngineering` is not treated as a complete update inventory because Microsoft documents that it returns only CBS-supplied updates.

## 5. Repository Baseline

Read-only commands attempted on 2026-08-08:

```text
git rev-parse --show-toplevel
git branch --show-current
git rev-parse HEAD
git status --porcelain=v1 --untracked-files=all
git status --short --branch
git diff --name-only
git diff --cached --name-only
```

All Git repository queries failed with `fatal: not a git repository (or any of the parent directories): .git`.

| Field | Value |
| --- | --- |
| Repository root (requested path) | `/Users/koichi/project/DisplayDeck` |
| Git repository root | `UNAVAILABLE` |
| Branch | `UNAVAILABLE` |
| HEAD full SHA | `UNAVAILABLE` |
| Working tree clean | `UNKNOWN` |
| Index clean | `UNKNOWN` |
| Untracked files | `UNKNOWN` |
| Design documents modified relative to HEAD | `UNKNOWN` |
| Phase 1A code and design fixed to the same commit | No |
| Repository baseline status | `NOT_FREEZABLE` |

Blocker resolution is human-owned: restore or initialize the intended Git metadata outside this record, place the approved design and future Phase 1A source on a human-selected branch/commit, make the working tree and index clean, and then rerun this read-only baseline capture. This record does not authorize `git init`, add, commit, checkout, switch, reset, clean, stash, branch, tag, or push.

## 6. Design Baseline SHA V1

Algorithm `Design Baseline V1`:

1. Sort the relative paths below in ascending UTF-8 byte order.
2. Hash each file's raw bytes with SHA-256 without newline or content normalization.
3. Build an in-memory UTF-8/LF manifest with one line `<sha256><two spaces><relative-path>\n` per file.
4. SHA-256 hash the manifest bytes.

No manifest file was created in the repository.

| Relative path | SHA-256 |
| --- | --- |
| `AGENTS.md` | `2a90b1316a219b9e1aa341938c243ab2a0030dda3fb5c80d58064a9328dc9f60` |
| `docs/architecture.md` | `ba0d8ce80d86f6b4bca13435f2c60667cc69dbb92d2d98889adffdba1b4f4ab4` |
| `docs/implementation-plan.md` | `bd4d8fed8193428d135ad9d4b544dc272a9ffce2fc3699f62a6b3f8d9ec8bfad` |
| `docs/requirements.md` | `4d9b5228fd8d00485116862264d39db97669f78dc555a419f1c8a872b224273a` |
| `docs/risks-and-open-questions.md` | `df4af2273cc7014ab44b5718c05b0a97a9fc652441ec5bed8348c6f3362854c2` |
| `docs/security.md` | `0e5cec4a73504795ba83c97167bb63dc9a1653739961db45499039325050b931` |
| `docs/tauri-design-closure-review.md` | `54df76d047f0ccef70098fd089105abcaa94ab9648ba73aa580dbbcaf5d1804d` |
| `docs/tauri-design-final-review.md` | `3d84be049bd6260c6134adc2de74dfc79db0492c35db68336bb9ca73b4fadc78` |
| `docs/tauri-design-review-checklist.md` | `d2c126c444f6121ef2cceca61aa060c18754aa064649660b0fe28b3ebd74d9c4` |
| `docs/tauri-migration.md` | `6a0a9c4d856af79f89f85fb11daa72af8d2baf4ed346d9161f99c359d364581a` |
| `docs/tauri-review-resolution.md` | `11e91d6f3e87b364cf34d01891fa63a78de84ce0886cae7109a92f4df6cf9db0` |
| `docs/testing-strategy.md` | `cd92e8538d2a836c16afff2563146f945bd2faf73810f254db3ff5a55c9b22ba` |
| `docs/ui-design.md` | `1af61f8cd773881445dc017a95076907b56d97d984d99ec669429017957bd362` |
| `docs/windows-display-research.md` | `f1ec53ca946e11edef7bb959e9ae9d070335aaad34dafa86eba1cff8ee8c9905` |

**Aggregate Design Baseline SHA V1:** `d3764f3e0cafa9c7b0a89468e59b7452f627885483de90e90f68dedafabb015e`

Digest state: `CALCULATED_NOT_FROZEN`. The digest is reproducible for the observed files, but it cannot be tied to a branch/full commit SHA until the repository blocker is resolved.

## 7. Toolchain Freeze

| Field | Proposed value | Basis / condition |
| --- | --- | --- |
| Rust channel | `stable` | Phase 1A needs no nightly-only feature. |
| rustc exact version | `1.97.1` | Rust Release Team published the 1.97.1 stable point release on 2026-07-16. |
| Cargo exact version | `PENDING_MACHINE_VERIFICATION` | The official 1.97.1 distribution contains the Cargo component and an x64 MSVC artifact named `cargo-1.97.1`; exact `cargo --version --verbose` output must be captured on the target machine. The manifest's Cargo package source version is `0.98.0 (c980f4866 2026-06-30)`, which is not substituted for machine CLI evidence. |
| Target triple | `x86_64-pc-windows-msvc` | Rust documents this as Tier 1 with host tools and the 1.97.1 distribution marks the target available. |
| rustup profile | `minimal` | Limits installation to rustc, rust-std, and Cargo; docs and unrelated targets are unnecessary. |
| Required profile components | `rustc`, `rust-std`, `cargo` | Compile/link prerequisites after separate authorization. |
| Additional component candidates | `rustfmt`, `clippy` | Proposed for formatting and lint evidence; availability and exact versions must be verified without installing/updating in this preflight. |
| Excluded components | nightly, beta, `rust-src`, `rust-docs`, Miri, LLVM tools | Not required for Phase 1A's approved future read-only scope. |
| Nightly features | None | All proposed language/library/API-binding work is expected to compile on stable; a nightly requirement returns to review. |
| Repository-design consistency | Yes: Windows-only Rust FFI boundary, no Tauri/React/watchdog/worker in Phase 1A. | Matches the fixed technology boundary and Phase 1A read-only scope. |
| Status | `PROPOSED` | Target-machine tool output and human approval are pending. |

Primary official evidence:

- [Rust 1.97.1 release announcement](https://blog.rust-lang.org/2026/07/16/Rust-1.97.1/)
- [Rust stable release index](https://blog.rust-lang.org/releases/)
- [Rust 1.97.1 distribution manifest](https://static.rust-lang.org/dist/channel-rust-1.97.1.toml)
- [Rust Windows MSVC target support](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)

Freeze gate: run no installer or toolchain command under this record. After repository freeze and human authorization, the target-machine record must capture `rustup show active-toolchain`, `rustc --version --verbose`, `cargo --version --verbose`, installed components, installed targets, and MSVC/Windows SDK identity. Until then, the toolchain is not `FROZEN`.

## 8. windows crate / Cargo Features

### 8.1 Crate candidate

| Field | Value |
| --- | --- |
| Crate | `windows` |
| Exact candidate version | `0.62.2` |
| Publication evidence | [crates.io](https://crates.io/crates/windows/0.62.2), [docs.rs release record](https://docs.rs/crate/windows/0.62.2), [Microsoft generated Rust docs](https://microsoft.github.io/windows-docs-rs/doc/windows/) |
| Target docs | [docs.rs x86_64-pc-windows-msvc](https://docs.rs/windows/0.62.2/x86_64-pc-windows-msvc/windows/) |
| API path availability | All nine requested functions are present in 0.62.2 at the paths below. |
| Exact dependency resolution / lock hash | `PENDING`; no `Cargo.toml` or `Cargo.lock` exists or was created. |
| Status | `PROPOSED` |

| Function | `windows` 0.62.2 module |
| --- | --- |
| `EnumDisplayDevicesW` | `windows::Win32::Graphics::Gdi` |
| `EnumDisplaySettingsExW` | `windows::Win32::Graphics::Gdi` |
| `GetDisplayConfigBufferSizes` | `windows::Win32::Devices::Display` |
| `QueryDisplayConfig` | `windows::Win32::Devices::Display` |
| `DisplayConfigGetDeviceInfo` | `windows::Win32::Devices::Display` |
| `OpenProcessToken` | `windows::Win32::Security` |
| `GetTokenInformation` | `windows::Win32::Security` |
| `ProcessIdToSessionId` | `windows::Win32::System::Threading` |
| `WTSGetActiveConsoleSessionId` | `windows::Win32::System::RemoteDesktop` |

### 8.2 Cargo feature proposal

Exact proposal (ordering is normative for the review artifact only):

```text
Win32_Devices_Display
Win32_Foundation
Win32_Graphics_Gdi
Win32_Security
Win32_System_RemoteDesktop
Win32_System_SystemInformation
Win32_System_Threading
```

No default-feature decision is approved here. The future `Cargo.toml` row must explicitly state whether `default-features` is disabled and must be resolved under human review.

| Feature | Required API/module and reason | Phase 1A use | Dangerous sibling examples exposed by the feature | Static-audit treatment |
| --- | --- | --- | --- | --- |
| `Win32_Devices_Display` | CCD query functions and packet types in `Win32::Devices::Display` | Buffer sizing, active/database topology, names, preferred mode, advanced color observation | `SetDisplayConfig`, `DisplayConfigSetDeviceInfo`, monitor-setting functions and `SDC_*` constants | Only named approved callsites; explicit forbidden-name/flag/import scans. |
| `Win32_Foundation` | `HANDLE`, `BOOL`, `WIN32_ERROR`, `LUID`, `CloseHandle`, `GetLastError` | Typed return/error values, immediate documented error capture, and owned token-handle close | Broad foundational handles/errors; misuse can close unrelated handles or read stale last-error | Handle provenance review; only the token handle from SEC-002 may reach SEC-014; SYS-004 is permitted only immediately after a documented failing call. |
| `Win32_Graphics_Gdi` | GDI enumeration and `DEVMODEW`/`DISPLAY_DEVICEW`; also required by generated CCD types | Adapter/monitor/current/registry/compatible/raw observation | `ChangeDisplaySettingsW/ExW`, `CDS_*`, many graphics mutation calls | Exact callsite allowlist plus mutation-flag and binary-import audit. |
| `Win32_Security` | Current-token query and SID helpers | User SID hash, admin group membership, elevation, integrity, session | `SetTokenInformation`, privilege/group adjustment, ACL/security-descriptor mutation | Query-class allowlist; reject every setter/adjuster/ACL call. |
| `Win32_System_RemoteDesktop` | `WTSGetActiveConsoleSessionId` | Compare the process session with the physical-console session | Session control, disconnect/logoff/send-message APIs | Only the one approved getter may be imported/called. |
| `Win32_System_SystemInformation` | `GetNativeSystemInfo`, `GetTickCount64`, `GetSystemTimePreciseAsFileTime` | Architecture and read-only clock evidence | Time/computer-name setter functions in the same module | Ban all `Set*` system/time/computer-name calls; exact getter import list. |
| `Win32_System_Threading` | `GetCurrentProcess`, `GetCurrentProcessId`, `ProcessIdToSessionId` | Bind token/session checks to the current process only | `CreateProcess*`, `TerminateProcess`, named mutex/event/semaphore, Job Object mutation | Spawn/termination/named-object/Job Object name scan and binary-import review. |

Feature status: `PROPOSED`. A feature exposing a dangerous sibling is not by itself a violation; a Phase 1A source callsite, re-export, dynamic lookup, or binary import outside the approved set is a violation.

## 9. Exact API Allowlist

Allowlist artifact: `PHASE1A-ALLOWLIST-V1`  
Artifact status: `READY_FOR_HUMAN_REVIEW`  
Proposed rows: **37**  
Human-approved rows: **0**

### 9.1 Common rules

- Every row status is `PROPOSED`; Reviewer, Approver, and Approval state are `PENDING_HUMAN_INPUT` / `PENDING_HUMAN_REVIEW`.
- A complete row is the join of its ID in 9.2 with the applicable 9.1 common rules and 9.3 evidence/governance entry. Ranges in 9.3 repeat the same field values for each individual ID; they are not family-level approval.
- Every display identifier, mode token, LUID, source ID, target ID, and enumeration result is session-scoped evidence, never durable authority.
- Enumeration arithmetic uses checked conversion/multiplication. Exceeding a cap is `BOUND_EXCEEDED`, not permission to truncate and continue.
- `DEVMODEW.dmSize = size_of::<DEVMODEW>()` and `dmDriverExtra = 0`; no driver-private bytes are collected.
- `TIMEBOX-P1A-01`: calls are synchronous and have no documented per-call cancellation. Timestamp immediately before/after each call. A returned call exceeding 5,000 ms marks the row `LATENCY_BUDGET_EXCEEDED` and stops later calls. Enumeration families also stop between calls after 10,000 ms. The source must not spawn or terminate a process to enforce timeout. A call that never returns is a failed/hung run requiring a separately approved external incident procedure; this record does not misstate that risk as enforceable cancellation.
- CCD allocation cap: path count <= 64, mode count <= 256, combined checked allocation <= 1 MiB. `ERROR_INSUFFICIENT_BUFFER` permits at most two complete size/query retries (three attempts total) and only after re-running the matching size row.
- Token variable-buffer cap: 65,536 bytes. One null/zero size probe may return `ERROR_INSUFFICIENT_BUFFER`, followed by one bounded data call; no third allocation retry.
- Evidence output is bounded UTF-8 JSON to stdout only, maximum 8 MiB. Phase 1A source has no file-persistence API.

### 9.2 Call contract rows

| ID | DLL / Rust module / function | Exact purpose and input | Exact flag / access | Buffer and retry cap | Expected / accepted result | Timeout / abort |
| --- | --- | --- | --- | --- | --- | --- |
| P1A-GDI-001 | User32.dll; `Win32::Graphics::Gdi`; `EnumDisplayDevicesW` | Adapter enumeration; `lpDevice=NULL`, consecutive `iDevNum=0..31`, initialized `DISPLAY_DEVICEW` | `dwFlags=0`; no handle/right | 1 fixed struct/call; max 32; retry 0 | nonzero=row; zero=end including index 0; no guessing from stale last-error | `TIMEBOX-P1A-01` |
| P1A-GDI-002 | User32.dll; same; `EnumDisplayDevicesW` | Monitor enumeration for one saved adapter `DeviceName`; `iDevNum=0..31` per adapter | `dwFlags=0`; no handle/right | 1 fixed struct; max 32/adapter and 1,024 total; retry 0 | nonzero=row; zero=end | `TIMEBOX-P1A-01` |
| P1A-GDI-003 | User32.dll; same; `EnumDisplayDevicesW` | Repeat monitor enumeration only to obtain monitor device-interface identity for cross-map evidence | `EDD_GET_DEVICE_INTERFACE_NAME`; no handle/right | Same caps as GDI-002; retry 0 | nonzero=row with `DeviceID`; zero=end | `TIMEBOX-P1A-01` |
| P1A-GDI-004 | User32.dll; `Win32::Graphics::Gdi`; `EnumDisplaySettingsExW` | Current mode for a GDI adapter `DeviceName` from GDI-001 | `iModeNum=ENUM_CURRENT_SETTINGS`, `dwFlags=0` | fixed `DEVMODEW`, `dmDriverExtra=0`; retry 0 | nonzero=success; zero=unsupported/failure and cell classification | `TIMEBOX-P1A-01` |
| P1A-GDI-005 | User32.dll; same; `EnumDisplaySettingsExW` | Registry/persisted mode for same adapter | `ENUM_REGISTRY_SETTINGS`, flags 0 | same; retry 0 | nonzero=success; zero=missing/failure evidence | `TIMEBOX-P1A-01` |
| P1A-GDI-006 | User32.dll; same; `EnumDisplaySettingsExW` | Compatible indexed mode enumeration | `iModeNum=0..4095`, flags 0 | 1 fixed struct/call; max 4,096/adapter; retry 0 | nonzero=row; zero=end; index-0 zero is empty/failure evidence | `TIMEBOX-P1A-01` |
| P1A-GDI-007 | User32.dll; same; `EnumDisplaySettingsExW` | Diagnostic comparison of driver-reported raw modes required by current P1A research/test design | `iModeNum=0..4095`, `EDS_RAWMODE` only | same cap; retry 0 | nonzero=row; zero=end; raw rows never become product candidates by this record | `TIMEBOX-P1A-01` |
| P1A-CCD-001 | User32.dll; `Win32::Devices::Display`; `GetDisplayConfigBufferSizes` | Size active-path query buffers for Windows 10 initial cell | `QDC_ONLY_ACTIVE_PATHS \| QDC_VIRTUAL_MODE_AWARE`; no right | counts only; caps above; part of max 3 pair attempts | `ERROR_SUCCESS`; `ACCESS_DENIED/NOT_SUPPORTED/GEN_FAILURE/INVALID_PARAMETER` recorded and row stops | `TIMEBOX-P1A-01` |
| P1A-CCD-002 | User32.dll; same; `QueryDisplayConfig` | Obtain current active topology/modes using counts from CCD-001; `currentTopologyId=NULL` | exact same flags as CCD-001 | paths<=64, modes<=256, <=1 MiB; only `ERROR_INSUFFICIENT_BUFFER` triggers paired retry, max 2 retries | `ERROR_SUCCESS`; documented errors recorded; access/not-supported blocks cell, insufficient buffer retries | `TIMEBOX-P1A-01` |
| P1A-CCD-003 | User32.dll; same; `GetDisplayConfigBufferSizes` | Size persistence-database query for currently connected monitors | `QDC_DATABASE_CURRENT \| QDC_VIRTUAL_MODE_AWARE` | same caps/pair retry | same documented results as CCD-001 | `TIMEBOX-P1A-01` |
| P1A-CCD-004 | User32.dll; same; `QueryDisplayConfig` | Obtain CCD database topology; non-null `currentTopologyId` | exact same flags as CCD-003 | same caps and paired retry | same as CCD-002; missing database details remain explicit, never synthesized | `TIMEBOX-P1A-01` |
| P1A-CCD-005 | User32.dll; `Win32::Devices::Display`; `DisplayConfigGetDeviceInfo` | Source name for each exact `(adapterId, sourceId)` from CCD-002/004 | `DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME` | fixed packet; <=64 unique sources; retry 0 | success; documented invalid/not-supported/access/buffer/gen errors recorded | `TIMEBOX-P1A-01` |
| P1A-CCD-006 | User32.dll; same; `DisplayConfigGetDeviceInfo` | Target monitor name/path for exact `(adapterId,targetId)` | `...GET_TARGET_NAME` | fixed packet; <=64 targets; retry 0 | same | `TIMEBOX-P1A-01` |
| P1A-CCD-007 | User32.dll; same; `DisplayConfigGetDeviceInfo` | Preferred target mode for exact target | `...GET_TARGET_PREFERRED_MODE` | fixed packet; <=64; retry 0 | same; unavailable preferred mode is explicit | `TIMEBOX-P1A-01` |
| P1A-CCD-008 | User32.dll; same; `DisplayConfigGetDeviceInfo` | Adapter device path for cross-map evidence | `...GET_ADAPTER_NAME` | fixed packet; <=16 unique adapters; retry 0 | same | `TIMEBOX-P1A-01` |
| P1A-CCD-009 | User32.dll; same; `DisplayConfigGetDeviceInfo` | Read advanced-color capability/state and bits per channel for exact active target | `...GET_ADVANCED_COLOR_INFO` | fixed packet; <=64; retry 0 | success; `ERROR_NOT_SUPPORTED` is a supported observation but disqualifies claims; access/invalid/buffer/gen recorded | `TIMEBOX-P1A-01` |
| P1A-SEC-001 | Kernel32.dll; `Win32::System::Threading`; `GetCurrentProcess` | Obtain current-process pseudo-handle only | no flags; no requested right; never accepts another PID/handle | none; retry 0 | non-null documented pseudo-handle | `TIMEBOX-P1A-01` |
| P1A-SEC-002 | Advapi32.dll; `Win32::Security`; `OpenProcessToken` | Open only SEC-001 current process token | `TOKEN_QUERY` only | one owned handle; retry 0 | nonzero; failure/GetLastError is terminal for privilege collection | `TIMEBOX-P1A-01` |
| P1A-SEC-003 | Advapi32.dll; same; `GetTokenInformation` | Current token user SID | `TokenUser`; SEC-002 token | variable buffer <=65,536; one probe+one fill | probe `ERROR_INSUFFICIENT_BUFFER`, then success; all else terminal | `TIMEBOX-P1A-01` |
| P1A-SEC-004 | Advapi32.dll; same; `GetTokenInformation` | Token groups to identify built-in Administrators membership/deny-only state | `TokenGroups`; SEC-002 token | <=65,536 and <=2,048 groups; one probe+fill | same | `TIMEBOX-P1A-01` |
| P1A-SEC-005 | Advapi32.dll; same; `GetTokenInformation` | Process elevated boolean | `TokenElevation` | exact structure size; retry 0 | success only | `TIMEBOX-P1A-01` |
| P1A-SEC-006 | Advapi32.dll; same; `GetTokenInformation` | UAC token elevation type | `TokenElevationType` | exact enum size; retry 0 | success only | `TIMEBOX-P1A-01` |
| P1A-SEC-007 | Advapi32.dll; same; `GetTokenInformation` | Token Windows session ID | `TokenSessionId` | exact DWORD size; retry 0 | success only | `TIMEBOX-P1A-01` |
| P1A-SEC-008 | Advapi32.dll; same; `GetTokenInformation` | Integrity-level SID | `TokenIntegrityLevel` | <=65,536; one probe+fill | probe insufficient buffer, then success | `TIMEBOX-P1A-01` |
| P1A-SEC-009 | Advapi32.dll; same; `GetTokenInformation` | Current logon authentication LUID for same-logon equality evidence | `TokenStatistics`; read `TOKEN_STATISTICS.AuthenticationId` only | exact `TOKEN_STATISTICS` size; retry 0 | success only; zero/unknown authentication ID is preserved, not replaced | `TIMEBOX-P1A-01` |
| P1A-SEC-010 | Advapi32.dll; `Win32::Security`; `GetLengthSid` | Bound the OS-returned user/group/integrity SID before hashing/inspection | only SID pointers rooted in SEC-003/004/008 buffers | max returned buffer; retry 0 | positive length <= containing buffer; zero/error terminal | `TIMEBOX-P1A-01` |
| P1A-SEC-011 | Advapi32.dll; same; `IsWellKnownSid` | Compare SEC-004 group SIDs with `WinBuiltinAdministratorsSid` only | exact well-known SID type | <=2,048 calls; retry 0 | true=match; false=non-match for OS-returned bounded SID | `TIMEBOX-P1A-01` |
| P1A-SEC-012 | Advapi32.dll; same; `GetSidSubAuthorityCount` | Read bounded integrity SID subauthority count | SEC-008 SID only | one byte pointer within validated SID; retry 0 | non-null and count>=1; otherwise terminal | `TIMEBOX-P1A-01` |
| P1A-SEC-013 | Advapi32.dll; same; `GetSidSubAuthority` | Read final integrity RID only | index=`count-1` from SEC-012 | one DWORD pointer within validated SID; retry 0 | non-null; map documented integrity RIDs, unknown remains `UNKNOWN` | `TIMEBOX-P1A-01` |
| P1A-SEC-014 | Kernel32.dll; `Win32::Foundation`; `CloseHandle` | Close exactly the owned token handle from SEC-002 once | no flags/right; pseudo-handle SEC-001 forbidden | one handle; retry 0 | nonzero success; zero recorded as resource-cleanup failure | `TIMEBOX-P1A-01` |
| P1A-SES-001 | Kernel32.dll; `Win32::System::Threading`; `GetCurrentProcessId` | Current PID for same-process session query | no input/flags | none; retry 0 | nonzero PID | `TIMEBOX-P1A-01` |
| P1A-SES-002 | Kernel32.dll; same; `ProcessIdToSessionId` | Resolve only SES-001 PID to Windows session | current PID only; no opened foreign-process handle | one DWORD; retry 0 | nonzero call result; failure/GetLastError terminal | `TIMEBOX-P1A-01` |
| P1A-SES-003 | Kernel32.dll; `Win32::System::RemoteDesktop`; `WTSGetActiveConsoleSessionId` | Obtain current physical-console session ID | no input/flags | one DWORD; retry 0 | ID; `0xFFFFFFFF` accepted as transient no-console observation and blocks run | `TIMEBOX-P1A-01` |
| P1A-SYS-001 | Kernel32.dll; `Win32::System::SystemInformation`; `GetNativeSystemInfo` | Native CPU architecture evidence | initialized `SYSTEM_INFO`; no flags | fixed struct; retry 0 | populated recognized architecture; unknown remains unknown | `TIMEBOX-P1A-01` |
| P1A-SYS-002 | Kernel32.dll; same; `GetTickCount64` | Same-boot monotonic tick observation; no deadline authority in Phase 1A | no input/flags | one u64; retry 0 | any u64; record raw value only in restricted evidence | `TIMEBOX-P1A-01` |
| P1A-SYS-003 | Kernel32.dll; same; `GetSystemTimePreciseAsFileTime` | Wall-clock diagnostic paired with SYS-002; never substitutes for live deadline | initialized `FILETIME`; no flags | fixed struct; retry 0 | populated FILETIME | `TIMEBOX-P1A-01` |
| P1A-SYS-004 | Kernel32.dll; `Win32::Foundation`; `GetLastError` | Capture extended error only immediately after an allowlisted call whose Microsoft contract directs the caller to it | no input/flags; never used for GDI enumeration end unless the API contract supplies a reliable distinction | one thread-local DWORD; at most one read per failing call; retry 0 | any code recorded with originating row; stale/unattributed reads forbidden | `TIMEBOX-P1A-01` |

### 9.3 Evidence, redaction, forbidden sibling, and governance per row

Documentation keys:

- `MS-EDD`: [EnumDisplayDevicesW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaydevicesw); `RS-EDD`: [windows 0.62.2 generated docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Graphics/Gdi/fn.EnumDisplayDevicesW.html)
- `MS-EDS`: [EnumDisplaySettingsExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsexw); `RS-EDS`: [generated docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Graphics/Gdi/fn.EnumDisplaySettingsExW.html)
- `MS-GBS`: [GetDisplayConfigBufferSizes](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdisplayconfigbuffersizes); `RS-GBS`: [generated docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Devices/Display/fn.GetDisplayConfigBufferSizes.html)
- `MS-QDC`: [QueryDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig); `RS-QDC`: [generated docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Devices/Display/fn.QueryDisplayConfig.html)
- `MS-DGI`: [DisplayConfigGetDeviceInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-displayconfiggetdeviceinfo); `RS-DGI`: [generated docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Devices/Display/fn.DisplayConfigGetDeviceInfo.html)
- `MS-OPT`: [OpenProcessToken](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocesstoken); `MS-GTI`: [GetTokenInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation); `MS-TIC`: [TOKEN_INFORMATION_CLASS](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ne-winnt-token_information_class); `RS-SEC`: [generated Security module](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Security/index.html)
- `MS-P2S`: [ProcessIdToSessionId](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-processidtosessionid); `RS-THR`: [generated Threading module](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Threading/index.html)
- `MS-WTS`: [WTSGetActiveConsoleSessionId](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-wtsgetactiveconsolesessionid); `RS-WTS`: [generated docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/RemoteDesktop/fn.WTSGetActiveConsoleSessionId.html)
- `MS-TICK`: [GetTickCount64](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64); `MS-TIME`: [GetSystemTimePreciseAsFileTime](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemtimepreciseasfiletime); `RS-SYS`: [generated SystemInformation module](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/SystemInformation/index.html)
- `MS-GLE`: [GetLastError](https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror); `RS-FND`: [generated Foundation module](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Foundation/index.html)

| ID range / row | Evidence fields | Redaction | Forbidden sibling examples | Docs | Reviewer / Approver / Approval |
| --- | --- | --- | --- | --- | --- |
| GDI-001 | index, state flags, primary/desktop state, `DeviceName/String/ID/Key` | policy table per field | `ChangeDisplaySettings*`, `CreateDC*` | MS-EDD / RS-EDD | all `PENDING` |
| GDI-002 | adapter hash, monitor index/flags/name/IDs | policy table | same | MS-EDD / RS-EDD | all `PENDING` |
| GDI-003 | interface-path hash and GDI cross-map relation | `HASH_SHA256` | SetupAPI calls not separately approved | MS-EDD / RS-EDD | all `PENDING` |
| GDI-004..005 | exact valid `dmFields`, width, height, frequency, bpp, flags, position, orientation; current vs registry label | identifiers hashed; mode values kept; driver-private bytes dropped | all `ChangeDisplaySettings*`, every `CDS_*` | MS-EDS / RS-EDS | all `PENDING` |
| GDI-006..007 | mode index/type, exact allowlisted tuple, duplicate marker, current/preferred relation | identifiers hashed; tuple kept | `EDS_ROTATEDMODE`; all mutation APIs/flags | MS-EDS / RS-EDS | all `PENDING` |
| CCD-001/003 | requested flags, returned counts, elapsed, return code | counts kept | `SetDisplayConfig`, every `SDC_*` | MS-GBS / RS-GBS | all `PENDING` |
| CCD-002/004 | path priority, active/database source/target tuples, LUID hash, IDs, output technology, rational refresh, rotation, scaling, scanline, status, topology ID | LUID hashed; source/target IDs and technical tuple kept; device paths absent | `SetDisplayConfig`, `QDC_ALL_PATHS`, `QDC_INCLUDE_HMD`, unsupported OS flags | MS-QDC / RS-QDC | all `PENDING` |
| CCD-005 | LUID hash, source ID/name hash | identity policy | `DisplayConfigSetDeviceInfo` | MS-DGI / RS-DGI | all `PENDING` |
| CCD-006 | target ID, friendly-name review field, monitor path hash, output flags | policy table | same | MS-DGI / RS-DGI | all `PENDING` |
| CCD-007 | target ID and exact preferred-mode structure | mode kept | same | MS-DGI / RS-DGI | all `PENDING` |
| CCD-008 | LUID hash and adapter device-path hash | `HASH_SHA256` | same | MS-DGI / RS-DGI | all `PENDING` |
| CCD-009 | target ID, advanced-color supported/enabled/wide-color flags, encoding, bits/channel, result | technical booleans/values kept | all `DISPLAYCONFIG_DEVICE_INFO_SET_*`, HDR/WCG setters | MS-DGI / RS-DGI | all `PENDING` |
| SEC-001/002/014 | operation, requested access exact value, success/error, elapsed; never raw handle | handle value `DROP` | foreign process, `TOKEN_ADJUST_*`, duplicate/inherit, arbitrary `CloseHandle` | MS-OPT / RS-SEC | all `PENDING` |
| SEC-003 | user SID scoped hash only | `HASH_SHA256`; raw SID/username dropped | account lookup/name resolution unless separately approved | MS-GTI/MS-TIC / RS-SEC | all `PENDING` |
| SEC-004/010/011 | admin well-known match, enabled/deny-only flags, count; no other group SID export | admin result kept; other SIDs dropped/hash only if needed | token/ACL/group mutation | MS-GTI/MS-TIC / RS-SEC | all `PENDING` |
| SEC-005/006 | elevated boolean and elevation enum | `KEEP` | linked-token open, token duplication/set | MS-GTI/MS-TIC / RS-SEC | all `PENDING` |
| SEC-007 | session ID | `KEEP` | `SetTokenInformation` | MS-GTI/MS-TIC / RS-SEC | all `PENDING` |
| SEC-008/012/013 | integrity RID and normalized label | RID/label `KEEP`; SID raw dropped | integrity/token setters | MS-GTI/MS-TIC / RS-SEC | all `PENDING` |
| SEC-009 | authentication LUID / current logon identity | `HASH_SHA256` | `TokenOrigin`, linked-token open, token setters | MS-GTI/MS-TIC / RS-SEC | all `PENDING` |
| SES-001 | current PID | `KEEP` | foreign PID enumeration/open | RS-THR | all `PENDING` |
| SES-002 | current PID/session pair | PID/session `KEEP` | foreign process/session query | MS-P2S / RS-THR | all `PENDING` |
| SES-003 | console session ID or `NO_CONSOLE` | `KEEP` | WTS disconnect/logoff/control/notification registration | MS-WTS / RS-WTS | all `PENDING` |
| SYS-001 | architecture and processor level/revision needed for support cell | architecture kept; unnecessary fields dropped | none beyond exact getter; system setters forbidden | Microsoft GetNativeSystemInfo / RS-SYS | all `PENDING` |
| SYS-002/003 | tick, UTC FILETIME, before/after elapsed and consistency marker | raw tick/timestamp kept in restricted evidence; publication review | time setters, inferred boot authority | MS-TICK/MS-TIME / RS-SYS | all `PENDING` |
| SYS-004 | originating row, extended error code, immediate-read sequence | `KEEP` | `SetLastError`, stale/unattributed reads | MS-GLE / RS-FND | all `PENDING` |

### 9.4 Flag decisions outside the proposed allowlist

| Flag / query | Decision | Reason |
| --- | --- | --- |
| `EDS_ROTATEDMODE` | `REJECTED` for first cell | Current orientation is observed by current mode and CCD. Enumerating every rotated orientation expands candidate space without serving the initial read-only identity/mode objective. A later orientation-specific cell needs a new row. |
| `QDC_ALL_PATHS` | `REJECTED` for first cell | Microsoft documents it as potentially very expensive; inactive paths lack full mode details. Initial safety mapping uses active and database-current paths. A separate exploratory record may propose it with its own measured timebox. |
| `QDC_VIRTUAL_REFRESH_RATE_AWARE` | `REJECTED` for the proposed Windows 10 first cell | Microsoft documents support beginning with Windows 11. A Windows 11 cell must add separate paired size/query rows with the exact flag combination. |
| `QDC_INCLUDE_HMD` | `REJECTED` | HMD/specialized displays are outside initial scope. |
| `QDC_ONLY_ACTIVE_PATHS` without `QDC_VIRTUAL_MODE_AWARE` | `REJECTED` for this proposal | Microsoft current sample uses virtual-mode awareness, and the design requires observing source/target virtual-mode differences on Windows 10. |

## 10. Forbidden-call Policy

Version: `PHASE1A-FORBIDDEN-V1`  
Status: `READY_FOR_HUMAN_REVIEW`

The rule applies to direct calls, wrappers, re-exports, macros, dynamic lookup, generated bindings invoked by source, dependency callbacks configured by Phase 1A source, and binary imports. Presence in the `windows` crate or an enabled feature alone is not a violation.

Explicitly forbidden:

- Display mutation/validation families: `ChangeDisplaySettingsA/W`, `ChangeDisplaySettingsExA/W`, `SetDisplayConfig`, `DisplayConfigSetDeviceInfo`, DDC/CI monitor setters, orientation/primary/scaling/HDR/WCG/color/profile setters.
- Mutation/validation flags anywhere in Phase 1A call construction: `CDS_TEST`, `CDS_UPDATEREGISTRY`, all other `CDS_*`; `SDC_VALIDATE`, `SDC_APPLY`, `SDC_SAVE_TO_DATABASE`, and all other `SDC_*`.
- Registry writes: `RegCreateKey*`, `RegOpenKey*` with write rights, `RegSetValue*`, `RegDelete*`, `RegRenameKey`, `RegRestoreKey*`, `RegSaveKey*`, `RegLoadKey*`, transaction variants, `SHSetValue*`, and Rust/crate registry write wrappers.
- Display profile writes, ICC/profile association changes, registry-based display writes, Explorer restart/sign-out automation.
- Named or writable synchronization objects: `CreateMutexW`, `OpenMutexW`, `CreateEventW`, `OpenEventW`, `CreateSemaphoreW`, `OpenSemaphoreW`, named waitable timers, `Global\\`/`Local\\` object names, ownership/wait/abandonment prototypes, writable file locks, DACL/SDDL/security-descriptor mutation.
- Filesystem persistence: Rust `std::fs::write`, `File::create`, write/append/truncate/create `OpenOptions`; Win32 `CreateFile*` with write/create access, direct source calls to `WriteFile`, `FlushFileBuffers`, `SetEndOfFile`, `DeleteFile*`, `MoveFileEx*`, `ReplaceFile*`, and equivalent crates. Bounded JSON may be written only to stdout by Phase 1A source. A `WriteFile` import proven to arise solely from the pinned Rust standard library's stdout implementation is documented and dataflow-reviewed rather than silently treated as source authorization.
- DecisionJournal, `ProvisionCurrentDecisionBaselineV1`, MachineActorRecord, operational WAL, lock, recovery, reclaim, lease, or transaction operations.
- Process and job control: `CreateProcess*`, Rust `Command`, shell/process crates, `TerminateProcess`, `ExitProcess` as a control strategy, remote-thread/APC creation, Job Object create/assign/terminate/mutation.
- Watchdog, worker role, heartbeat, takeover, parent-loss, process fencing, process spawn/termination prototypes.
- Tauri, React, WebView2, `BOOT_HANDSHAKE`, presentation acknowledgement, event/command prototype, UI/network/server code.
- Shells and script hosts: PowerShell, `pwsh`, `cmd.exe`, WSH, batch/PowerShell scripts, shell execute APIs.
- Dynamic code/import bypass: `LoadLibrary*`, `GetProcAddress`, `LdrLoadDll`, raw function-pointer invocation obtained at runtime, inline/global assembly, undocumented syscalls.
- Elevation/identity mutation: UAC restart, `runas`, service/scheduled task, token privilege/group/session/integrity setters, impersonation, token duplication, foreign-process token access.
- Scale mutation, direct display registry manipulation, primary-monitor mutation, HDR/WCG mutation, orientation mutation, profile persistence, virtual-display/DLDSR mutation.

## 11. Static Audit Plan

Version: `PHASE1A-STATIC-AUDIT-V1`  
Status: `READY_FOR_HUMAN_REVIEW`  
Execution status for every item: `NOT RUN`

| Audit | Planned evidence / pass condition | Status |
| --- | --- | --- |
| Win32 callsites | AST plus text inventory equals the human-approved allowlist function set; no generic Win32 invoke abstraction | `NOT RUN` |
| Forbidden API names | Case-sensitive and case-insensitive source/generated/config search has zero unauthorized matches, excluding policy/test-oracle text reviewed by path | `NOT RUN` |
| Mutation flags | Zero `CDS_*`/`SDC_*` use; rejected query flags absent | `NOT RUN` |
| `unsafe` blocks | Complete file/line/invariant/owner list; each block is within FFI boundary and tied to a row ID | `NOT RUN` |
| FFI boundaries | Module/export/call graph list; raw pointer/handle/union cannot escape to domain/evidence layer | `NOT RUN` |
| Windows imports | Source import list exactly matches approved modules/types/functions | `NOT RUN` |
| Cargo features | Resolved feature set equals the approved exact set; unexpected transitive feature reviewed | `NOT RUN` |
| Filesystem writes | Search Rust std/crates/Win32 calls and dataflow; zero write/create/append/truncate/persist path | `NOT RUN` |
| Registry writes | Search API names, crates, desired access masks, and strings; zero writes | `NOT RUN` |
| Process spawn | Search Rust `Command`, process crates, Win32 spawn APIs, shell strings; zero | `NOT RUN` |
| Process termination | Search termination/job/APC/remote-thread APIs; zero | `NOT RUN` |
| Named objects | Search create/open/wait names and `Global\\`/`Local\\`; zero | `NOT RUN` |
| Tauri/React/WebView | Dependency and source search; zero | `NOT RUN` |
| PowerShell/cmd | Source/config/script/command search; zero | `NOT RUN` |
| Dynamic loading | Search `LoadLibrary*`, libloading equivalents, DLL strings; zero | `NOT RUN` |
| Raw `GetProcAddress` | Search direct/wrapper/function-pointer paths; zero | `NOT RUN` |
| Inline assembly | Search `asm!`, `global_asm!`, build flags; zero | `NOT RUN` |
| `build.rs` | Must be absent unless separately reviewed and approved; capture hash/content if present | `NOT RUN` |
| Dependency tree | `cargo tree --locked --all-features` and inverse review after authorization; no Tauri/shell/registry/process/persistence helper | `NOT RUN` |
| Source hash | SHA-256 per source plus aggregate manifest tied to branch/full HEAD/design digest | `NOT RUN` |
| Cargo.lock hash | Exact raw-byte SHA-256, resolver/source/checksum review | `NOT RUN` |
| Binary import table | After a separately authorized build, inspect PE imports; imports must be explainable by source/dependencies and contain no forbidden API | `NOT RUN` |
| Binary/source reproducibility | Record toolchain, target, command, source/lock hashes and compare repeated artifact/import results | `NOT RUN` |

This plan does not authorize running Cargo, compiling, or producing a binary now.

## 12. Evidence Plan

Version: `PHASE1A-EVIDENCE-V1`  
Status: `READY_FOR_HUMAN_REVIEW`

Evidence is one immutable bundle per run. Failed/partial runs are retained as separate IDs and never overwritten by a retry. The future source emits bounded sanitized JSON to stdout; the human-approved capture mechanism, storage, retention, access, and immutable ID remain human decisions.

### 12.1 Environment

Required fields:

- Machine ID; Windows edition, version, build; installed KB inventory and collection-method limitations; ESU status.
- CPU/native architecture; execution privilege state; local console/RDP; current/other interactive session and Fast User Switching observation.
- GPU exact model, PCI/hardware identity under redaction, driver version/date/provider.
- Display exact model/firmware where available; connection type; physical port; dock/adapter/cable evidence.
- Allowed port value when the OS/driver cannot expose a trusted number: `GPU-side port number: NOT EXPOSED`.
- Current resolution, rational refresh, orientation, bits per pixel/channel, HDR/advanced-color state, scale observation and collection method.

### 12.2 Repository

- Repository path in masked form, branch, full HEAD SHA, working tree/index/untracked state.
- `Design Baseline SHA V1` and complete per-file hash table.
- Phase 1A source per-file and aggregate hash.
- Raw-byte `Cargo.lock` SHA-256.
- Allowlist, forbidden policy, audit plan, evidence plan, and redaction policy versions/hashes plus human approval ID.

### 12.3 Toolchain

- `rustup show active-toolchain`, verbose rustc and Cargo exact output, host/target, rustup profile/components/targets.
- MSVC/linker and Windows SDK exact versions.
- `windows = 0.62.2`, complete resolved dependency graph, Cargo features, lockfile package source/checksum.

### 12.4 Execution

- Evidence ID; API row ID and policy versions; exact command with path masking.
- Start/end UTC, `GetTickCount64` before/after, elapsed milliseconds, timebox result.
- Exit code; bounded stdout/stderr hashes and sanitized content; truncation flag must be false.
- Per call: sequence, exact flag/query class, input-source row IDs, return type/value, Win32 error code, attempt/retry count, returned counts, cap decisions.
- Overall maximum JSON: 8 MiB; adapter<=32; monitor<=32/adapter; mode<=4,096/list/adapter; CCD path<=64/mode<=256/1 MiB; token buffer<=65,536.

### 12.5 Result

- Adapters and displays; current and registry mode; compatible/raw mode tuples and duplicates.
- GDI identity hashes and CCD identity hashes; exact cross-map relation or `AMBIGUOUS/UNMAPPED`.
- Active and database topology; source/target modes; rational refresh; rotation/scaling/scanline/output technology/status.
- Source name hash, target friendly-name review value/path hash, adapter path hash, preferred mode.
- Advanced color observation and unsupported/error state.
- Token-derived privilege/session profile and console-session comparison.
- All exclusions, cap failures, hotplug/race retries, access denials, unsupported driver/session results, and most-severe result.

### 12.6 Evidence result rules

- No mapping by label similarity. Exactly one complete observation is required to claim an exact map.
- Missing, contradictory, truncated, cap-exceeded, invalid UTF-16/union/enum, access-denied, remote/non-console, or topology-race exhaustion is preserved and fails closed.
- Environment data collected manually or through a method not represented by an approved API row is labeled with collection method and is never treated as source-verified runtime authority.
- Installed KB evidence must state its coverage. `Win32_QuickFixEngineering`, if separately approved later, cannot be labeled a complete Windows update inventory.

## 13. Redaction Policy

Version: `PHASE1A-REDACTION-V1`  
Status: `READY_FOR_HUMAN_REVIEW`

`HASH_SHA256` means a domain-separated, evidence-bundle-scoped salted SHA-256 over the raw field bytes. The same salt/domain is used only where equality within one bundle is required; the salt and raw mapping are held separately by the Evidence Owner under the still-pending access policy. It is not a durable product identifier. `MASK` replaces the identifying portion with a stable bundle-local alias. `HUMAN_REVIEW` defaults to removal from any shared bundle unless the Reviewer records why a sanitized value is necessary.

| Field | Classification | Rule / rationale |
| --- | --- | --- |
| User SID | `HASH_SHA256` | Preserve same-user equality within bundle; never emit raw SID. |
| Username / domain account name | `DROP` | Not needed for technical validation. |
| Computer name | `HASH_SHA256` | Preserve environment correlation without publishing host name. |
| Machine ID | `MASK` | Use human-issued evidence alias, not hardware/host identifier. |
| Repository absolute path | `MASK` | Replace root with `$REPO`; preserve relative path. |
| Exact command | `MASK` | Keep arguments/flags; replace absolute paths and user names. |
| Windows device path | `HASH_SHA256` | Preserve cross-map equality, not raw path. |
| GDI `DeviceName` | `HASH_SHA256` | Session-scoped cross-map key. |
| GDI `DeviceString` | `KEEP` | Adapter/monitor model evidence; apply friendly-name review if it contains a user-assigned label. |
| GDI `DeviceID` | `HASH_SHA256` | May fingerprint hardware. |
| GDI `DeviceKey` | `HASH_SHA256` | Contains registry/device identity; no raw key. |
| Monitor friendly name | `HUMAN_REVIEW` | Keep model-only value when clearly non-personal; otherwise mask/drop custom/serial-like content. |
| Monitor device path | `HASH_SHA256` | Preserve exact equality within bundle. |
| Monitor serial | `DROP` | Not required for Phase 1A mapping. |
| Adapter LUID | `HASH_SHA256` | Preserve source/target adapter equality without exporting raw locally unique value. |
| Source ID | `KEEP` | Small session-scoped technical index; pair only with hashed LUID. |
| Target ID | `KEEP` | Same. |
| EDID-like/raw descriptor data | `HUMAN_REVIEW` | Default `DROP`; a separately approved minimal parsed model field may be kept. |
| Logon ID / authentication LUID | `HASH_SHA256` | Preserve same-logon equality only. |
| Process ID | `KEEP` | Ephemeral diagnostic value needed to prove current-process binding. |
| Raw handle/pointer/address | `DROP` | No validation need and unsafe to expose. |
| OS edition/version/build | `KEEP` | Required support-cell evidence. |
| Installed KB IDs | `KEEP` | Required qualification evidence; drop installer/account metadata. |
| ESU status | `KEEP` | Required qualification decision. |
| GPU model | `KEEP` | Required support-cell evidence. |
| GPU driver version/provider/date | `KEEP` | Required support-cell evidence. |
| Display model/firmware | `HUMAN_REVIEW` | Keep model/firmware; remove serial/custom label. |
| Connection / physical port | `KEEP` | Required cell evidence; `NOT EXPOSED` is allowed. |
| Display resolution | `KEEP` | Required technical result. |
| Refresh numerator/denominator and Hz rendering | `KEEP` | Preserve rational and display value. |
| Orientation/scaling/bpp/advanced-color booleans | `KEEP` | Required non-mutation observation. |
| API function/row ID/flags | `KEEP` | Required audit evidence. |
| API/Win32 error code | `KEEP` | Required troubleshooting/result classification. |
| UTC/tick/elapsed/exit code | `KEEP` | Restricted evidence; publication still reviews temporal linkability. |
| Raw stdout/stderr | `HUMAN_REVIEW` | Serializer should already sanitize; scan again before sharing. |

## 14. Execution Privilege

| Required field | Current value / collection |
| --- | --- |
| Account administrator-group membership | `PENDING_MACHINE_OBSERVATION`; SEC-004/010/011, including deny-only attribute |
| Process elevated | `PENDING_MACHINE_OBSERVATION`; SEC-005 |
| Integrity level | `PENDING_MACHINE_OBSERVATION`; SEC-008/012/013 |
| UAC state | `PENDING_HUMAN_INPUT`; record only token-derived elevation type/limited evidence from SEC-006 and do not infer global UAC policy when it is not provable |
| Effective execution profile | `PENDING_MACHINE_OBSERVATION` |

Allowed candidate profiles:

- `STANDARD_USER`
- `ADMIN_MEMBER_NON_ELEVATED`
- `ADMIN_ELEVATED`
- `UNKNOWN`

Decision rule: Phase 1A may be proposed only for `STANDARD_USER` or `ADMIN_MEMBER_NON_ELEVATED`. If observation yields `ADMIN_ELEVATED`, inconsistent token fields, unprovable administrator membership, unknown integrity, or automatic elevation/UAC is required, do not start or continue Phase 1A; return to Reviewer. No elevation retry is allowed.

## 15. Target Cell Scope

The first Phase 1A is exactly one physical cell, proposed shape only:

```text
Windows 10 exact edition/version/build/KB/ESU decision
NVIDIA exact GPU
exact GPU driver
exact physical monitor/firmware
exact direct connection/port or documented NOT EXPOSED
single local interactive console session
single-monitor observation scope
```

Every exact value is pending. Completion of one cell does not establish support for all Windows 10/11, NVIDIA/AMD/Intel, or all displays/drivers. Future separate cells are required for Windows 11, AMD, Intel, multi-monitor, HDMI, DisplayPort, dock/adapter, virtual display, RDP, HDR/DRR, and any other support claim. Their absence is not by itself a blocker to completing the first record, but no broader claim may be made.

## 16. Open Human Decisions

1. Restore/select the intended Git repository and freeze branch/full HEAD/clean tree/index.
2. Confirm the current Design Baseline digest against that commit and decide whether this newly created record belongs in the same baseline commit.
3. Supply every Human inputs and Target Machine field, including exact Windows 10 cell and ESU acceptance.
4. Review Rust/Cargo target profile and capture exact target-machine versions; decide whether `rustfmt`/`clippy` are required.
5. Review and approve/reject each of the 37 API rows and rejected flag decisions; assign immutable allowlist/approval IDs.
6. Decide whether environment version/KB/ESU evidence remains manual or needs separately proposed exact read-only API/WMI rows.
7. Approve forbidden/audit/evidence/redaction versions and evidence capture, location, retention, access, and failure handling.
8. Confirm Operator, Evidence Owner, Reviewer independence requirements and execution date/location.
9. Decide the target-machine procedure for a Win32 query that never returns; no process termination mechanism is authorized in Phase 1A source.
10. Issue a phase-specific authorization only after all blockers close. Record completion alone is insufficient.

## 17. Technical Preflight Status

| Item | Status | Reason |
| --- | --- | --- |
| Repository baseline | `NOT_FREEZABLE` | Not recognized as Git; branch/HEAD/tree/index unavailable. |
| Design Baseline SHA V1 | `CALCULATED_NOT_FROZEN` | Digest computed, but not tied to Git baseline. |
| Rust toolchain | `PROPOSED` | Official candidate verified; target-machine CLI/components/MSVC/SDK not verified or human-approved. |
| windows crate | `PROPOSED` | 0.62.2 publication/API paths verified; no Cargo resolution/compile or approval. |
| Cargo features | `PROPOSED` | Seven-feature minimal proposal; no resolved graph/build or approval. |
| API allowlist | `READY_FOR_HUMAN_REVIEW` | 37 complete proposed rows; 0 approved. |
| Forbidden list | `READY_FOR_HUMAN_REVIEW` | `PHASE1A-FORBIDDEN-V1`. |
| Static audit plan | `READY_FOR_HUMAN_REVIEW` | `PHASE1A-STATIC-AUDIT-V1`; all items `NOT RUN`. |
| Evidence plan | `READY_FOR_HUMAN_REVIEW` | `PHASE1A-EVIDENCE-V1`. |
| Redaction policy | `READY_FOR_HUMAN_REVIEW` | `PHASE1A-REDACTION-V1`. |
| Overall | `TECHNICAL_PREFLIGHT_BLOCKED` | Repository baseline cannot be frozen. |

## 18. Human Authorization

| Field | Value |
| --- | --- |
| Reviewer decision | `PENDING_HUMAN_INPUT` |
| Approver decision | `PENDING_HUMAN_INPUT` |
| Immutable approval ID | `PENDING_HUMAN_INPUT` |
| Signed/reference record | `PENDING_HUMAN_INPUT` |
| Approved allowlist row IDs | None (0) |
| Authorization state | `NOT AUTHORIZED` |

## 19. Record Completion

| Field | Value |
| --- | --- |
| Repository baseline frozen | No |
| Technical proposals prepared | Yes |
| Human fields complete | No |
| Human approval complete | No |
| Record status | `INCOMPLETE` |
| Completion reason | Technical policy is reviewable, but the repository baseline is `NOT_FREEZABLE` and all human authorization fields are pending. |

## 20. Phase Authorization

| Phase | Status |
| --- | --- |
| Phase 1A record | `INCOMPLETE` |
| Phase 1A execution | `NOT EXECUTABLE` |
| Phase 2A | `NOT EXECUTABLE` |
| Phase 1B | `NOT EXECUTABLE` |
| Phase 2B | `NOT EXECUTABLE` |

Stop point: this file records the preflight only. Do not create Phase 1A source/Cargo files, install/update a toolchain or crate, run Cargo/build/test, create a Windows binary, execute a Windows/display API, collect target-machine evidence, or begin any phase under this record.
