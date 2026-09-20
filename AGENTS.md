# DisplayDeck repository instructions

## Current version scope — owner direction, 2026-09-21

v0.1.0 is a read-only Windows display inventory and diagnostics app. Its scope is display enumeration, current-mode acquisition, read-only UI, diagnostic JSON, fail-closed behavior, installer, and uninstall. Display-setting changes, Apply / Restore, WAL, crash recovery, watchdog, mutation safety, and Gate B belong to v0.2 or later and must not be prerequisites for completing v0.1.0. Existing mutation code and approval records are future-track assets, not v0.1.0 requirements. This scope decision does not authorize new Windows operations, mutation, or a broader release/support claim, and does not change the qualified release 01 artifact. Apply the mutation safety constraints below when working on that future track.

## Current project gate: read-only release complete, mutation-track implementation authorized

The roadmap was reduced to the three human gates in `docs/implementation-plan.md` on 2026-08-24. Gate A / Stage 0 and the non-mutating Stage 1 are complete. On 2026-08-30 the actual D07 inspector returned `NO_GO:DirectoryAnchorUnproven` with `MutationAuthorized: false`; Stage 2 therefore ended with zero display API calls. The read-only Stage 3 NSIS build/install/smoke/uninstall verification is complete. On 2026-08-30 the human owner approved Gate C release 01: the 2,160,426-byte installer with SHA-256 `3307DB604C5C96B4E753D499ECB006E2209695006965F9BA7D65A1BF6F1EFD2F` is the completed read-only MVP for the recorded Windows 10 exact cell.

On 2026-08-30 the human owner explicitly approved `GATE-A-MUTATION-ADDENDUM-01` in `docs/implementation-plan.md`. M1-M3 application/native/config/test changes and display-API-free format, typecheck, unit/process test, and non-mutating build are authorized. These changes form a new candidate and do not alter or overwrite the qualified release 01 artifact.

On 2026-08-30 the human owner explicitly approved revision `GATE-A-MUTATION-ADDENDUM-01-R1`: the same `displaydeck-actor` executable image may implement a fixed-name, demand-start, one-shot LocalSystem SCM handshake for the M1 SYSTEM creator lane. Source/config/test changes and non-mutating cross-compilation are authorized. No SCM command may be executed on Windows yet, and the handshake must not create or write protected machine-data.

On 2026-08-31 the human owner explicitly approved revision `GATE-A-MUTATION-ADDENDUM-01-R2`: source may implement the fixed-path bounded `DisplayDeckProvisionManifestV1` parser/hash binding, detached PKCS#7 and Authenticode-chain verification, Gate-B-pinned publisher certificate SHA-256 and opened actor-image binding, fail-closed provision grant, and Candidate 04 wire/state transitions. The publisher and manifest digests must remain unset/zero, so authorization always denies. Only source/config/test and display-API-free local tests/cross-compilation are authorized; signing, manifest/package artifact generation, Windows signature-verification/SCM/provision execution, and actual machine-data writes remain unauthorized.

Windows provision/install, actual machine-data write, D07/D08 execution, `CDS_TEST`, dynamic apply, display disconnect or physical single-path reconfiguration, Gate B/M4, persistent or multi-display changes, Windows 11 or other support-cell claims, signing, auto-update, public distribution, and mutation release remain unauthorized.

Whenever the next operator action runs on Windows, first record the exact command and stop/continue conditions in `docs/windows-validation-history.md`, add only a concise pointer to `README.md` when the action is current, then commit and push that documentation before asking the operator to run it.

Within the approved M1-M3 implementation lane, agents must not:

- install, update, or remove packages, toolchains, plugins, or system components;
- run packaging, installation, development-server, or Windows operator/display-setting operations;
- execute any display API, including `CDS_TEST`, or write actual Windows machine-data;
- register, start, stop, delete, or otherwise operate the R1 Windows service before the exact Windows command is recorded, committed, pushed, and separately authorized in Gate B;
- change an actual operating-system display setting;
- add a dependency, framework, generic backend abstraction, installer privilege, signing, or distribution scope without returning to the approved record;
- treat a non-mutating build/test success as Gate B authority or a support claim.

`docs/design-review.md` and `docs/review-resolution.md` are immutable historical records unless a human owner explicitly asks to amend the record. If a pre-approval request crosses this boundary, stop and request explicit authorization.

### Recorded limited exception — freeze-evidence scaffold only

The human owner subsequently authorized one narrow, non-product exception for
the DD-FR-002 freeze-evidence lane: deterministic full-byte fixture/hash/index
generation, a dependency-free evidence-format validator, and D07/D08/G1A
evidence templates/procedures. This exception is historical and scoped to the
versioned `fixtures/dd-fr-002-wire-v1-candidate-*` candidates and `tools/`
evidence artifacts; it
does not authorize application/runtime source, Tauri/watchdog/worker
integration, a runtime serializer or WAL, fixture execution against product
code, a fault harness, Windows probing beyond separately authorized read-only
evidence, Phase 2A, or any display mutation. The normal gate and all other
prohibitions remain in force.

## Fixed technology and product boundaries

- The product is a Windows-only desktop application targeting Windows 10 and Windows 11. Exact editions, builds, CPU architectures, support status, GPU/driver/display cells, and release qualification remain explicit decision gates.
- The desktop foundation is Tauri 2. The UI stack is React, TypeScript, and Vite; native and Windows-specific behavior is Rust.
- Microsoft `windows` crate is the first binding candidate for documented Windows APIs. Its exact version and feature set are not selected until the approved spike.
- The UI runs in WebView2. Do not add alternate operating-system backends or operating-system switching abstractions.
- Do not add Electron-specific application code or an Electron compatibility layer.
- Do not use PowerShell as the display-control strategy without a new design review. Do not add a Node.js native add-on.
- JSON is the default format for non-secret preferences and versioned structured recovery records, subject to the durability and validation rules below.
- NSIS `setup.exe` is the MVP installer. MSI, auto-update, repair, upgrade migration, signing, and public distribution are post-MVP backlog unless Gate C explicitly includes them.
- The initial product uses one application window and is not resident. A short-lived independent recovery process may exist only while a display transaction or startup recovery is active.

## Non-negotiable safety constraints

- Automatic restoration after an unsafe, invisible, or unverifiable display mode is the highest-priority property.
- React is presentation and draft state only. It must never own the rollback deadline, transaction truth, recovery record, Windows handle, or final keep/revert decision.
- Tauri Rust core alone is not a sufficient watchdog. A packaged independent Rust watchdog process must own the live transaction lock, durable epoch, recovery journal, deadlines, parent-loss detection, and recovery decisions.
- A process that performs a potentially blocking Win32 display call must not also be the only watchdog. The watchdog delegates each inspect, preflight, apply, readback, and restore operation to a one-shot Rust worker process and waits for proven process exit before starting a conflicting operation.
- The watchdog may be built from the same Rust package or executable image as a worker role, but watchdog, Tauri core, and an in-flight worker must be separate processes and failure domains.
- If recovery data cannot be durably written and verified, the watchdog cannot start, the ownership lock cannot be acquired, the old worker cannot be proven quiescent, or the candidate cannot be mapped exactly, Windows must not be changed.
- Persist a crash-consistent, versioned dual-slot write-ahead recovery record before any operation receives its one-use `GO`. A checksum without durable ordering and crash-window decisions is insufficient.
- Before a per-user WAL may enter `PREPARED`, publish and verify the protected `MachineActorRecordV1` wire state `ACTIVE_INTENT` while holding the machine-wide gate. Advance it through the canonical `ACTIVE_*` states only as the linked owner WAL and actors become ready. Clear it to `TERMINAL_CLEAN` only after the per-user journal is durably terminal, every worker is proven quiescent, and no live transaction actor remains other than the finalizer performing this last write. `ACTIVE`/`PREPARING`/`CRITICAL_UNKNOWN` are UI or diagnostic projections, never wire values. A missing, unreadable, corrupt, stale, or contradictory machine record blocks maintenance and mutation; it never authorizes cross-user restoration.
- Fence live and recovery actors with a machine-wide maintenance/mutation gate, a trusted-target-derived per-display mutation lock, a per-user/logon-session recovery lock, durable epoch, lease version, journal generation, owner SID/logon identity, actor nonces, operation sequence, and process identity consisting of PID plus process creation time, image identity, role, and nonce. A stale, cross-user, cross-logon, cross-boot, or wrong-display actor must never acknowledge presentation, keep, finalize, reapply, restore, clean up, or maintain a later session.
- The initial release admits mutation only when exactly one supported local interactive console logon is active and no RDP, remote, or second interactive logon is present. Before `KEEP_AUTHORIZED`, a console-session change or Fast User Switching is an immediate revert trigger. After authorization it cannot reverse the decision; it blocks frontend authority while the watchdog commits or a fenced replacement derives Keep/Revert/failed-closed from the decision journal. Another user may inspect read-only state but may not mutate or restore the owner's journal.
- The watchdog must use the Win32 `GetTickCount64` clock for all same-boot live acceptance, presentation, keep/revert arbitration, and safety deadlines because the chosen contract counts sleep and hibernation. Wall clock may be recorded for diagnostics and may help detect stale or cross-boot records, but it never extends, reconstructs, or substitutes for a live deadline. Before `KEEP_AUTHORIZED`, a boot identity mismatch, unprovable boot identity, resume after the entry deadline, or expired maximum session lifetime rejects Keep and selects recovery. After authorization, those signals cannot reverse the in-memory decision; actor loss is resolved from the durable journal.
- `viewRevision` is a core-issued CSPRNG view-instance token, not a React counter. A versioned `get_display_change_status` request with mode `BOOT_HANDSHAKE` is the accepted path for initial React root mount, root remount, explicit frontend-boot restart, and renderer recovery. It invalidates the prior binding before issuing a new token. `ORDINARY_RESYNC` and `PRESENTATION_RESYNC` never rotate or transfer authority. A presentation acknowledgement is accepted only for the exact current view, controller, session, lease, generation, stage token, deadline, and observed payload.
- A presentation acknowledgement is a dedicated typed Tauri command, not an event and not an overload of Confirm. Both bounded presentation stages must be acknowledged before confirmation is active; missing, stale, duplicate-with-different-payload, or late acknowledgement selects recovery.
- The confirmation deadline is the deadline for the authoritative watchdog to enter the one-way in-memory state `KEEP_AUTHORIZED` while holding the decision lock. Before that transition it must verify the exact actor/session identity, `AWAITING_CONFIRMATION`, absence of a winning Revert-class trigger, and `GetTickCount64 <= confirmationDeadlineTickMs`. Once entered, ordinary manual Revert, timeout, EOF, presentation failure, and session change cannot win that live arbitration; storage I/O is not part of the 15-second acceptance window.
- `KEEP_AUTHORIZED` is internal acceptance only and is never durable startup authority or a success response to React. Confirm is committed only after `DecisionJournalV1` publishes and re-reads a valid newer `KEPT_SESSION` slot. If the actor is lost before a valid terminal Keep survives, recovery selects Revert; if a valid terminal Keep survives, recovery selects Keep. An uncertain write/flush/readback outcome is resolved by closing and re-reading both fixed slots, never by immediately overwriting the possible Keep with Revert.
- Before any display mutation, run the architecture-defined `ProvisionCurrentDecisionBaselineV1` under the existing machine→display→user→journal-writer lock order. The current decision chain must begin with one durable, re-read `REVERT_REQUIRED` root at `generation=1, previousGeneration=0`; generation is session-local, and slots with a different session, boot, display, owner, logon, or decision-chain lease must never be connected to it. Keep must target the other slot and retain the root through publication readback. Normal old terminal evidence may be reclaimed one slot at a time only after terminal-clean and actor-absence proof plus a durable reclaim intent; critical, blocked, unknown, corrupt, or unsupported evidence is never auto-reclaimed. Active or unresolved files are never truncated, deleted, or recreated.
- Tauri core must monitor watchdog process exit and heartbeat. A standalone watchdog loss is a live-takeover case: a replacement watchdog may act only after acquiring the machine/display/user locks, proving the old watchdog exited and any worker is quiescent, incrementing `leaseVersion`, and durably invalidating all old actors. Loss of both Tauri core and watchdog remains outside the 15-second guarantee.
- Treat monitor identifiers, mode tokens, and snapshot revisions as opaque and session-scoped. Trusted Rust code re-enumerates and re-resolves them immediately before applying.
- Do not apply a GDI candidate when its relation to CCD readback is ambiguous. Candidate identity, UI label, apply tuple, and expected readback are separate concepts. `canApply` requires exactly one complete expected observation from an approved read-only rule or exact support-fingerprint qualification evidence.
- Apply a mode temporarily first. The initial release must not write `CDS_UPDATEREGISTRY`, `SDC_SAVE_TO_DATABASE`, or equivalent profile persistence. Session-only keep remains subject to human approval.
- Do not use undocumented DPI-scaling setters or direct registry manipulation. Scaling mutation is outside the initial release.
- Do not claim DLDSR, multi-monitor mutation, HDR preservation, virtual-display behavior, or scaling mutation support until the relevant Windows hardware matrix has passed.
- On stale topology, unsupported mode, failed validation, concurrent transaction, remote/non-console session, ambiguous target, unknown recovery schema, or recovery actor conflict, fail closed without changing Windows.
- Never present rollback failure, degraded restoration, persisted-state drift, or recovery blocked by an in-flight call as an ordinary apply error. Preserve recovery evidence and show the most severe state.
- Next-launch recovery is not a 15-second guarantee. Full watchdog loss, all-process termination, OS crash, reboot, and power loss require an explicit product guarantee decision and, if covered, a separately reviewed always-available recovery architecture. Every admitted support cell still needs a pre-approved blind reboot/sign-out/physical path back to its captured persisted baseline.

## Tauri and Rust trust boundary

- React may call only named, operation-specific Tauri commands through a typed wrapper. Do not expose a generic invoke wrapper to feature code.
- Register only the commands required by the approved design. Capabilities and Permissions must bind them to the single local main WebViewWindow; do not enable remote origins.
- Do not grant the frontend shell, process, filesystem, HTTP, opener, updater, or arbitrary window privileges. The frontend must not start the watchdog or choose any executable/file path.
- Treat all command arguments as untrusted. Rust validates the invoking window/origin where supported, exact object shape, integer range, token syntax, session identity, state, revision, current enumeration membership, and topology.
- Tauri events are presentation hints only. A missed event is repaired through an authoritative status command; an event never confirms or restores a transaction.
- Do not load external pages, remote scripts, CDN assets, or arbitrary navigation. Define and verify a restrictive CSP without `unsafe-eval`; add only sources demonstrably required by the selected Tauri/WebView2 build.
- The watchdog executable path is fixed by the packaged application. Start it without a command shell, verify package location and signed image identity, pass only bounded structured data and inherited private handles, and never accept a path or command line from React.
- Do not assume Tauri sidecar packaging makes the watchdog independent of the parent. Parent-death survival, Windows Job behavior, inherited handles, installer inclusion, signature verification, and Task Manager termination are mandatory Phase 2 evidence.
- Limit Rust `unsafe` to small Windows FFI boundary modules. Each unsafe block needs a stated invariant; expose safe domain types and checked buffers above the boundary.

## Research and decision discipline

- Prefer Microsoft, Tauri, Rust, WebView2, and crate-author primary documentation.
- Label material statements as confirmed fact, design decision, inference, or verification item.
- Do not infer a Windows API, Tauri permission identifier, privilege requirement, driver behavior, or crate capability from a name or community sample.
- Keep unresolved product, SLA, support, installer, signing, and accessibility choices in `docs/risks-and-open-questions.md`.
- Windows-specific claims must be confirmed on every exact Windows 10/11 release cell admitted by the approved support matrix, using real GPUs and physical displays. An untested cell is not supported.
- Use only the three gates in `docs/implementation-plan.md`: Gate A authorizes the non-mutating Stage 1 product/safety-core implementation, Gate B authorizes one exact controlled-mutation cell and subsequent MVP packaging work, and Gate C authorizes release. Do not create separate approval gates for UI, read-only integration, watchdog prototype, G1A/G2A, individual fixtures, or NSIS implementation. Display mutation always remains behind Gate B; public release and untested support claims remain behind Gate C.
- Do not convert a historical Electron review finding into “closed for Tauri” by assumption. `docs/tauri-migration.md` records which controls were carried forward and which require re-review.

## After approvals

Implementation must follow the approved stages in `docs/implementation-plan.md`. Recovery foundations and watchdog tests precede product mutation. Gate A does not authorize display mutation; Gate B does not authorize public release, signing, elevation, or a broader support matrix; Gate C applies only to the qualified package and cells named in its record.
