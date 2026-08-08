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
| Human approval represented by this record | Human Technical Decision Stage 1 and the five individual Section 9.5 flag decisions only; no source-creation, build, final, or execution authorization |
| Technical preflight status | `TECHNICAL_PREFLIGHT_COMPLETE` |
| Technical revision status | `TECHNICAL_REVISION_V2_REVIEW_COMPLETE` |
| Human Technical Decision Stage 1 | `HUMAN_TECHNICAL_DECISIONS_COMPLETE` |
| Human Flag Decision Stage | `HUMAN_FLAG_DECISIONS_COMPLETE` |
| Stage 1 decision recorded date | 2026-08-08 |
| Flag decision recorded date | 2026-08-08 |
| Hung-call procedure | `PHASE1A-HUNG-CALL-PROCEDURE-V1`; `PROPOSED_FOR_HUMAN_APPROVAL`; Human decision `PENDING_HUMAN_APPROVAL` |
| Blocking reason | Stage 1 technical decisions and the five individual Section 9.5 flag decisions are complete. Target-machine, people, evidence-storage, schedule, Approval ID, signature/reference, hung-call procedure approval, immutable record binding, source/build scope authorizations, source/binary evidence, and final execution authorization remain pending. |
| Phase 1A record status | `INCOMPLETE` |
| Phase 1A execution | `NOT EXECUTABLE` |
| Final authorization | `NOT AUTHORIZED` |

Normative interpretation: Stage 1 approves the technical selections explicitly recorded below, and the Human Flag Decision Stage records only the five exact exclusions in Section 9.5. `APPROVE_FIRST_CELL` selects a row for the proposed first cell only after all remaining gates and separate phase-specific authorization; it does not authorize implementation or execution now. `DEFER` does not approve a row for the first cell. Reviewer/Approver identity, immutable Approval ID, signature, hung-call procedure approval, final authorization, and all operational inputs remain pending. This record does not authorize source creation, dependency installation, compilation, static-audit execution, display API calls, evidence collection, or any phase.

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

The operational, identity, evidence-storage, schedule, and final-authorization values in this table remain absent. Stage 1 technical decisions are recorded in Sections 7–13 and 17–19; no value below was inferred.

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
| Windows edition | `PENDING_MACHINE_OBSERVATION` |
| Windows version | `PENDING_MACHINE_OBSERVATION` |
| OS build | `PENDING_MACHINE_OBSERVATION` |
| Installed KB list and evidence | `PENDING_MACHINE_OBSERVATION` |
| ESU status | `NOT CONFIRMED（登録済み表示なし）` |
| Approve this non-ESU-confirmed environment as the Phase 1A exploration cell | `PENDING_HUMAN_INPUT` |
| CPU architecture | `PENDING_MACHINE_OBSERVATION` (planned target: x64) |
| GPU / exact model | `PENDING_MACHINE_OBSERVATION` |
| GPU driver | `PENDING_MACHINE_OBSERVATION` |
| Display / firmware | `PENDING_MACHINE_OBSERVATION` |
| Connection | `PENDING_MACHINE_OBSERVATION` |
| Physical port | `PENDING_MACHINE_OBSERVATION`; allowed evidence value: `GPU-side port number: NOT EXPOSED` |
| Dock / adapter | `PENDING_MACHINE_OBSERVATION` |
| Current resolution / refresh | `PENDING_MACHINE_OBSERVATION` |
| HDR / advanced color / scale | `PENDING_MACHINE_OBSERVATION` |
| Local console / RDP | `PENDING_MACHINE_OBSERVATION` |
| Multi-user / Fast User Switching state | `PENDING_MACHINE_OBSERVATION` |

OS edition/version/build/KB and ESU are environment evidence, not feature-presence authority. Phase 1A source must not use registry writes, process spawning, PowerShell, `cmd.exe`, or an undocumented version query. This record proposes only the Win32 rows below; any automated WMI/CIM or additional version/KB call requires a new row and human review. `Win32_QuickFixEngineering` is not treated as a complete update inventory because Microsoft documents that it returns only CBS-supplied updates.

## 5. Repository Baseline

Repository state was verified with read-only Git commands on 2026-08-08 at `2026-08-08T13:43:47+09:00`.

```text
git rev-parse --show-toplevel
git branch --show-current
git rev-parse HEAD
git rev-parse --short=7 HEAD
git log -1 --format=%s
git log -1 --format=%cI
git status --porcelain=v1 --untracked-files=all
git status --short --branch
git diff --name-only
git diff --cached --name-only
git ls-files --others --exclude-standard
git remote get-url origin
git remote get-url --push origin
```

`RepositoryBaselineV1 = repository root + branch + full HEAD SHA + clean working tree + Design Baseline SHA V1`

| Field | Value |
| --- | --- |
| Repository Baseline ID | `P1A-REPO-09be6a3` |
| Repository root | `/Users/koichi/project/DisplayDeck` |
| Branch | `main` |
| HEAD full SHA | `09be6a3e05651b9587d526c2d57e542823ec9297` |
| HEAD short SHA | `09be6a3` |
| Latest commit subject | `second commit` |
| Latest commit timestamp | `2026-08-08T13:37:36+09:00` |
| Working tree state | `CLEAN` |
| Index state | `CLEAN` |
| Untracked files | None |
| Modified files | None |
| Staged files | None |
| Design documents modified relative to HEAD | None |
| Design Baseline SHA V1 | `d3764f3e0cafa9c7b0a89468e59b7452f627885483de90e90f68dedafabb015e` (`FROZEN`) |
| Remote origin fetch URL | `git@github.com:garugan/DisplayDeck.git` |
| Remote origin push URL | `git@github.com:garugan/DisplayDeck.git` |
| Baseline created timestamp | `2026-08-08T13:37:36+09:00` (baseline commit timestamp) |
| Baseline verification timestamp | `2026-08-08T13:43:47+09:00` |
| Repository baseline status | `FROZEN` |

The verified HEAD contains `.gitignore`, `AGENTS.md`, `README.md`, `docs/requirements.md`, `docs/architecture.md`, `docs/windows-display-research.md`, `docs/security.md`, `docs/testing-strategy.md`, `docs/implementation-plan.md`, `docs/tauri-design-review-checklist.md`, `docs/tauri-review-resolution.md`, `docs/tauri-design-final-review.md`, and `docs/tauri-design-closure-review.md`. It also contains `docs/phase-1a-execution-record.md`; therefore `BASELINE_RECORD_NOT_COMMITTED` does not apply.

The clean-state observation above is the pre-edit capture used to identify the immutable HEAD baseline. The current working tree contains the permitted modification of `docs/phase-1a-execution-record.md` and the untracked review artifacts `docs/phase-1a-human-review.md` and `docs/phase-1a-human-rereview.md`. Those working-tree artifacts do not alter the identified HEAD commit, Repository Baseline ID, or Design Baseline SHA V1 and do not authorize a commit or phase execution.

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

Digest state: `FROZEN`.

Previous calculated SHA: `MATCH` (`d3764f3e0cafa9c7b0a89468e59b7452f627885483de90e90f68dedafabb015e`). All 14 Design Baseline V1 files are present in HEAD, and no differing file was detected.

## 7. Toolchain Freeze

| Field | Proposed value | Basis / condition |
| --- | --- | --- |
| Rust channel | `stable` | Phase 1A needs no nightly-only feature. |
| rustc exact version | `1.97.1` | Rust Release Team published the 1.97.1 stable point release on 2026-07-16. |
| Cargo exact version | `PENDING_MACHINE_VERIFICATION` | The official 1.97.1 distribution contains the Cargo component and an x64 MSVC artifact named `cargo-1.97.1`; exact `cargo --version --verbose` output must be captured on the target machine. The manifest's Cargo package source version is `0.98.0 (c980f4866 2026-06-30)`, which is not substituted for machine CLI evidence. |
| Target triple | `x86_64-pc-windows-msvc` | Rust documents this as Tier 1 with host tools and the 1.97.1 distribution marks the target available. |
| rustup profile | `minimal` | Limits installation to rustc, rust-std, and Cargo; docs and unrelated targets are unnecessary. |
| Required profile components | `rustc`, `rust-std`, `cargo` | Compile/link prerequisites after separate authorization. |
| Additional approved Stage 1 components | `rustfmt`, `clippy` | Approved for future formatting/lint evidence; availability and exact target-machine versions still require capture after separate authorization. |
| Excluded components | nightly, beta, `rust-src`, `rust-docs`, Miri, LLVM tools | Not required for Phase 1A's approved future read-only scope. |
| Nightly features | None | All proposed language/library/API-binding work is expected to compile on stable; a nightly requirement returns to review. |
| Repository-design consistency | Yes: Windows-only Rust FFI boundary, no Tauri/React/watchdog/worker in Phase 1A. | Matches the fixed technology boundary and Phase 1A read-only scope. |
| Rust stable 1.97.1 / x64 MSVC / minimal Stage 1 decision | `APPROVE` | Technical selection only; target-machine verification and execution authorization remain pending. |
| rustfmt Stage 1 decision | `APPROVE` | Approved as future `SHOULD` evidence; not installed or run. |
| Clippy Stage 1 decision | `APPROVE` | Approved as future `SHOULD` evidence; not installed or run. |
| Status | `HUMAN_TECHNICAL_DECISION_APPROVED` | Toolchain is not installed, target-verified, `FROZEN`, or executable under Stage 1. |

Primary official evidence:

- [Rust 1.97.1 release announcement](https://blog.rust-lang.org/2026/07/16/Rust-1.97.1/)
- [Rust stable release index](https://blog.rust-lang.org/releases/)
- [Rust 1.97.1 distribution manifest](https://static.rust-lang.org/dist/channel-rust-1.97.1.toml)
- [Rust Windows MSVC target support](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html)

Freeze gate: run no installer or toolchain command under this record. After repository freeze and human authorization, the target-machine record must capture `rustup show active-toolchain`, `rustc --version --verbose`, `cargo --version --verbose`, installed components, installed targets, and MSVC/Windows SDK identity. Until then, the toolchain is not `FROZEN`.

Toolchain Stage 1 decision: `APPROVE` for stable Rust 1.97.1, `x86_64-pc-windows-msvc`, minimal profile, rustfmt, and Clippy. rustfmt and Clippy remain `SHOULD` evidence before execution. This decision does not install, run, freeze, or authorize the toolchain.

## 8. windows crate / Cargo Features

### 8.1 Crate candidate

| Field | Value |
| --- | --- |
| Crate | `windows` |
| Exact candidate version | `=0.62.2` |
| Publication evidence | [crates.io](https://crates.io/crates/windows/0.62.2), [docs.rs release record](https://docs.rs/crate/windows/0.62.2), [Microsoft generated Rust docs](https://microsoft.github.io/windows-docs-rs/doc/windows/) |
| Target docs | [docs.rs x86_64-pc-windows-msvc](https://docs.rs/windows/0.62.2/x86_64-pc-windows-msvc/windows/) |
| API path availability | All 21 distinct functions referenced by `PHASE1A-ALLOWLIST-V2` are present in 0.62.2 at the paths below. |
| Exact dependency resolution / lock hash | `PENDING`; no `Cargo.toml` or `Cargo.lock` exists or was created. |
| Stage 1 Human decision | `APPROVE` |
| Status | `HUMAN_TECHNICAL_DECISION_APPROVED` |

| Function | `windows` 0.62.2 module | V2 row IDs | First-cell classification | Binding availability | Feature |
| --- | --- | --- | --- | --- | --- |
| `EnumDisplayDevicesW` | `windows::Win32::Graphics::Gdi` | P1A-GDI-001, P1A-GDI-002, P1A-GDI-003 | `REQUIRED` | `CONFIRMED` | `Win32_Graphics_Gdi` |
| `EnumDisplaySettingsExW` | `windows::Win32::Graphics::Gdi` | P1A-GDI-004, P1A-GDI-005, P1A-GDI-006, P1A-GDI-007 | `REQUIRED` + `DEFERRED` | `CONFIRMED` | `Win32_Graphics_Gdi` |
| `GetDisplayConfigBufferSizes` | `windows::Win32::Devices::Display` | P1A-CCD-001, P1A-CCD-003 | `REQUIRED` + `DEFERRED` | `CONFIRMED` | `Win32_Devices_Display` |
| `QueryDisplayConfig` | `windows::Win32::Devices::Display` | P1A-CCD-002, P1A-CCD-004 | `REQUIRED` + `DEFERRED` | `CONFIRMED` | `Win32_Devices_Display` |
| `DisplayConfigGetDeviceInfo` | `windows::Win32::Devices::Display` | P1A-CCD-005, P1A-CCD-006, P1A-CCD-007, P1A-CCD-008, P1A-CCD-009 | `REQUIRED` + `OPTIONAL` + `DEFERRED` | `CONFIRMED` | `Win32_Devices_Display` |
| `GetCurrentProcess` | `windows::Win32::System::Threading` | P1A-SEC-001 | `REQUIRED` | `CONFIRMED` | `Win32_System_Threading` |
| `OpenProcessToken` | `windows::Win32::Security` | P1A-SEC-002 | `REQUIRED` | `CONFIRMED` | `Win32_Security` |
| `GetTokenInformation` | `windows::Win32::Security` | P1A-SEC-003, P1A-SEC-004, P1A-SEC-005, P1A-SEC-006, P1A-SEC-007, P1A-SEC-008, P1A-SEC-009 | `REQUIRED` + `OPTIONAL` | `CONFIRMED` | `Win32_Security` |
| `GetLengthSid` | `windows::Win32::Security` | P1A-SEC-010 | `REQUIRED` | `CONFIRMED` | `Win32_Security` |
| `IsValidSid` | `windows::Win32::Security` | P1A-SEC-015 | `REQUIRED` | `CONFIRMED` | `Win32_Security` |
| `IsWellKnownSid` | `windows::Win32::Security` | P1A-SEC-011 | `REQUIRED` | `CONFIRMED` | `Win32_Security` |
| `GetSidSubAuthorityCount` | `windows::Win32::Security` | P1A-SEC-012 | `REQUIRED` | `CONFIRMED` | `Win32_Security` |
| `GetSidSubAuthority` | `windows::Win32::Security` | P1A-SEC-013 | `REQUIRED` | `CONFIRMED` | `Win32_Security` |
| `CloseHandle` | `windows::Win32::Foundation` | P1A-SEC-014 | `REQUIRED` | `CONFIRMED` | `Win32_Foundation` |
| `GetCurrentProcessId` | `windows::Win32::System::Threading` | P1A-SES-001 | `DEFERRED_REMOVE` | `CONFIRMED` | `Win32_System_Threading` |
| `ProcessIdToSessionId` | `windows::Win32::System::Threading` | P1A-SES-002 | `DEFERRED_REMOVE` | `CONFIRMED` | `Win32_System_Threading` |
| `WTSGetActiveConsoleSessionId` | `windows::Win32::System::RemoteDesktop` | P1A-SES-003 | `REQUIRED` | `CONFIRMED` | `Win32_System_RemoteDesktop` |
| `GetNativeSystemInfo` | `windows::Win32::System::SystemInformation` | P1A-SYS-001 | `OPTIONAL` | `CONFIRMED` | `Win32_System_SystemInformation` |
| `GetTickCount64` | `windows::Win32::System::SystemInformation` | P1A-SYS-002 | `REQUIRED` | `CONFIRMED` | `Win32_System_SystemInformation` |
| `GetSystemTimePreciseAsFileTime` | `windows::Win32::System::SystemInformation` | P1A-SYS-003 | `DEFERRED_EXPLORATORY` | `CONFIRMED` | `Win32_System_SystemInformation` |
| `GetLastError` | `windows::Win32::Foundation` | P1A-SYS-004 | `DEFERRED_REMOVE` | `CONFIRMED` | `Win32_Foundation` |

The availability classification is documentation-only evidence and is not compile evidence or approval. Deferred functions remain in this inventory because their rows remain in Allowlist V2 pending per-row human decisions.

### 8.2 Cargo feature proposal

Exact dependency proposal (ordering is normative for the review artifact only):

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

Stage 1 Human decisions are `APPROVE` for `windows = "=0.62.2"`, `APPROVE` for `default-features = true`, and `APPROVE_ALL_7` for the seven features shown above. No `Cargo.toml` or lockfile is created, and no dependency resolution, build, or execution is authorized by these decisions.

| Feature | Required API/module and reason | Phase 1A use | Dangerous sibling examples exposed by the feature | Static-audit treatment |
| --- | --- | --- | --- | --- |
| `Win32_Devices_Display` | CCD query functions and packet types in `Win32::Devices::Display` | Buffer sizing, active/database topology, names, preferred mode, advanced color observation | `SetDisplayConfig`, `DisplayConfigSetDeviceInfo`, monitor-setting functions and `SDC_*` constants | Only named approved callsites; explicit forbidden-name/flag/import scans. |
| `Win32_Foundation` | `HANDLE`, `BOOL`, `WIN32_ERROR`, `LUID`, `CloseHandle`, `GetLastError` | Typed return/error values, immediate documented error capture, and owned token-handle close | Broad foundational handles/errors; misuse can close unrelated handles or read stale last-error | Handle provenance review; only the token handle from SEC-002 may reach SEC-014; direct SYS-004 may be used only if individually approved and immediately after a documented failing call. |
| `Win32_Graphics_Gdi` | GDI enumeration and `DEVMODEW`/`DISPLAY_DEVICEW`; also required by generated CCD types | Adapter/monitor/current/registry/compatible/raw observation | `ChangeDisplaySettingsW/ExW`, `CDS_*`, many graphics mutation calls | Exact callsite allowlist plus mutation-flag and binary-import audit. |
| `Win32_Security` | Current-token query and SID helpers | User SID hash, admin group membership, elevation, integrity, session | `SetTokenInformation`, privilege/group adjustment, ACL/security-descriptor mutation | Query-class allowlist; reject every setter/adjuster/ACL call. |
| `Win32_System_RemoteDesktop` | `WTSGetActiveConsoleSessionId` | Compare the process session with the physical-console session | Session control, disconnect/logoff/send-message APIs | Only the one approved getter may be imported/called. |
| `Win32_System_SystemInformation` | `GetNativeSystemInfo`, `GetTickCount64`, `GetSystemTimePreciseAsFileTime` | Architecture and read-only clock evidence | Time/computer-name setter functions in the same module | Ban all `Set*` system/time/computer-name calls; exact getter import list. |
| `Win32_System_Threading` | `GetCurrentProcess`, `GetCurrentProcessId`, `ProcessIdToSessionId` | Bind token/session checks to the current process only | `CreateProcess*`, `TerminateProcess`, named mutex/event/semaphore, Job Object mutation | Spawn/termination/named-object/Job Object name scan and binary-import review. |

Feature status: `HUMAN_TECHNICAL_DECISION_APPROVED`; Stage 1 decision: `APPROVE_ALL_7`. A feature exposing a dangerous sibling is not by itself a violation. A future Phase 1A source callsite, re-export, or dynamic lookup outside an approved first-cell row is a violation; final-PE imports are adjudicated by `PHASE1A-PE-IMPORT-PROVENANCE-V1`, with every display-mutation import and every `UNEXPLAINED` import failing the audit.

## 9. Exact API Allowlist

Allowlist artifact: `PHASE1A-ALLOWLIST-V2`

Artifact status: `HUMAN_TECHNICAL_DECISIONS_COMPLETE`

Total rows: **38**

Stage 1 decisions recorded: **38** (`APPROVE_FIRST_CELL`: **23**; `DEFER`: **15**)

Final phase-approved or executable rows: **0**

History: `PHASE1A-ALLOWLIST-V1` contained 37 rows and was superseded by V2 before human approval. V1 was never human-approved. V2 adds `P1A-SEC-015`; independent Human Re-review found V2 ready for Human decisions, and Human Technical Decision Stage 1 then recorded one decision for each of its 38 rows. These Stage 1 decisions do not constitute final phase approval or execution authorization.

### 9.1 Common rules

- Every row contract remains the reviewed V2 contract. Section 9.4.1 records the individual Stage 1 Human decision for all 38 rows. Reviewer identity, Approver identity, immutable Approval ID, signed/reference record, final authorization, and execution state remain `PENDING_HUMAN_INPUT` / `NOT AUTHORIZED`.
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
| P1A-SEC-003 | Advapi32.dll; same; `GetTokenInformation` | Current token user SID. After success: validate returned structure/buffer bounds, prove SID pointer/header minimum region is in-buffer, require SEC-015, then SEC-010, prove the full SID byte range is in-buffer, and only then hash. Never hash an invalid SID. | `TokenUser`; SEC-002 token | variable buffer <=65,536; one probe+one fill | probe `ERROR_INSUFFICIENT_BUFFER`, then success; all else terminal; invalid SID makes this optional evidence unavailable without raw parsing | `TIMEBOX-P1A-01` |
| P1A-SEC-004 | Advapi32.dll; same; `GetTokenInformation` | Token groups to identify built-in Administrators membership/deny-only state. After success: checked group-count arithmetic; for each group prove SID pointer/header in-buffer, require SEC-015, then SEC-010 and full SID range in-buffer, then read group attributes, then SEC-011. | `TokenGroups`; SEC-002 token | <=65,536 and <=2,048 groups; one probe+fill | same; any invalid group SID makes the allowed execution profile unprovable and the run fails closed | `TIMEBOX-P1A-01` |
| P1A-SEC-005 | Advapi32.dll; same; `GetTokenInformation` | Process elevated boolean | `TokenElevation` | exact structure size; retry 0 | success only | `TIMEBOX-P1A-01` |
| P1A-SEC-006 | Advapi32.dll; same; `GetTokenInformation` | UAC token elevation type | `TokenElevationType` | exact enum size; retry 0 | success only | `TIMEBOX-P1A-01` |
| P1A-SEC-007 | Advapi32.dll; same; `GetTokenInformation` | Token Windows session ID | `TokenSessionId` | exact DWORD size; retry 0 | success only | `TIMEBOX-P1A-01` |
| P1A-SEC-008 | Advapi32.dll; same; `GetTokenInformation` | Integrity-level SID. After success: validate structure bounds and SID pointer/header bounds, require SEC-015, then SEC-010 and full SID range bounds, then SEC-012, checked nonzero count, and SEC-013. | `TokenIntegrityLevel` | <=65,536; one probe+fill | probe insufficient buffer, then success; invalid SID, zero subauthority count, or any out-of-buffer condition fails closed | `TIMEBOX-P1A-01` |
| P1A-SEC-009 | Advapi32.dll; same; `GetTokenInformation` | Current logon authentication LUID for same-logon equality evidence | `TokenStatistics`; read `TOKEN_STATISTICS.AuthenticationId` only | exact `TOKEN_STATISTICS` size; retry 0 | success only; zero/unknown authentication ID is preserved, not replaced | `TIMEBOX-P1A-01` |
| P1A-SEC-010 | Advapi32.dll; `Win32::Security`; `GetLengthSid` | Obtain the bounded serialized length of a validated OS-returned user/group/integrity SID. `GetLengthSid` is called only after SEC-015 `IsValidSid` returned TRUE; it is not a validation API. | only already-validated SID pointers rooted in SEC-003/004/008 bounded buffers | returned length must fit the containing buffer; retry 0 | use the returned length only for the subsequent full-range bounds proof; never call on an invalid SID or infer validity from its return value | `TIMEBOX-P1A-01` |
| P1A-SEC-011 | Advapi32.dll; same; `IsWellKnownSid` | Compare a SEC-004 group SID with `WinBuiltinAdministratorsSid` only after SEC-015, SEC-010, full SID range validation, and safe group-attribute read | exact well-known SID type; validated in-buffer SID only | <=2,048 calls; retry 0 | true=match; false=non-match only for a structurally valid, fully bounded SID | `TIMEBOX-P1A-01` |
| P1A-SEC-012 | Advapi32.dll; same; `GetSidSubAuthorityCount` | Read bounded integrity SID subauthority count only after SEC-015, SEC-010, and full SID range validation | SEC-008 validated in-buffer SID only | returned count pointer must remain within validated SID; retry 0 | non-null and checked count>=1; otherwise fail closed and never call SEC-013 | `TIMEBOX-P1A-01` |
| P1A-SEC-013 | Advapi32.dll; same; `GetSidSubAuthority` | Read final integrity RID only after SEC-012 returned a checked nonzero count | index=`count-1` from SEC-012; SEC-008 validated in-buffer SID only | returned DWORD range must remain within validated SID; retry 0 | non-null/in-bounds; map documented integrity RIDs, unknown remains `UNKNOWN`; failure is fail-closed | `TIMEBOX-P1A-01` |
| P1A-SEC-014 | Kernel32.dll; `Win32::Foundation`; `CloseHandle` | Close exactly the owned token handle from SEC-002 once | no flags/right; pseudo-handle SEC-001 forbidden | one handle; retry 0 | nonzero success; zero recorded as resource-cleanup failure | `TIMEBOX-P1A-01` |
| P1A-SEC-015 | Advapi32.dll; `Win32::Security`; `IsValidSid` | Structurally validate only a SID pointer from a successful SEC-003 `TokenUser`, SEC-004 `TokenGroups`, or SEC-008 `TokenIntegrityLevel` bounded buffer before any SID-reading API. First prove non-null, pointer in the containing buffer, and the minimum SID-header region fully in-buffer. Arbitrary/external SIDs or pointers are forbidden. | no flags/right; bounded OS-returned SID only | retry 0 | TRUE=structural validation pass; FALSE=current SID is unusable and must not reach SEC-010/011/012/013. Invalid required SID fails closed; invalid optional SID becomes unavailable evidence with no raw parsing. No associated last-error is attributed unless the Microsoft contract requires it. | `TIMEBOX-P1A-01` |
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
- `MS-IVS`: [IsValidSid](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-isvalidsid); `MS-GLS`: [GetLengthSid](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-getlengthsid); `RS-IVS`: [windows 0.62.2 generated docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Security/fn.IsValidSid.html)
- `MS-P2S`: [ProcessIdToSessionId](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-processidtosessionid); `RS-THR`: [generated Threading module](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Threading/index.html)
- `MS-WTS`: [WTSGetActiveConsoleSessionId](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-wtsgetactiveconsolesessionid); `RS-WTS`: [generated docs](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/RemoteDesktop/fn.WTSGetActiveConsoleSessionId.html)
- `MS-TICK`: [GetTickCount64](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64); `MS-TIME`: [GetSystemTimePreciseAsFileTime](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemtimepreciseasfiletime); `RS-SYS`: [generated SystemInformation module](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/SystemInformation/index.html)
- `MS-GLE`: [GetLastError](https://learn.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror); `RS-FND`: [generated Foundation module](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Foundation/index.html)

| ID range / row | Evidence fields | Redaction | Forbidden sibling examples | Docs | Reviewer identity / Approver identity / final authorization |
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
| SEC-003 | user SID scoped hash only after SEC-015, SEC-010, and full-range bounds proof | `HASH_SHA256` only after validation; raw SID/username dropped | account lookup/name resolution unless separately approved | MS-GTI/MS-TIC/MS-IVS/MS-GLS / RS-SEC | all `PENDING` |
| SEC-004/010/011 | admin well-known match, enabled/deny-only flags, checked count, length/bounds result; no other group SID export | admin/validation result `KEEP`; raw SID `DROP`; hash only after validation | SID/ACL/token mutation, arbitrary caller-provided SID parsing, out-of-buffer dereference | MS-GTI/MS-TIC/MS-IVS/MS-GLS / RS-SEC | all `PENDING` |
| SEC-005/006 | elevated boolean and elevation enum | `KEEP` | linked-token open, token duplication/set | MS-GTI/MS-TIC / RS-SEC | all `PENDING` |
| SEC-007 | session ID | `KEEP` | `SetTokenInformation` | MS-GTI/MS-TIC / RS-SEC | all `PENDING` |
| SEC-008/012/013 | length/bounds result, checked subauthority count, integrity RID and normalized label | validation result and RID/label `KEEP`; raw SID `DROP`; hash only after validation | SID/integrity/token/ACL mutation, arbitrary SID parsing, out-of-buffer dereference | MS-GTI/MS-TIC/MS-IVS/MS-GLS / RS-SEC | all `PENDING` |
| SEC-009 | authentication LUID / current logon identity | `HASH_SHA256` | `TokenOrigin`, linked-token open, token setters | MS-GTI/MS-TIC / RS-SEC | all `PENDING` |
| SEC-015 | originating token row, SID category, validation success/failure, elapsed, and associated Win32 error only when the Microsoft contract requires it | validation result `KEEP`; raw SID `DROP`; SID hash allowed only after TRUE | SID/ACL/token mutation, arbitrary caller-provided SID parsing, out-of-buffer dereference | MS-IVS / RS-IVS | all `PENDING` |
| SES-001 | current-process provenance | normalize to `CURRENT_PROCESS`; PID `DROP` | foreign PID enumeration/open | RS-THR | all `PENDING` |
| SES-002 | current-process/session relation | normalize provenance to `CURRENT_PROCESS`; PID `DROP`, session `KEEP` | foreign process/session query | MS-P2S / RS-THR | all `PENDING` |
| SES-003 | console session ID or `NO_CONSOLE` | `KEEP` | WTS disconnect/logoff/control/notification registration | MS-WTS / RS-WTS | all `PENDING` |
| SYS-001 | architecture and processor level/revision needed for support cell | architecture kept; unnecessary fields dropped | none beyond exact getter; system setters forbidden | Microsoft GetNativeSystemInfo / RS-SYS | all `PENDING` |
| SYS-002/003 | tick, UTC FILETIME, before/after elapsed and consistency marker | raw tick/timestamp kept in restricted evidence; publication review | time setters, inferred boot authority | MS-TICK/MS-TIME / RS-SYS | all `PENDING` |
| SYS-004 | originating row, extended error code, immediate-read sequence | `KEEP` | `SetLastError`, stale/unattributed reads | MS-GLE / RS-FND | all `PENDING` |

### 9.4 First-cell candidate V2

This classification was the Technical Revision V2 proposal reviewed in the Human Re-review. Human Technical Decision Stage 1 decisions are recorded individually in Section 9.4.1; they select or defer rows technically but do not authorize source creation, audit, build, API execution, or any phase.

| Classification | Count | Row IDs |
| --- | ---: | --- |
| Required candidate | **23** | P1A-GDI-001, P1A-GDI-002, P1A-GDI-003, P1A-GDI-004, P1A-GDI-006; P1A-CCD-001, P1A-CCD-002, P1A-CCD-005, P1A-CCD-006; P1A-SEC-001, P1A-SEC-002, P1A-SEC-004, P1A-SEC-005, P1A-SEC-007, P1A-SEC-008, P1A-SEC-010, P1A-SEC-011, P1A-SEC-012, P1A-SEC-013, P1A-SEC-014, P1A-SEC-015; P1A-SES-003; P1A-SYS-002 |
| Optional candidate | **5** | P1A-CCD-008; P1A-SEC-003, P1A-SEC-006, P1A-SEC-009; P1A-SYS-001 |
| Deferred / removed from first cell | **10** | P1A-GDI-005, P1A-GDI-007; P1A-CCD-003, P1A-CCD-004, P1A-CCD-007, P1A-CCD-009; P1A-SES-001, P1A-SES-002; P1A-SYS-003, P1A-SYS-004 |

Within the Deferred set, GDI-005, GDI-007, CCD-003, CCD-004, CCD-007, CCD-009, and SYS-003 are future read-only exploratory candidates. SES-001, SES-002, and direct SYS-004 are candidates for removal from the first cell. Keeping any of these rows in Allowlist V2 does not authorize their execution.

### 9.4.1 Human Technical Decision Stage 1 per row

`APPROVE_FIRST_CELL` selects the exact reviewed row for the proposed first cell after all remaining gates and a separate phase-specific authorization. `DEFER` excludes the row from that first cell. Neither decision makes a row executable under this record.

| Row ID | Technical classification | Stage 1 Human decision | Effect at this stage |
| --- | --- | --- | --- |
| P1A-GDI-001 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-GDI-002 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-GDI-003 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-GDI-004 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-GDI-006 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-CCD-001 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-CCD-002 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-CCD-005 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-CCD-006 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-001 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-002 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-004 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-005 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-007 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-008 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-010 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-011 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-012 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-013 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-014 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SEC-015 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SES-003 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-SYS-002 | Required candidate | `APPROVE_FIRST_CELL` | Selected for the first cell; not executable without later gates. |
| P1A-CCD-008 | Optional candidate | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-SEC-003 | Optional candidate | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-SEC-006 | Optional candidate | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-SEC-009 | Optional candidate | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-SYS-001 | Optional candidate | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-GDI-005 | Deferred / removed from first cell | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-GDI-007 | Deferred / removed from first cell | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-CCD-003 | Deferred / removed from first cell | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-CCD-004 | Deferred / removed from first cell | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-CCD-007 | Deferred / removed from first cell | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-CCD-009 | Deferred / removed from first cell | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-SES-001 | Deferred / removed from first cell | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-SES-002 | Deferred / removed from first cell | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-SYS-003 | Deferred / removed from first cell | `DEFER` | Excluded from the first cell; not approved for execution. |
| P1A-SYS-004 | Deferred / removed from first cell | `DEFER` | Excluded from the first cell; not approved for execution. |

### 9.5 Flag decisions outside the proposed allowlist

Human Flag Decision Stage status: `HUMAN_FLAG_DECISIONS_COMPLETE`.

These five decisions are individually recorded Human decisions. They are not a wildcard approval, do not add an API row, and do not authorize source creation, build, or execution.

| Decision ID | Flag / query | Human decision | Human-approved reason |
| --- | --- | --- | --- |
| F-01 | `EDS_ROTATEDMODE` | `REJECT_FIRST_CELL` | Current orientation can be observed from current mode and CCD, so the first cell does not need to enumerate every rotated mode. |
| F-02 | `QDC_ALL_PATHS` | `REJECT_FIRST_CELL` | The first cell targets active topology and does not include high-cost inactive-path exploration. |
| F-03 | `QDC_VIRTUAL_REFRESH_RATE_AWARE` | `REJECT_FIRST_CELL` | It will be handled in a separate Windows 11 cell and is not included in this Windows 10 first cell. |
| F-04 | `QDC_INCLUDE_HMD` | `REJECT` | HMD and specialized displays are outside the initial scope. |
| F-05 | `QDC_ONLY_ACTIVE_PATHS` without `QDC_VIRTUAL_MODE_AWARE` | `REJECT_FIRST_CELL` | The approved first-cell query is fixed to the exact combination `QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_MODE_AWARE`. |

### 9.6 First-cell exact display-query flag surface

The Human decisions in Sections 9.4.1 and 9.5 constrain the first-cell display-query surface to the following exact calls. This surface remains non-executable until every later gate and separate execution authorization is complete.

| API family | Exact first-cell surface | Binding to approved rows |
| --- | --- | --- |
| `EnumDisplayDevicesW` | `dwFlags = 0`; or `EDD_GET_DEVICE_INTERFACE_NAME` only for its separately approved interface-identity purpose | `dwFlags = 0` only in P1A-GDI-001/002; `EDD_GET_DEVICE_INTERFACE_NAME` only in P1A-GDI-003 |
| `EnumDisplaySettingsExW` | `ENUM_CURRENT_SETTINGS` with flags `0`; indexed normal enumeration with flags `0` | P1A-GDI-004 and P1A-GDI-006 only |
| CCD active query pair | Exact combination `QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_MODE_AWARE` for both sizing and query | P1A-CCD-001 and P1A-CCD-002 only |

The following flags and query surfaces are not usable in the first cell:

- `ENUM_REGISTRY_SETTINGS`, because P1A-GDI-005 is `DEFER`.
- `EDS_RAWMODE`, because P1A-GDI-007 is `DEFER`.
- `EDS_ROTATEDMODE`, by F-01 `REJECT_FIRST_CELL`.
- `QDC_ALL_PATHS`, by F-02 `REJECT_FIRST_CELL`.
- `QDC_VIRTUAL_REFRESH_RATE_AWARE`, by F-03 `REJECT_FIRST_CELL`.
- `QDC_INCLUDE_HMD`, by F-04 `REJECT`.
- `QDC_ONLY_ACTIVE_PATHS` without `QDC_VIRTUAL_MODE_AWARE`, by F-05 `REJECT_FIRST_CELL`.
- `QDC_DATABASE_CURRENT`, because P1A-CCD-003 and P1A-CCD-004 are both `DEFER`; its API rows are not part of the first-cell surface.

## 10. Forbidden-call Policy

Version: `PHASE1A-FORBIDDEN-V2`

Status: `HUMAN_TECHNICAL_DECISION_APPROVED`

Stage 1 Human decision: `APPROVE`

History: `PHASE1A-FORBIDDEN-V1` was never executed or human-approved and is superseded by V2 before human approval. V2 retains every V1 prohibition and adds the bypass and provenance controls below.

The rule applies to direct calls, wrappers, re-exports, macros, dynamic lookup, generated bindings invoked by source, dependency callbacks configured by Phase 1A source, build/link configuration, and binary imports. Presence in the `windows` crate or an enabled feature alone is not a violation.

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

### 10.1 Manual binding bypass

Every implementation of an approved row must use its reviewed `windows` 0.62.2 binding. Phase 1A source must not bypass that callsite through manual `extern "system"`, manual `extern "stdcall"`, `#[link(...)]`, hand-written Win32 FFI, raw function-pointer tables, alternate Win32 binding crates, `windows-sys`, bindgen or equivalent binding generation, or an inline/generated FFI wrapper not tied to an individually approved row. SID/ACL/token mutation, arbitrary caller-provided SID parsing, and out-of-buffer SID-pointer dereference remain forbidden.

### 10.2 Dynamic resolution bypass

Phase 1A source must not use or construct `GetModuleHandleA`, `GetModuleHandleW`, `GetModuleHandleExA`, `GetModuleHandleExW`, any `LoadLibrary` family member, `GetProcAddress`, `LdrLoadDll`, `LdrGetProcedureAddress`, equivalent loader APIs, `libloading` or another dynamic-loader crate, manual PE export lookup, or delay/manual function resolution.

### 10.3 Build/link injection

Without separate human review and approval, the root project must not contain or use `build.rs`, a custom linker wrapper, custom native library, unapproved `.cargo/config` or `.cargo/config.toml`, rustflags, `-C link-arg`, additional `.lib`/`.dll` injection, a custom linker script, proc-macro Win32-call generation, or code generation that creates FFI outside an approved row.

A transitive dependency build script is not automatically forbidden, but Static Audit V2 must inventory its package, version, artifact/source hash, purpose, and emitted link/config output for human review. Runtime/import provenance never grants source authorization.

## 11. Static Audit Plan

Version: `PHASE1A-STATIC-AUDIT-V2`

Status: `HUMAN_TECHNICAL_DECISION_APPROVED`

Stage 1 Human decision: `APPROVE`

Execution status for every item: `NOT RUN`

History: `PHASE1A-STATIC-AUDIT-V1` was never executed or human-approved and is superseded by V2 before human approval.

The dependency graph MUST be captured against the committed `Cargo.toml`, committed `Cargo.lock`, exact approved default-feature decision, exact approved `windows` feature set, and exact target under the same configuration as the approved build. The planned command is:

```text
cargo tree --locked -e features --target x86_64-pc-windows-msvc
```

The root-manifest feature conditions and resolved graph must match the approved configuration. `cargo tree --locked --all-features` is not pass evidence; if separately authorized, it is labeled `NON_AUTHORIZATION_EXPLORATORY_AUDIT` and kept outside the Phase 1A execution-artifact pass evidence.

| Requirement | Audit | Planned evidence / pass condition | Status |
| --- | --- | --- | --- |
| `MUST` | Win32 callsite inventory | AST plus manual inventory equals individually approved row callsites; no generic Win32 invocation path | `NOT RUN` |
| `MUST` | Forbidden API reference scan | Review source/config/build/generated-use paths; unauthorized callable reference count is zero | `NOT RUN` |
| `MUST` | Mutation flag scan | Source call construction has zero `CDS_*`/`SDC_*` and zero rejected QDC/EDS flags | `NOT RUN` |
| `MUST` | `unsafe` block inventory | 100% file/line/row/invariant/owner list | `NOT RUN` |
| `MUST` | FFI boundary inventory | Raw pointer/handle/union cannot escape to safe domain/evidence layers | `NOT RUN` |
| `MUST` | Manual extern / `#[link]` scan | Zero manual `extern "system"`/`extern "stdcall"`, `#[link]`, hand-written or generated unauthorized FFI | `NOT RUN` |
| `MUST` | Alternate binding scan | Zero `windows-sys`, alternate binding, bindgen, raw function-table, or unapproved generated-wrapper bypass | `NOT RUN` |
| `MUST` | Cargo exact features/default features | Exact seven `windows` features and exact approved default-feature decision; no unauthorized root feature | `NOT RUN` |
| `MUST` | Filesystem write scan | Source and dataflow have zero file write/create/append/truncate/persist path; stdout only | `NOT RUN` |
| `MUST` | Registry write scan | API/access-mask/crate/string scan has zero registry write path | `NOT RUN` |
| `MUST` | Process spawn scan | Rust, crate, Win32, shell, and script-host spawn paths are zero | `NOT RUN` |
| `MUST` | Process termination scan | Terminate/job/APC/remote-thread control paths are zero | `NOT RUN` |
| `MUST` | Named object scan | Create/open/wait and `Global\\`/`Local\\` object paths are zero | `NOT RUN` |
| `MUST` | Tauri/React/WebView scan | Dependency/source/config paths are zero | `NOT RUN` |
| `MUST` | PowerShell/cmd/script-host scan | Source/config/script/artifact strings and invocation paths are zero | `NOT RUN` |
| `MUST` | Dynamic loading scan | `LoadLibrary*`, `GetProcAddress`, dynamic-loader crates, manual export resolution, and function-pointer paths are zero | `NOT RUN` |
| `MUST` | `GetModuleHandle` / `Ldr*` scan | `GetModuleHandleA/W/ExA/ExW`, `LdrLoadDll`, `LdrGetProcedureAddress`, and equivalents are zero | `NOT RUN` |
| `MUST` | Inline/global assembly scan | `asm!`, `global_asm!`, source/build/rustflag assembly paths are zero | `NOT RUN` |
| `MUST` | Root `build.rs` absence | Root build script is absent unless separately reviewed and approved | `NOT RUN` |
| `MUST` | Transitive build-script inventory | Package, version, hash, purpose, emitted link/config output, and human review result for every transitive build script | `NOT RUN` |
| `MUST` | `.cargo/config*` inventory | Every config file, target override, linker, runner, alias, and source replacement inventoried; unauthorized entry zero | `NOT RUN` |
| `MUST` | rustflags/link-arg inventory | Root/environment/config/build output has no unauthorized rustflag, `-C link-arg`, native lib, or linker script | `NOT RUN` |
| `MUST` | Proc-macro inventory | Every proc macro and generated output provenance reviewed; no Win32 call/FFI generation outside an approved row | `NOT RUN` |
| `MUST` | Exact dependency tree | `cargo tree --locked -e features --target x86_64-pc-windows-msvc` equivalent under approved build conditions; graph matches | `NOT RUN` |
| `MUST` | Source hashes | Approved commit source/manifest/config per-file hashes plus aggregate | `NOT RUN` |
| `MUST` | Cargo.lock hash/review | Raw-byte hash, resolver/source/checksum, and unexpected-package review | `NOT RUN` |
| `MUST` | Final PE hash | Execution-target PE raw-byte hash bound to source, lockfile, toolchain, target, features, and build command | `NOT RUN` |
| `MUST` | Final PE import table | Import table from the execution-target PE itself; mutation import count zero | `NOT RUN` |
| `MUST` | PE import provenance | Apply `PHASE1A-PE-IMPORT-PROVENANCE-V1`; every import classified and explained or fail | `NOT RUN` |
| `MUST` | Source-to-binary binding | Bind exact source, lockfile, toolchain, target, features, build command, PE hash, import table, and provenance record | `NOT RUN` |
| `SHOULD` | `cargo fmt --check` | Approved rustfmt version; diff zero | `NOT RUN` |
| `SHOULD` | Clippy | Exact target/features/lint policy; no unreviewed warning | `NOT RUN` |
| `SHOULD` | Dependency license/advisory snapshot | Snapshot the exact resolved graph; advisory absence alone is not a safety claim | `NOT RUN` |
| `OPTIONAL` | Byte-for-byte reproducible binary | Not an initial Phase 1A blocker; exact source/lock/toolchain/target/features/build command remain MUST evidence | `NOT RUN` |

Source scanning and final-PE import review are both MUST. Source scanning detects dynamic/build/config bypasses and dead unauthorized code; PE review detects generated wrappers, linker effects, and transitive/runtime imports. Neither substitutes for the other.

### 11.1 Binary Import Provenance Rule

Version: `PHASE1A-PE-IMPORT-PROVENANCE-V1`

Status: `HUMAN_TECHNICAL_DECISION_APPROVED`

Stage 1 Human decision: `APPROVE`

Execution status: `NOT RUN`

Every final-PE import must be assigned exactly one provenance class: `APPROVED_DIRECT`, `WINDOWS_BINDING_RUNTIME`, `RUST_RUNTIME`, `TRANSITIVE_DEPENDENCY`, `TOOLCHAIN_RUNTIME`, or `UNEXPLAINED`.

If the execution-target PE imports any display-mutation capability, including `ChangeDisplaySettings*`, `SetDisplayConfig`, `DisplayConfigSetDeviceInfo`, a DDC/CI setter, or an HDR/WCG/display setter, the result is `STATIC_AUDIT_FAIL` regardless of provenance.

For another source-direct-allowlist-external import such as runtime `WriteFile`, `GetLastError`, or another runtime import, do not automatically approve it and do not automatically treat it as a violation. The audit must prove all of the following:

1. Import-owner artifact.
2. Originating package or runtime.
3. Dependency path.
4. No direct call from Phase 1A source.
5. Role in the approved Phase 1A dataflow.
6. No source-reachable path to a forbidden operation.

An import that cannot meet all six requirements is `UNEXPLAINED` and causes `STATIC_AUDIT_FAIL`. Runtime provenance is explanation evidence only; it never authorizes a source callsite.

This plan does not authorize running Cargo, compiling, or producing a binary now.

## 12. Evidence Plan

Version: `PHASE1A-EVIDENCE-V2`

Status: `HUMAN_TECHNICAL_DECISION_APPROVED`

Stage 1 Human decision: `APPROVE`

History: `PHASE1A-EVIDENCE-V1` was never executed or human-approved and is superseded by V2 before human approval. V2 aligns mandatory capture with the proposed first-cell rows; it does not approve those rows.

Evidence is one immutable bundle per run. Failed/partial runs are retained as separate IDs and never overwritten by a retry. The future source emits bounded sanitized JSON to stdout; the human-approved capture mechanism, storage, retention, access, and immutable ID remain human decisions.

### 12.1 `MUST_CAPTURE`

- Authorization and cell identity: Evidence ID; policy versions; authorization ID; approved Machine ID; Windows edition/version/build; KB collection coverage; ESU decision.
- Hardware/environment: CPU architecture; GPU model and driver; display model/firmware; connection/port/dock; local-console/RDP state; multi-user/Fast User Switching state.
- Privilege: administrator membership; elevation; integrity; effective privilege profile.
- Current display: resolution; rational refresh; orientation; bits per pixel.
- Repository/source: branch; source commit full SHA; Design Baseline SHA; source hashes; `Cargo.toml` hash; `Cargo.lock` hash; exact dependency graph.
- Toolchain/build identity: rustc and Cargo; MSVC/linker/Windows SDK; exact `windows` crate version; exact Cargo feature/default-feature decision.
- Call trace: API row IDs; sequence; exact flags/query classes; return/result/error; retry/count/cap decisions; per-call elapsed/timebox result.
- Process output: exit code; sanitized stdout content and hash; `truncation=false`; stderr hash and size.
- Required results: GDI adapters/monitors/current mode/normal modes; GDI-to-CCD mapping and `AMBIGUOUS/UNMAPPED`; CCD active topology; every exclusion/failure/cap/hotplug/access-denied result and the most-severe result.

Allowed port value when the OS/driver cannot expose a trusted number: `GPU-side port number: NOT EXPOSED`.

### 12.2 `USEFUL_RESTRICTED`

- Complete Design Baseline per-file hashes copied into each run bundle; the aggregate remains MUST.
- Scale observation with collection method.
- Token elevation type.
- User SID bundle-scoped hash, only after SEC-015/SEC-010/full-range validation.
- Authentication LUID bundle-scoped hash.
- Adapter-path bundle-scoped hash.
- Raw absolute `GetTickCount64` value, restricted and subject to Redaction V2.
- UTC correlation timestamp, restricted/coarsened under Evidence Owner policy.
- Sanitized stderr body after secondary review; stderr hash and size remain MUST.

### 12.3 `DEFER`

- Registry/persisted mode.
- Raw modes.
- Database topology and topology database ID.
- Preferred mode.
- Advanced-color/HDR API result.
- PID export; first-cell provenance is normalized to `CURRENT_PROCESS` instead.
- Global UAC policy; token-derived facts do not prove the global policy.

If HDR, advanced color, or another deferred environment fact is recorded as a manual environment note, its collection method must be explicit and it must not be represented as approved runtime-API evidence.

### 12.4 Boundedness and result rules

- Overall maximum JSON remains 8 MiB; adapter<=32; monitor<=32/adapter; mode<=4,096/list/adapter; CCD path<=64/mode<=256/combined checked allocation<=1 MiB; token buffer<=65,536; retries remain bounded by the allowlist row/common rules.
- No mapping by label similarity. Exactly one complete observation is required to claim an exact map.
- Missing, contradictory, truncated, cap-exceeded, invalid UTF-16/union/enum/SID, access-denied, remote/non-console, or topology-race exhaustion is preserved and fails closed where the row contract requires it.
- Environment data collected manually or through a method not represented by an approved API row is labeled with its collection method and is never treated as source-verified runtime authority.
- Installed-KB evidence must state its coverage. `Win32_QuickFixEngineering`, if separately approved later, cannot be labeled a complete Windows update inventory.

## 13. Redaction Policy

Version: `PHASE1A-REDACTION-V2`

Status: `HUMAN_TECHNICAL_DECISION_APPROVED`

Stage 1 Human decision: `APPROVE`

History: `PHASE1A-REDACTION-V1` was never executed or human-approved and is superseded by V2 before human approval.

`HASH_SHA256` means a domain-separated, evidence-bundle-scoped salted SHA-256 over the raw field bytes. The same salt/domain is used only where equality within one bundle is required; the salt and raw mapping are held separately by the Evidence Owner under the still-pending access policy. It is not a durable product identifier. `MASK` replaces the identifying portion with a stable bundle-local alias. `HUMAN_REVIEW` defaults to removal from any shared bundle unless the Reviewer records why a sanitized value is necessary.

| Field | Classification | Rule / rationale |
| --- | --- | --- |
| User SID | `HASH_SHA256` | Preserve same-user equality within bundle; never emit raw SID. |
| Username / domain account name | `DROP` | Not needed for technical validation. |
| Computer name | `DROP` | Use the human-issued Machine ID alias for run/cell correlation. A restricted internal bundle may use a scoped hash only after an explicit Human Reviewer decision. |
| Machine ID | `MASK` | Use human-issued evidence alias, not hardware/host identifier. |
| Repository absolute path | `MASK` | Replace root with `$REPO`; preserve relative path. |
| Exact command | `MASK` | Keep arguments/flags; replace absolute paths and user names. |
| Windows device path | `HASH_SHA256` | Preserve cross-map equality, not raw path. |
| GDI `DeviceName` | `HASH_SHA256` | Session-scoped cross-map key. |
| GDI `DeviceString` | `HUMAN_REVIEW` | `KEEP` only when clearly a hardware model; `MASK` or `DROP` a custom, username-like, serial-like, or user-assigned value. |
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
| Process ID | `DROP` | Do not export the PID in first-cell evidence; normalize provenance to `CURRENT_PROCESS`. SES-001/002 are deferred/removal candidates. |
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
| Orientation/scaling/bpp/advanced-color booleans | `KEEP` | Keep only a technical value collected under its approved row or explicitly labeled manual method; this rule does not make deferred fields mandatory. |
| API function/row ID/flags | `KEEP` | Required audit evidence. |
| API/Win32 error code | `KEEP` | Required troubleshooting/result classification. |
| Elapsed milliseconds | `KEEP` | Required timebox evidence; no absolute tick is needed in the shared bundle. |
| Exit code | `KEEP` | Required bounded execution result. |
| Raw absolute `GetTickCount64` value | `HUMAN_REVIEW` | Restricted evidence only; `DROP` from the shared bundle. |
| UTC timestamp | `HUMAN_REVIEW` | Keep only as required by Evidence Owner run-correlation policy, preferably coarsened. |
| Raw stdout/stderr | `HUMAN_REVIEW` | Serializer should already sanitize; scan again before sharing. |

### 13.1 Technical revision version summary

| Artifact | Version | Status | Stage 1 Human decision |
| --- | --- | --- | --- |
| API allowlist | `PHASE1A-ALLOWLIST-V2` | `HUMAN_TECHNICAL_DECISIONS_COMPLETE`; 0 rows executable | 23 individual `APPROVE_FIRST_CELL`; 15 individual `DEFER` |
| Forbidden-call policy | `PHASE1A-FORBIDDEN-V2` | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE` |
| Static audit plan | `PHASE1A-STATIC-AUDIT-V2` | `HUMAN_TECHNICAL_DECISION_APPROVED`; all audits `NOT RUN` | `APPROVE` |
| PE import provenance | `PHASE1A-PE-IMPORT-PROVENANCE-V1` | `HUMAN_TECHNICAL_DECISION_APPROVED`; `NOT RUN` | `APPROVE` |
| Evidence plan | `PHASE1A-EVIDENCE-V2` | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE` |
| Redaction policy | `PHASE1A-REDACTION-V2` | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE` |
| Flag decisions | F-01 through F-05 | `HUMAN_FLAG_DECISIONS_COMPLETE` | Five individual Human decisions in Section 9.5; no wildcard |
| External hung-call procedure | `PHASE1A-HUNG-CALL-PROCEDURE-V1` | `PROPOSED_FOR_HUMAN_APPROVAL` | `PENDING_HUMAN_APPROVAL` |

## 14. Execution Privilege

| Required field | Current value / collection |
| --- | --- |
| Account administrator-group membership | `PENDING_MACHINE_OBSERVATION`; SEC-004/015/010/011, including deny-only attribute |
| Process elevated | `PENDING_MACHINE_OBSERVATION`; SEC-005 |
| Integrity level | `PENDING_MACHINE_OBSERVATION`; SEC-008/015/010/012/013 |
| UAC state | `PENDING_MACHINE_OBSERVATION`; record only token-derived elevation type/limited evidence from SEC-006 and explicitly identified manual evidence; do not infer global UAC policy when it is not provable |
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

## 16. Remaining Human Inputs and Completion Gates

Human Technical Decision Stage 1 resolved the toolchain, crate, Cargo-feature, 38-row, and five policy-version technical decisions recorded above. The Human Flag Decision Stage resolved F-01 through F-05 individually. Neither stage supplies machine facts, people, evidence storage, immutable authorization bindings, source/build evidence, or phase-specific authorization.

### 16.1 `PHASE1A-HUNG-CALL-PROCEDURE-V1`

| Field | Value |
| --- | --- |
| Procedure name/version | `PHASE1A-HUNG-CALL-PROCEDURE-V1` |
| Procedure status | `PROPOSED_FOR_HUMAN_APPROVAL` |
| Human decision | `PENDING_HUMAN_APPROVAL` |
| Source capability effect | None; this proposal does not authorize or justify any process-spawn, watchdog, worker, cancellation, or termination capability in Phase 1A source. |

Principles:

- Phase 1A source does not kill or forcibly cancel its own hung call.
- Phase 1A source must not contain a watchdog process, worker process, process spawn, `TerminateProcess`, Job Object termination, thread termination, forced DLL unload, or shell/PowerShell kill path.
- A read-only Win32 call that never returns is not a successful run even though no display-mutation API was called.

Proposed Human/operator procedure when a Win32 call does not return and ordinary interaction indicates that the process cannot finish normally:

1. Do not initiate another display operation on the executing PC.
2. Do not disconnect/reconnect a display cable or change resolution or refresh rate.
3. Record the run start time and the last externally identifiable API Row ID.
4. Preserve stdout, stderr, and other evidence already captured outside the Phase 1A process.
5. If the Operator attempts an ordinary external application close, record the method and result.
6. If ordinary close cannot end the process, classify the run as `HUNG_CALL_ABORTED_EXTERNALLY`.
7. If external termination through Task Manager or another operating-system facility is necessary, record it explicitly as a Human incident action, separate from Phase 1A source capability.
8. Do not classify the externally terminated run as successful.
9. Do not retry with the same Evidence ID.
10. Issue a new Evidence ID for any authorized retry.
11. Do not rerun the same API row without limit or without root-cause review.
12. If display state appears abnormal, stop Phase 1A and return the matter to the Reviewer.

Approval condition: this procedure remains only a proposal until an identified Human Approver records approval in the immutable authorization state. Its approval cannot expand the Phase 1A source allowlist or weaken `PHASE1A-FORBIDDEN-V2`.

### 16.2 Classification of the remaining inputs

The classifications below prevent source-creation prerequisites from being mixed with target-machine observations acquired after source/build or immediately before execution:

- `HUMAN_ENTERED`: a Human supplies an alias, identity, location, schedule, evidence-storage value, or record identifier/reference. It is not inferred from the machine.
- `MACHINE_OBSERVED_LATER`: an exact target-machine or execution-profile fact must be captured and verified later; a placeholder, planned value, or product assumption is not evidence.
- `HUMAN_DECISION_ON_MACHINE_EVIDENCE`: a Human must make an explicit decision or authorization after reviewing the applicable machine evidence and, where relevant, frozen record/source/binary evidence. A machine observation alone cannot populate the decision.

#### MACHINE / CELL — 21 remaining inputs

| ID | Remaining input | Classification | Required timing / condition |
| --- | --- | --- | --- |
| M-01 | Target Machine ID / evidence alias | `HUMAN_ENTERED` | Gate A; use a Human-issued alias, not a guessed host or hardware identifier. |
| M-02 | Windows edition | `MACHINE_OBSERVED_LATER` | Exact target-machine evidence before Gate F. |
| M-03 | Windows version | `MACHINE_OBSERVED_LATER` | Exact target-machine evidence before Gate F. |
| M-04 | OS build | `MACHINE_OBSERVED_LATER` | Exact target-machine evidence before Gate F. |
| M-05 | Installed KB evidence and stated collection coverage | `MACHINE_OBSERVED_LATER` | Preserve coverage limits; do not call an incomplete source complete. |
| M-06 | ESU status/evidence | `MACHINE_OBSERVED_LATER` | Preserve `NOT CONFIRMED` unless later evidence proves a different value. |
| M-07 | Acceptance or rejection of the exact non-ESU-confirmed first cell | `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | Explicit decision after reviewing M-02 through M-06. |
| M-08 | CPU architecture | `MACHINE_OBSERVED_LATER` | Exact target-machine evidence; the planned x64 shape is not the value. |
| M-09 | GPU exact model | `MACHINE_OBSERVED_LATER` | Exact target-machine evidence. |
| M-10 | GPU driver exact version/provider/date | `MACHINE_OBSERVED_LATER` | Exact target-machine evidence. |
| M-11 | Display exact model | `MACHINE_OBSERVED_LATER` | Separate model from serial/custom label. |
| M-12 | Display firmware | `MACHINE_OBSERVED_LATER` | Exact value or explicit unavailable observation. |
| M-13 | Connection type | `MACHINE_OBSERVED_LATER` | Exact target-machine/display-path evidence. |
| M-14 | Physical port | `MACHINE_OBSERVED_LATER` | Exact value or `GPU-side port number: NOT EXPOSED`. |
| M-15 | Dock / adapter / cable path | `MACHINE_OBSERVED_LATER` | Exact value, including explicit none where true. |
| M-16 | Current resolution | `MACHINE_OBSERVED_LATER` | Capture immediately before the authorized run. |
| M-17 | Current rational refresh | `MACHINE_OBSERVED_LATER` | Numerator/denominator plus rendered label immediately before the authorized run. |
| M-18 | HDR / advanced-color observation and collection method | `MACHINE_OBSERVED_LATER` | Manual/environment evidence only unless a later row is separately approved; do not represent it as first-cell CCD-009 evidence. |
| M-19 | Scale observation and collection method | `MACHINE_OBSERVED_LATER` | Manual/environment evidence; no scale API row is approved here. |
| M-20 | Local console / RDP state | `MACHINE_OBSERVED_LATER` | Exact execution-time state; remote/non-console blocks the run. |
| M-21 | Multi-user / Fast User Switching / second interactive state | `MACHINE_OBSERVED_LATER` | Exact execution-time state; unknown or conflicting state blocks the run. |

#### PRIVILEGE — 5 remaining inputs

| ID | Remaining input | Classification | Required timing / condition |
| --- | --- | --- | --- |
| P-01 | Administrators-group membership and deny-only state | `MACHINE_OBSERVED_LATER` | Token-derived evidence before Gate F. |
| P-02 | Process elevated | `MACHINE_OBSERVED_LATER` | Token-derived boolean before Gate F. |
| P-03 | Integrity | `MACHINE_OBSERVED_LATER` | Validated RID and normalized label before Gate F. |
| P-04 | UAC evidence | `MACHINE_OBSERVED_LATER` | Keep token-derived and manually obtained facts distinct; do not infer global policy. |
| P-05 | Effective execution profile | `MACHINE_OBSERVED_LATER` | Derive from the exact privilege evidence; elevated, inconsistent, or unknown blocks the run. |

#### PEOPLE — 4 remaining inputs

| ID | Remaining input | Classification | Required timing / condition |
| --- | --- | --- | --- |
| H-01 | Operator | `HUMAN_ENTERED` | Identified person/role; do not infer a name. |
| H-02 | Evidence Owner | `HUMAN_ENTERED` | Identified person/role; do not infer a name. |
| H-03 | Reviewer | `HUMAN_ENTERED` | Identified person/role and required independence decision. |
| H-04 | Approver | `HUMAN_ENTERED` | Identified person/role and required independence decision. |

#### EVIDENCE — 4 remaining inputs

| ID | Remaining input | Classification | Required timing / condition |
| --- | --- | --- | --- |
| E-01 | Evidence ID | `HUMAN_ENTERED` | Approved immutable scheme/value; a retry receives a new value. |
| E-02 | Evidence location | `HUMAN_ENTERED` | Approved fixed location and external capture mechanism; do not guess a path or silently invent a capture path. |
| E-03 | Retention period | `HUMAN_ENTERED` | Explicit duration or terminal rule. |
| E-04 | Access principals | `HUMAN_ENTERED` | Exact identified people/groups. |

#### EXECUTION — 2 remaining inputs

| ID | Remaining input | Classification | Required timing / condition |
| --- | --- | --- | --- |
| X-01 | Physical execution location | `HUMAN_ENTERED` | Exact lab/location. |
| X-02 | Planned execution date/timezone | `HUMAN_ENTERED` | Exact date/time and timezone. |

#### AUTHORIZATION — 11 remaining inputs

| ID | Remaining input | Classification | Required timing / condition |
| --- | --- | --- | --- |
| AR-01 | Approval ID | `HUMAN_ENTERED` | Immutable Human-issued ID; this record does not issue it. |
| AR-02 | Reviewer decision | `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | Explicit decision on the evidence applicable to the requested gate. |
| AR-03 | Approver decision | `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | Explicit decision distinct from record completion. |
| AR-04 | Approved Target Machine ID | `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | Exact binding to M-01 and, for execution, its reviewed machine evidence. |
| AR-05 | Approved immutable Execution Record commit full SHA | `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | Gate B binding; the current working tree and Repository Baseline ID are not substitutes. |
| AR-06 | Approved policy versions and hashes | `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | Bind the exact Allowlist, Forbidden, Static Audit, PE provenance, Evidence, Redaction, flag-decision, and hung-procedure state. |
| AR-07 | Approved source-creation scope | `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | Separately authorize the exact Cargo/source files and exclusions after Gate B. |
| AR-08 | Explicit exclusions and stop conditions | `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | Bind the no-mutation, no-spawn/kill, no-watchdog, no-shell, and other applicable limits. |
| AR-09 | Hung-call procedure approval | `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | Explicitly approve or reject `PHASE1A-HUNG-CALL-PROCEDURE-V1`; currently `PENDING_HUMAN_APPROVAL`. |
| AR-10 | Signature / signed-record reference | `HUMAN_ENTERED` | Human-provided immutable signature/reference; this record does not proxy-enter it. |
| AR-11 | Phase-specific authorization | `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | Source creation, build/static audit, and target-machine API execution require distinct explicit scope decisions. |

#### Remaining-input count

| Classification | Count |
| --- | ---: |
| `HUMAN_ENTERED` | **13** |
| `MACHINE_OBSERVED_LATER` | **24** |
| `HUMAN_DECISION_ON_MACHINE_EVIDENCE` | **10** |
| **Total remaining Human inputs** | **47** |

Count basis: MACHINE/CELL 21 + PRIVILEGE 5 + PEOPLE 4 + EVIDENCE 4 + EXECUTION 2 + AUTHORIZATION 11 = 47. This is the normalized remaining-input inventory for the current record; it does not reuse the prior review's original 52-entry structure as a subtraction formula. Stage 1 technical decisions and F-01 through F-05 are complete and therefore are not counted again. The external capture mechanism is bound with E-02, and hung-run failure handling is bound with AR-08/AR-09. Grouped authorization outputs such as policy hashes remain one input row but must bind every listed exact version/hash; grouping is not wildcard policy approval.

### 16.3 Authorization sequencing and completion gates

The gates are ordered and non-substitutable. Passing one gate never starts or authorizes a later gate.

#### Gate A — Human metadata

Complete and review:

- Target Machine alias M-01.
- People H-01 through H-04.
- Evidence storage E-01 through E-04.
- Execution location/date X-01 and X-02.
- Existing Stage 1 technical and F-01 through F-05 policy decisions.
- `PHASE1A-HUNG-CALL-PROCEDURE-V1` Human decision.

Current result: `NOT COMPLETE`, because the Human metadata and hung-call procedure approval remain pending.

#### Gate B — Authorization Record Freeze

After Gate A and the exact source-creation scope, exclusions, stop conditions, and other decisions needed for source-creation authorization are complete, freeze an authorization-record state in a Git commit that includes at least:

- `docs/phase-1a-execution-record.md`
- `docs/phase-1a-human-review.md`
- `docs/phase-1a-human-rereview.md`

Capture and bind the full commit SHA, the raw-byte SHA-256 of this Execution Record, and the exact policy hashes. This commit is the `Phase 1A source creation authorization baseline`; it is not the source baseline, execution binary baseline, or execution authorization.

Current result: `NOT REACHED`. No Git add, commit, or push is authorized by this update.

#### Gate C — Phase 1A source creation

Only after a separate Human source-creation authorization bound to Gate B may the exact approved scope create `Cargo.toml`, `Cargo.lock`, and Rust source. The authorization must not be inferred from Stage 1 decisions, flag decisions, record completion, or the existence of an implementation-plan phase.

Current result: `NOT AUTHORIZED`.

#### Gate D — Build / Static Audit

Only after source completion and a separate build/static-audit authorization may the approved scope run build, test, rustfmt/Clippy evidence, Static Audit V2, dependency review, and PE hash/import/provenance work.

Current result: `NOT AUTHORIZED`; every Static Audit V2 item remains `NOT RUN`.

#### Gate E — Source/Binary Baseline Freeze

After Gate D passes, freeze the reviewed source and build artifact using a distinct source commit/full SHA and source/config/manifest/lock hashes, plus the execution-target PE hash, imports, provenance, toolchain, target, features, and exact build command. This is a separate identity from Gate B.

Current result: `NOT REACHED`; no source or binary exists under this record.

#### Gate F — Target Machine execution authorization

Review the exact Target Machine and privilege observations, Gate E evidence, remaining Reviewer/Approver decisions, Approval ID, signature/reference, explicit exclusions, and stop conditions. Only then may a Human issue the final, machine-bound Phase 1A execution authorization. Windows API execution is permitted only after that authorization and only for the exact frozen binary/evidence/cell.

Current result: `NOT AUTHORIZED`; Phase 1A execution remains `NOT EXECUTABLE`.

### 16.4 Non-equivalent decisions and identities

Authorization decisions:

```text
Stage 1 Human technical approval
!= source creation authorization
!= build/static-audit authorization
!= target-machine execution authorization
```

Artifact identities:

```text
Repository Baseline P1A-REPO-09be6a3
!= Authorization Record commit/full SHA
!= Source Baseline commit/full SHA
!= execution Binary hash
```

The Repository Baseline identifies the earlier frozen repository state. The Authorization Record commit identifies what may later be created and audited. The Source Baseline identifies the reviewed source/config/dependency state. The Binary hash identifies the exact execution artifact. None transfers authority to another identity.

## 17. Technical Preflight Status

| Item | Status | Reason |
| --- | --- | --- |
| Repository baseline | `FROZEN` | `P1A-REPO-09be6a3`: `main` at full HEAD `09be6a3e05651b9587d526c2d57e542823ec9297`; clean tree/index, no untracked files, required baseline files in HEAD. |
| Design Baseline SHA V1 | `FROZEN` | Recalculated aggregate matches the previous value and is tied to the frozen repository baseline. |
| Rust toolchain | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE`: stable 1.97.1 / `x86_64-pc-windows-msvc` / minimal, rustfmt, and Clippy. Target-machine verification, installation, and execution remain unauthorized. |
| windows crate | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE`: exact `windows = "=0.62.2"`; no Cargo resolution, source, compile, or execution. |
| Cargo features | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE` for `default-features = true`; `APPROVE_ALL_7` for the exact seven-feature set. No resolved graph or build. |
| API allowlist | `HUMAN_TECHNICAL_DECISIONS_COMPLETE` | `PHASE1A-ALLOWLIST-V2`: all 38 rows have individual decisions in Section 9.4.1; 23 `APPROVE_FIRST_CELL`, 15 `DEFER`, 0 executable now. |
| Forbidden list | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE`: `PHASE1A-FORBIDDEN-V2`. |
| Static audit plan | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE`: `PHASE1A-STATIC-AUDIT-V2`; all items remain `NOT RUN`. |
| PE import provenance | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE`: `PHASE1A-PE-IMPORT-PROVENANCE-V1`; remains `NOT RUN`. |
| Evidence plan | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE`: `PHASE1A-EVIDENCE-V2`; evidence storage and execution details remain pending. |
| Redaction policy | `HUMAN_TECHNICAL_DECISION_APPROVED` | `APPROVE`: `PHASE1A-REDACTION-V2`. |
| Human flag decisions | `HUMAN_FLAG_DECISIONS_COMPLETE` | F-01 through F-05 have five individual Human decisions in Section 9.5; the exact first-cell flag surface is fixed in Section 9.6. |
| External hung-call procedure | `PENDING_HUMAN_APPROVAL` | `PHASE1A-HUNG-CALL-PROCEDURE-V1` is `PROPOSED_FOR_HUMAN_APPROVAL`; it has not been approved by Codex or a Human. |
| Overall | `HUMAN_TECHNICAL_DECISIONS_COMPLETE`; `HUMAN_FLAG_DECISIONS_COMPLETE` | Technical preflight and independent Human Re-review preceded the recorded decisions. These remain neither source/build approval nor final phase authorization. |

## 18. Human Authorization

| Field | Value |
| --- | --- |
| Reviewer decision | `PENDING_HUMAN_INPUT` |
| Approver decision | `PENDING_HUMAN_INPUT` |
| Immutable approval ID | `PENDING_HUMAN_INPUT` |
| Signed/reference record | `PENDING_HUMAN_INPUT` |
| Human Technical Decision Stage 1 | `HUMAN_TECHNICAL_DECISIONS_COMPLETE` |
| Human Flag Decision Stage | `HUMAN_FLAG_DECISIONS_COMPLETE` |
| Stage 1 technical authorization subset | The technical selections corresponding to the prior review's A-06 through A-13 are recorded; current remaining inputs are normalized independently in Section 16.2 |
| Remaining input inventory | 47 total: 13 `HUMAN_ENTERED`, 24 `MACHINE_OBSERVED_LATER`, 10 `HUMAN_DECISION_ON_MACHINE_EVIDENCE` |
| Hung-call procedure Human decision | `PENDING_HUMAN_APPROVAL` |
| Primary operational and final-authorization inputs complete | No; Target Machine, machine observations, people, evidence storage, schedule, Approval ID, signature/reference, hung-call approval, immutable commit binding, source/build evidence, and phase-specific authorization remain pending |
| Allowlist row decisions | 38 of 38 recorded individually in Section 9.4.1 |
| Stage 1 first-cell row IDs | 23 individual `APPROVE_FIRST_CELL` decisions in Section 9.4.1 |
| Stage 1 deferred row IDs | 15 individual `DEFER` decisions in Section 9.4.1 |
| Final phase-approved / executable row IDs | None (0) |
| Authorization state | `NOT AUTHORIZED` |

## 19. Record Completion

| Field | Value |
| --- | --- |
| Repository baseline frozen | Yes |
| Technical proposals prepared | Yes |
| Human Technical Decision Stage 1 complete | Yes: `HUMAN_TECHNICAL_DECISIONS_COMPLETE` |
| Human Flag Decision Stage complete | Yes: `HUMAN_FLAG_DECISIONS_COMPLETE`; F-01 through F-05 recorded individually |
| Human fields complete | No |
| Final Human approval complete | No |
| Record status | `INCOMPLETE` |
| Completion reason | Stage 1 technical decisions and the five flag decisions are complete, but the record remains incomplete because 47 operational, machine-evidence, identity, evidence-storage, schedule, hung-procedure, immutable Approval ID/signature, record/source/binary binding, and phase-specific authorization inputs remain pending. |

## 20. Phase Authorization

| Phase | Status |
| --- | --- |
| Phase 1A record | `INCOMPLETE` |
| Phase 1A execution | `NOT EXECUTABLE` |
| Final authorization | `NOT AUTHORIZED` |
| Phase 2A | `NOT EXECUTABLE` |
| Phase 1B | `NOT EXECUTABLE` |
| Phase 2B | `NOT EXECUTABLE` |

Stop point: this file records the preflight, Human Technical Decision Stage 1, five individual Human flag decisions, the proposed unapproved hung-call procedure, the remaining-input inventory, and the Gate A–F sequence only. Stop after this documentation update. Do not create Phase 1A source/Cargo files, install/update a toolchain or crate, run Cargo/build/test/static audit, create a Windows binary, execute a Windows/display API, collect target-machine evidence, run Git add/commit/push, or begin any phase under this record.
