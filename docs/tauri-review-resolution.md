# DisplayDeck Tauri設計レビュー対応表

最終更新: 2026-08-05  
対象レビュー: `docs/tauri-design-review.md`、DD-RR-001〜009/Q01再レビュー指摘、`docs/tauri-design-rerereview.md`のDD-RRR-001〜003、`docs/tauri-design-final-review.md`のDD-FR-001〜002  
状態: TDR/DD-RR/DD-RRR対応に加え、DD-FR-001のbaseline/session-rollover design gapを`ProvisionCurrentDecisionBaselineV1`で現行文書へ反映し、DD-FR-002をpre-Phase 2A freeze gateに記録した。人間による短い再確認、Phase開始記録の入力、実装、Phase 1A/2A/1B/2B、Windows/file API実行は未実施・未承認。

## 1. Statusの意味

| Status | 意味 |
| --- | --- |
| RESOLVED | レビューが指摘した未定義/矛盾を、state、priority、timeout、fencing、fail-closed条件、test oracleまで設計文書へ具体化した。実装/実機evidence済みという意味ではない |
| RESOLVED_WITH_PRE_PHASE_2A_FREEZE_CONDITION | Phase 1A前に解くdesign gapは残っていないが、Phase 2A code/file/fixture/fault harness作成前にexact wire/serialization/test-oracle artifactをfreeze/reviewすることが必須 |
| PARTIALLY_RESOLVED | 設計の一部だけ具体化し、実装者判断が残る |
| DEFERRED | 当該scopeを後続版へ明示移動した |
| NOT_RESOLVED | 指摘された設計gapが残る |

本対応ではTDR-001〜TDR-006、DD-RR-001〜009/Q01、DD-RRR-001〜003、DD-FR-001を設計上`RESOLVED`、DD-FR-002を`RESOLVED_WITH_PRE_PHASE_2A_FREEZE_CONDITION`とする。これは短い再確認へ提出できることを示すだけで、review pass、Phase authorization、実装/evidence完了ではない。Phase 1Aはexact machine/roles/evidence/approval recordが未決定、Phase 2AはPhase 1A/G1A closure、DD-FR-002 freeze、専用承認が未完了、Phase 1BはPhase 2A/G2A closure、exact lab準備、実変更の人間承認が未完了であり、全て現時点で`NOT EXECUTABLE`である。

## 2. Summary

| ID | Severity | Status | Phase 1A blocker | Phase 1B blocker |
| --- | --- | --- | --- | --- |
| TDR-001 | High | RESOLVED | No。read-only scopeにはpresentationなし | Design gapは解消。再レビュー/Phase 1B共通gate完了まではYes |
| TDR-002 | High | RESOLVED | No | Design gapは解消。race evidence/Phase 1B gate完了まではYes |
| TDR-003 | High | RESOLVED | No | Design gapは解消。Phase 2A takeover evidence完了まではYes。Phase 2BはPhase 1B後 |
| TDR-004 | High | RESOLVED | No。clock/session read-only evidence項目は追加 | Design gapは解消。clock/sleep/boot evidence完了まではYes |
| TDR-005 | High | RESOLVED | No。cross-session観測はPhase 1A必須 | Design gapは解消。lock/DACL/session fencing evidence完了まではYes |
| TDR-006 | High | RESOLVED | No | Design gapは解消。Phase 2A/8 maintenance evidence完了まではYes |

## 3. TDR-001 — 2段階presentation ACKを送る公開経路がない

- **指摘ID**: TDR-001
- **重要度**: High
- **元の問題**: 公開5 commandにfrontend→RustのStage 1/2 ACK経路がなく、eventはRust→Reactのhintだけだった。実装者がgeneric eventまたはConfirmを流用するとsecurity surfaceとstate semanticsが崩れる。
- **採用した解決方針**: 6番目のoperation-specific Tauri command `ack_display_change_presentation`を追加した。payloadを`{sessionId,generation,leaseVersion,presentationToken,stage,viewRevision,ackNonce}`へ限定し、main local current viewだけから受ける。Stage 1/2ごとtoken/generation/nonceをrotateする。
- **更新した文書**: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/ui-design.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/tauri-migration.md`、`docs/tauri-design-review-checklist.md`
- **更新したセクション**: requirements FR-201/203/208/308、architecture 3/4.1/5/6/10/10.2/18、ui-design 2.2/4/8/9、security 3.1/3.2/5.3/13、testing 3.3/4/7.2/traceability、implementation-plan Phase 0/2/3/5/6
- **新しい状態遷移**: `APPLIED_VERIFIED(t0) -> PRESENTING_STAGE1 -> PRESENTING_STAGE2 -> AWAITING_CONFIRMATION`。両ACKが揃うまでConfirm不可。missing/invalid/late/crashは`REVERT_DECIDED -> Restoring`。
- **新しい安全条件**: ACK deadlineは`min(t0+2,000ms, confirmationDeadline-12,000ms)`。ACKはdeadlineを開始/延長せず物理可視性も証明しない。同一stage/token/nonce/payloadのduplicateだけ同じresultを返す。old view、wrong stage、payload mismatch、stale generation/lease/tokenは拒否する。
- **残余リスク**: DOM/focus ACKはmonitorの物理可視性を証明しない。WebView2 focus/topmost/render timingはpackaged physical E2Eが必要。
- **技術スパイクで確認する項目**: Stage 1/2 render/focus、IPC latency、reload/crash/hidden、duplicate/late/stale ACK、event loss/status repair、2秒/残12秒margin。
- **対応状況**: RESOLVED
- **再レビューが必要か**: Yes。6 commandのCapability/Permission/DTO/stateをTauri再レビューする。
- **Phase 1Aを妨げるか**: No。Phase 1AはReact/Tauri UIを作らずread-onlyである。
- **Phase 1Bを妨げるか**: 設計gapとしては解消。再レビューとPhase 1B共通gateが完了するまでは開始を妨げる。

## 4. TDR-002 — Confirming中のRevert可否が矛盾

- **指摘ID**: TDR-002
- **重要度**: High
- **元の問題**: command表はApplying以降のRevertを許す一方、application stateのConfirmingはstatusのみだった。Tauri core local stateがwatchdogより先にRevertを拒否し、危険なKeepを事実上勝たせ得た。
- **採用した解決方針**: Confirmingでも`restore_display_change`をwatchdogへforwardする。final readback後、decision lock内でdeadlineまでに`KEEP_AUTHORIZED`へ入る前はmanual Revert/timeout/EOF/presentation/session failureと競合し、authorization後はordinary Revertを開始しない。React successは`DecisionJournalV1`のvalid `KEPT_SESSION` readback後だけである。
- **更新した文書**: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/ui-design.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/tauri-migration.md`、`docs/tauri-design-review-checklist.md`
- **更新したセクション**: requirements FR-309/404/405、architecture 5.1/6/10.1/11.1、ui-design 4/7/8、security 5.3、testing 3/5.2/7.2/7.5、implementation-plan Phase 2A/2B/6
- **新しい状態遷移**: `AWAITING_CONFIRMATION -> Confirming -> KEEP_AUTHORIZED(in-memory) -> KEPT_SESSION(durable)`、またはauthorization前のRevert classにより`AWAITING/Confirming -> REVERT_DECIDED -> Restoring`。authorization後はjournal outcomeへ従い、`KEPT_SESSION`後のRevertはterminal reject。
- **新しい安全条件**: watchdogだけがdecision pathをserializeする。coreの`Confirming` projectionはRevertを拒否する根拠にならない。同一sessionに`KEPT_SESSION`とRevert terminalの双方を作らない。duplicate Confirm/Revertはconsumed nonce/terminal resultを返し、workerを追加しない。
- **残余リスク**: authorization後のdisk/AV latencyやalive-but-hung writerはUIを長時間`ConfirmCommitInProgress`にし得る。writer fencing/outcome readback値は実測が必要。
- **技術スパイクで確認する項目**: Confirm vs manual Revert/timeout/EOF、`KEEP_AUTHORIZED`直前直後、slot write/flush/readback crash、duplicate/response loss、deadline just-before/at/after、writer hang、single terminal invariant。
- **対応状況**: RESOLVED
- **再レビューが必要か**: Yes。commit semanticsとUI copyをSafety/UX reviewerが確認する。
- **Phase 1Aを妨げるか**: No。
- **Phase 1Bを妨げるか**: 設計gapとしては解消。Phase 2A fake-clock/fault evidenceとPhase 1B gate完了までは開始を妨げる。

## 5. TDR-003 — watchdog単独crash時のlive takeover未定義

- **指摘ID**: TDR-003
- **重要度**: High
- **元の問題**: Tauri coreが生存しwatchdogだけがcrashした場合、replacement actor、deadline継承、mutex abandonment、old worker quiescence、old watchdog fencingがなかった。
- **採用した解決方針**: 選択肢を比較し、initial releaseはTauri core monitorがreplacement watchdogを起動する方式を採用した。core direct restore、常時複数watchdog、Windows serviceは不採用。heartbeat missはsuspect、process handle+full identityでexit確定とし、exit未証明でtakeoverしない。250ms/4 miss等はPhase 2A候補値へ格下げし、承認済み`HeartbeatPolicyV1`を要求する。replacementはmachine/display/user locks、old exit/worker quiescence、journal identityを検証し、leaseVersionをdurableに増やす。
- **更新した文書**: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/ui-design.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`、`docs/tauri-migration.md`、`docs/tauri-design-review-checklist.md`
- **更新したセクション**: requirements FR-406/409/AC-021、architecture 4.5/4.6/16/17/18、ui-design 9、security 8、testing 7.3/7.4/10/13、implementation-plan Phase 2A/2B/6、risks R-T13/Q07
- **新しい状態遷移**: pre-mutation stateはreplacementが`ABORTED_NO_MUTATION`、APPLY/PRESENTING/AWAITINGはconfirmationを再開せず`REVERT_DECIDED`、Confirmingは`DecisionJournalV1` readbackでvalid Keep/No Keep/unreadableを判定、restore intentはquiescence後resume、terminalはno-op。
- **新しい安全条件**: old process exitとworker quiescenceを証明するまでnew GOなし。new watchdogInstanceId/leaseVersion durable後にだけreplacementがactする。old leaseのframe/decision/resultは拒否。takeover failureは`RECOVERY_TAKEOVER_BLOCKED`で並行callなし。
- **残余リスク**: core+watchdog simultaneous loss、termination/query denied、replacement launch failure、OS suspend/crashはlive guarantee外。常時serviceを採らないためblind/next-launch recoveryが必要。
- **技術スパイクで確認する項目**: heartbeat timing、hang/exit detection、exact process termination、Job/handle、replacement launch、all durable states、old actor revival、worker hang/query denied、package/AV behavior。
- **対応状況**: RESOLVED
- **再レビューが必要か**: Yes。Phase 2A evidence前提のarchitecture/security reviewが必要。
- **Phase 1Aを妨げるか**: No。
- **Phase 1Bを妨げるか**: 設計gapとしては解消。Phase 2A/G2A takeover evidenceとhuman mutation approval完了までは開始を妨げる。Phase 2B evidenceはPhase 1Bの前提ではない。

## 6. TDR-004 — clock、sleep、boot、deadline contract不足

- **指摘ID**: TDR-004
- **重要度**: High
- **元の問題**: `monotonic`だけでexact API、sleep inclusion、boot identity、watchdog restart比較、UI projectionがなく、sleep/clock changeで15秒が延長し得た。
- **採用した解決方針**: live deadlineはwatchdogが直接読むWin32 `GetTickCount64`へ固定し、sleep/hibernateを経過へ含める。wall clockはdiagnosticとboot/stale矛盾検出の補助だけでlive acceptanceへ使わない。bootIdはdocumented `LastBootUpTime`、exact OS build、`GetTickCount64`/UTC差のcross-checkから作り、証明不能ならmutation不可/restoreとする。UIへはadvisory bounded `remainingMs`だけを返し、Keep oracleにしない。
- **更新した文書**: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/ui-design.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`、`docs/tauri-design-review-checklist.md`
- **更新したセクション**: requirements FR-205/307/308/311、architecture 10.1/16、research 13、ui-design 8.1/9、security 5.3/8、testing 7.2、implementation-plan Phase 1A/2A/2B、risks R-T14/Q02
- **新しい状態遷移**: created+30秒を超えるpre-applyは`ABORTED_NO_MUTATION`、created+60秒またはt0+15秒超過でmutation済みかつ未`KEEP_AUTHORIZED`なら`REVERT_DECIDED`。authorization後はdeadlineでdecisionを逆転せずjournal outcomeへ従う。resume/boot mismatchではconfirmation再開なし。
- **新しい安全条件**: t0はapply API returnやACKではなくpost-apply exact readback durable tick。presentationはt0+2秒、confirmationはt0+15秒、maximumはcreated+60秒で非延長。takeoverは同じabsolute tickを継承。deadline直前requestはcommitを保証しない。
- **残余リスク**: OS sleep中はprocessをscheduleできずrollback call-startを保証できない。boot evidence query/cross-checkのdriver/enterprise environment差、wall-clock changeによるsafe false-negativeがあり得る。
- **技術スパイクで確認する項目**: GetTickCount64 sleep/hibernate、wall-clock forward/back、LastBootUpTime/Fast Startup/reboot、WMI timeout/failure、takeover、resume-to-call-start、UI resync。
- **対応状況**: RESOLVED
- **再レビューが必要か**: Yes。内部contractは固定済みだが外部SLA/copyはQ02のhuman decision。
- **Phase 1Aを妨げるか**: No。read-only clock/session observationをPhase 1Aへ追加する。
- **Phase 1Bを妨げるか**: 設計gapとしては解消。Phase 2A clock evidenceとPhase 1B gate完了までは開始を妨げる。

## 7. TDR-005 — cross-session actor fencing不足

- **指摘ID**: TDR-005
- **重要度**: High
- **元の問題**: per-user/logon lockだけではFast User Switching等の別session actorが同じphysical display stackへ競合できた。old controller/watchdog/view、別user/displayを全operationでfenceするtupleも不足していた。
- **採用した解決方針**: initial mutationを唯一のlocal active-console interactive logon、RDP/second logonなし、single active pathへ限定した。machine-wide gate、trusted display digestのper-display lock、owner SID/logon recovery lockを順に取得する。bootId、owner SID/logon/session、sessionId、displayId、controller/watchdog instance、epoch、leaseVersion、generation、presentation token、command nonceをoperation別に検証する。
- **更新した文書**: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`、`docs/windows-display-research.md`、`docs/tauri-migration.md`、`docs/tauri-design-review-checklist.md`
- **更新したセクション**: requirements FR-204〜208/501/502/AC-023/024、architecture 3/4.6/18/19、security 5.3/8.2/8.3、testing 7.4/8/9/14、implementation-plan Phase 1A/2A/2B、risks R-T15/7.1、research 9/13
- **新しい状態遷移**: session/console changeは`KEEP_AUTHORIZED`前のactive stateから`REVERT_DECIDED`、authorization後はold frontend authorityを失効してDecisionJournal commit/recoveryを継続する。old/foreign operationはstate transitionなしでreject。takeoverだけがleaseVersionを増やしてold actorをfenceする。
- **新しい安全条件**: 全operationのrequired tupleを表で固定した。frontendはSID/boot/display native identityを指定できない。別userはread-onlyのみ、他owner journalをrestore/cleanup不可。abandoned lockはrecovery triggerで、fresh mutation permissionではない。
- **残余リスク**: standard userによるGlobal namespace/DACL、session notification delivery、same/other-user DoSは実機未証明。serviceを使わないためowner不在recoveryはできない。
- **技術スパイクで確認する項目**: Phase 1Aのread-only API/session visibility。Global/Local mutex create/open、DACL、FUS/RDP/second user、old view/process/token、wrong display、PID reuse、session notification lossはPhase 2A。
- **対応状況**: RESOLVED
- **再レビューが必要か**: Yes。security/Windows session modelを再レビューする。
- **Phase 1Aを妨げるか**: No。ただしcross-session read-only観測はPhase 1Aの必須項目。
- **Phase 1Bを妨げるか**: 設計gapとしては解消。Phase 2A/G2A fencing evidenceとexact single-user lab conditionが揃うまでは開始を妨げる。Phase 2BはPhase 1B後。

## 8. TDR-006 — per-machine maintenanceとper-user recoveryの調停不足

- **指摘ID**: TDR-006
- **重要度**: High
- **元の問題**: per-machine installerが、別user LocalAppDataのpending journal/watchdogを検出せずshared recovery binaryをupgrade/uninstallできた。
- **採用した解決方針**: runtime/watchdogとinstallerが共通のmachine-wide maintenance/mutation gateをexclusiveに保持する。canonical machine wire `ACTIVE_INTENT`をowner WAL `PREPARED`より先にdurable化し、owner WAL terminalとworker/transaction actor quiescence後にだけ`TERMINALIZING -> TERMINAL_CLEAN`へ進める。maintenanceはmachine recordとreferenced owner WALをread-only照合し、non-clean/unknown/unreadable/inconsistent stateでは拒否する。別admin/installerは他owner journalをrestoreしない。
- **更新した文書**: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`、`docs/tauri-migration.md`、`docs/tauri-design-review-checklist.md`
- **更新したセクション**: requirements FR-505/AC-024、architecture 15/19、security 7/8.3/12/13、testing 14、implementation-plan Phase 2A/8、risks R-D03/R-T16/Q04/7.1
- **新しい状態遷移**: runtimeはmachine gate取得後にactor recordをactive化してtransactionへ進み、terminal durable後だけclean化する。installerはgate取得→maintenance intent→全record terminal-cleanのときだけbinary operationへ進む。abandoned/stale/unknownはmaintenance reject。
- **新しい安全条件**: lock順はmachine gate→per-display→per-user recovery→file handles。逆順/lock中UI wait禁止。machine recordはdiscovery/blocking専用でrestore authorityなし。in-use/old-version recovery binaryをmaintenance完了前に削除せず、version coexistence/rollbackをPhase 8で検証する。
- **残余リスク**: ProgramData ACL/record durability、cross-user process query、AV/installer behavior、malicious local-user DoSは実機未証明。owner不在pending recordはmaintenanceを長期blockし得る。
- **技術スパイクで確認する項目**: Phase 2A Global gate/actor record/DACL/abandoned/boot/FUS、Phase 8 User A transaction中のUser B/admin update/repair/uninstall、version coexistence、NSIS/MSI、owner-unavailable拒否。
- **対応状況**: RESOLVED
- **再レビューが必要か**: Yes。installer/security/recovery joint reviewが必要。
- **Phase 1Aを妨げるか**: No。
- **Phase 1Bを妨げるか**: Design gapは解消。Phase 2A maintenance coordinationとPhase 1B共通gate完了までは開始を妨げる。Phase 8/public releaseにはpackaged evidenceが別途必須。

## 9. Resolution後のgate

### Tauri再レビュー

6件は再レビューへ提出可能である。reviewerは少なくとも次を一つのcontractとして確認する。

- 6 commandと2-stage ACK
- `KEPT_SESSION` commit pointとConfirming中Revert
- `GetTickCount64`/bootId/sleep/max lifetime
- replacement watchdog/leaseVersion/old worker quiescence
- owner/logon/boot/display/actor/nonce fencing
- machine gate/per-display/per-user lock/actor record/maintenance拒否

### Phase 1A

範囲はread-only allowlistだけである。`CDS_TEST`、`SDC_VALIDATE`、`ChangeDisplaySettingsExW`、`SetDisplayConfig`、named mutex create/open、DACL/SDDL、machine/WAL、process/watchdog、apply、restore、registry/profile writeは禁止する。設計上のHighはPhase 1Aをblockしないが、`docs/implementation-plan.md` 1.1のexact call allowlist、redaction/evidence、Target Machine、Operator、Evidence Owner、Reviewer、evidence location、approver/resultが未決定なので現時点は開始不可である。

### Phase 1B

TDR/DD-RR resolutionだけでは開始できない。Tauri再レビュー、Phase 1A/G1A closure、Phase 2A/G2A coordination/storage/process evidence、exact machine/operator/evidence owner/reviewer、GPU/monitor/connection、emergency/blind recovery、out-of-band別操作経路、exact mutation runの人間承認が全て必要である。

## 10. DD-RR summary

| ID | Severity | Design status | Phase 1A blocker | Phase 1B blocker |
| --- | --- | --- | --- | --- |
| DD-RR-001 | High | RESOLVED | No | Design gapは解消。Phase 2A/G2A view-fence evidenceまではYes |
| DD-RR-002 | High | RESOLVED | No | DD-RRRでcontract置換済み。Phase 2A/G2A DecisionJournal evidenceまではYes |
| DD-RR-003 | High | RESOLVED | No | Design gapは解消。Phase 2A/G2A machine/WAL evidenceまではYes |
| DD-RR-004 | Medium | RESOLVED | No | Yes。roadmap上Phase 2A/G2A closureが明示前提 |
| DD-RR-005 | Medium | RESOLVED | No | Yes。`HeartbeatPolicyV1` evidence/approvalが必要 |
| DD-RR-006 | Medium | RESOLVED | No | Yes。Phase 2A clock/boot/UI-oracle evidenceが必要 |
| DD-RR-007 | Medium | RESOLVED | No | Yes。Phase 2A maintenance-fence evidenceが必要 |
| DD-RR-008 | Low | RESOLVED | No。ただしPhase 1A開始record未完成 | No。named-object workはPhase 2Aへ移管済み |
| DD-RR-009 | Low | RESOLVED | No | No |
| DD-RR-Q01 | Gate | RESOLVED | **Yes**。templateは整備済みだが全欄未決定/未承認 | Yes。先行Phaseも未承認 |

## 11. DD-RR-001 — `viewRevision` lifecycle未定義

- **指摘ID / 重要度**: DD-RR-001 / High
- **関連TDR**: TDR-001（presentation ACK経路）、TDR-005（cross-session/view fencing）
- **元の問題**: `viewRevision`の発行主体、entropy、rotate条件、controller/presentation binding、reload後のStage 1 ACK継承、stale/duplicate errorが未定義で、old rendererまたはXSSが新しい確認UIをACKできた。
- **採用した解決方針**: core-issued 128-bit以上CSPRNG opaque view-instance tokenとし、React counter/storageを禁止した。core memoryのcurrent bindingとsanitized projectionだけに保持し、raw log/WALへ保存しない。
- **exact state/sequence**: initial/root remount/frontend restart/renderer recoveryは`get_display_change_status(StatusRequestV1{mode:BOOT_HANDSHAKE})`受理時に旧bindingを失効し`V0`発行 → Stage 1 exact tuple ACK → 同じviewをStage 2へrebind → 両ACK後`AWAITING_CONFIRMATION`。active presentation中のnew viewはStage 1から再構築し、deadlineを延長しない。focus/minimize/child remountは`ORDINARY_RESYNC`でrotateしない。
- **新しい安全条件**: ACKは`controllerInstanceId,viewRevision,sessionId,leaseVersion,generation,stage,presentationToken,ackNonce,deadline,observedPayloadDigest`のexact一致だけ。同一payload/nonce duplicateは同result、same nonce/different payload/old tupleは`STALE_PRESENTATION_ACK`、old viewは`STALE_VIEW_INSTANCE`。Stage 1 authority transfer/Stage 2-only new-view ACKはpresentation failure→Revert。
- **crash window**: Stage 1前crashはACKなし、Stage 1後crashはold ACKを移送せずnew Stage 1から再構築、Stage 2後/Confirm前crashもauthoritative statusへ同期し、旧view操作を拒否する。Tauri core lossはparent-loss Revert path。
- **残余リスク**: DOM ACKは物理可視性を証明せず、core memory tokenのCSPRNG/zeroization/log redaction、WebView lifecycle event orderingは未実装・未検証。
- **技術スパイク**: Phase 2A protocol model、Phase 3〜6 packaged WebView reload/crash/XSS-old-renderer/focus E2E、token entropy/log audit。
- **更新文書 / section**: `AGENTS.md` safety constraints、requirements FR-207/208/410・AC-025、architecture 5.4/18.1/18.2、UI 4/8、security 5.3/13、testing 3.3/7.2/traceability、checklist 3/10/12/15。
- **Phase 2Aで確認する事項**: protocol modelでview rotate/stale/duplicate/reload/root-remount/controller/generation/token fence。packaged WebView lifecycle/XSS E2EはPhase 3以降の別gate。
- **対応状況 / 再レビュー**: RESOLVED / Yes（Security + Tauri + Safety）。
- **Phase blocker**: Phase 1A No。Phase 1BはPhase 2A/G2A evidenceまでYes。

## 12. DD-RR-002 — `KEPT_SESSION` crash-consistent commit未定義

- **指摘ID / 重要度**: DD-RR-002 / High
- **関連TDR**: TDR-002（Confirm/Revert race）、TDR-004（deadline/clock）
- **元の問題**: terminal frameのwrite/flush/reopenのどこがcommit pointか、deadline/manual Revertとのlinearization、partial write/old-new coexist/duplicate Confirmが曖昧で、Keep/Revert二重成立またはdeadline後Keepが可能だった。
- **採用した解決方針**: DD-RRR-001により旧C方式を置き換えた。final readback後、decision lock内で期限までにone-way in-memory `KEEP_AUTHORIZED`へ入る点をConfirm acceptanceとし、1 file/fixed A/B slotの`DecisionJournalV1`でnew valid `KEPT_SESSION`をflush/readbackした点をcommitとする。
- **exact state/sequence**: `AWAITING_CONFIRMATION` → final readback current=R/persisted=P0 + worker exit → decision lockでexact identity/state/Revert未勝利/fresh tick検証 → `tick <= deadline`なら`KEEP_AUTHORIZED` → current identityのroot baselineと反対側のfixed slotへfull write/checksum/FlushFileBuffers/close/reopen → current chainのA/Bでexact new generationが最大validなら`KEPT_SESSION`。foreign generationは比較しない。authorization前にRevert/lateなら`REVERT_DECIDED`。
- **新しい安全条件**: acceptedはReact successでもstartup Keep authorityでもない。authorization後はdeadlineを再検査せずordinary Revertを開始しない。write/flush/readback outcome unknownはpossible Keepを上書きせずjournal再読込でKeep/Revert/FAILED_CLOSEDを決める。Move/Replaceをdeadline CASにしない。
- **crash window**: authorization前、authorization後slot前、partial/torn/short write、full write/flush前、flush後/readback前、readback後/response前、deadline直前entry、entry後I/O delay、same-generation conflictをarchitecture 7.4表に固定した。
- **残余リスク**: fixed-slot/write/flush/readbackのfilesystem/storage/filter/power-loss挙動とalive-hung writer fencingは未証明。一意なresultが得られないcellはmutation No-Goとなる。
- **技術スパイク**: Phase 2A fault/power-loss harnessで全crash point、fixed-offset write/flush/readback、generation chain、deadline entry、duplicate/race、AV/EDR/filter、writer hangを実証。Phase 2Bでqualified transitionと統合。
- **更新文書 / section**: `AGENTS.md` safety constraints、requirements FR-309/404・AC-020/026、architecture 6.2/7.3/7.4/11.1、security 5.3/8.3/13、testing 6/7.2/traceability、implementation Phase 2A、risks R-T18、checklist 9/11/15。
- **Phase 2Aで確認する事項**: every required crash checkpoint、fixed slots/short-torn write/flush/readback/AV/filter/power loss、deadline entry、generation conflict、duplicate/race、outcome unknown、writer fencing No-Go。
- **対応状況 / 再レビュー**: RESOLVED / Yes（Storage + Windows + Safety）。
- **Phase blocker**: Phase 1A No。Phase 1BはPhase 2A/G2A DecisionJournal/writer-fencing evidence/approvalまでYes。

## 13. DD-RR-003 — machine actor record / per-user WAL durable order未定義

- **指摘ID / 重要度**: DD-RR-003 / High
- **関連TDR**: TDR-006（per-machine maintenance）、TDR-005（cross-session fencing）
- **元の問題**: machine recordとowner WALの作成/terminal化/cleanup順、片側だけdurableなcrash、DACL、maintenance read failure、actor absenceが未定義で、installerがrecovery binaryを誤って更新/削除できた。
- **採用した解決方針**: DD-RRR-003によりarchitecture 19.2を唯一の`MachineActorRecordV1` wire正本とし、canonical fields、13 state enum、state別field/transition/maintenance/recovery/actor/WAL表を固定した。`ACTIVE/PREPARING/CRITICAL_UNKNOWN/CLEAN/PENDING`はprojectionだけに限定した。
- **exact state/sequence**: machine gate→existing record inspection→display lock→user lock→identity allocation→machine ACTIVE_INTENT durable/reopen→owner WAL PREPARED→WATCHDOG_READY→machine exact-generation link→APPLY_GO_ARMED durable/link→one-use GO→worker operations。終了はowner WAL terminal publish→workers/old actors absent→controller transaction binding invalidation→sole finalizer `FINALIZER_EXITING`→owner WAL terminal durability再確認→machine TERMINAL_CLEAN durable→WAL handle/user/display/machine lock reverse release。
- **新しい安全条件**: machine recordはrestore authorityではない。maintenanceはsame machine gate取得後、recordとreferenced owner WALをread-only exact照合し、unreadable/missing/mismatch/active/critical/unknown/tamper/reparse/actor不明ならfail closed。別user/adminはowner journalをrestore/cleanupしない。installer read権限候補はterminal照合のみでwrite不可、exact SDDLはPhase 2A gate。
- **crash window**: ACTIVE_INTENT partial/flush/reopen、WAL PREPARED partial/ACTIVE_PREPARED link前、WATCHDOG_READY/APPLY_ARMED/MUTATED、WAL terminal、TERMINALIZING、worker exit、TERMINAL_CLEAN publication、lock release、schema migration/rebootのmatrixをarchitecture 19.5へ固定した。片側だけcleanを推測しない。
- **残余リスク**: standard userがprotected machine recordを安全に更新できるDACL、cross-user read、same-user/local-user DoS、AV/installer share、owner不在availabilityは未証明。成立しなければmutation/maintenance No-Go。
- **技術スパイク**: Phase 2A exact SDDL/named namespace/FUS/RDP/crash/power/tamper/read-denied/update begin-complete、Phase 8 packaged NSIS/MSI update/repair/uninstall。
- **更新文書 / section**: `AGENTS.md` safety constraints、requirements FR-411・AC-024/027、architecture 10/19.2〜19.6、security 8.3/12/13、testing 7.5/14/traceability、implementation Phase 2A/8、risks R-T19/7.1、checklist 10/13/15。
- **Phase 2Aで確認する事項**: exact SDDL/Global-Local namespace/FUS/RDP、全durable crash point、record delay/WAL先行/generation/boot/owner mismatch、abandoned lock、AV/read denied、maintenance/update/repair/uninstall begin/complete。Phase 8でpackaged installerを別確認。
- **対応状況 / 再レビュー**: RESOLVED / Yes（Windows Security + Installer + Recovery）。
- **Phase blocker**: Phase 1A No。Phase 1BはPhase 2A/G2A durable-order/DACL evidenceまでYes。

## 14. DD-RR-004〜009 / Q01

### DD-RR-004 — Phase順序

- **指摘ID / 元の重要度 / 関連TDR**: DD-RR-004 / Medium / TDR-002〜006のevidence gate
- **再レビューの問題**: Phase 1Bより前にcoordination/storage/process evidenceがなく、Phase 2Bとの依存も曖昧だった。
- **採用した解決**: roadmapを`P0→P1A→G1A→P2A(no mutation)→G2A→P1B→P2B→P3以降`へ固定し、P2BはP1B完了後とした。
- **更新文書 / section**: `AGENTS.md` research discipline、implementation roadmap/Phase 1B/2A/2B/3 prerequisites、risks 8/9、checklist 15。
- **crash/race contract**: runtime crashはN/A。G1A/G2A未closureまたはapproval ID mismatchを`NOT EXECUTABLE`にするapproval race fence。
- **残余リスク**: 人間approval ID/closureは未記入。
- **Phase 1A / 1B blocker**: P1A No（Q01は別blocker）。P1B Yes（P2A/G2Aまで）。
- **Phase 2Aで確認する事項**: G1A closureをreadbackし、Phase 2A専用record/approvalをfreezeする。
- **対応状況 / 再レビュー**: RESOLVED / Yes。

### DD-RR-005 — heartbeat値とhang分類

- **指摘ID / 元の重要度 / 関連TDR**: DD-RR-005 / Medium / TDR-003
- **再レビューの問題**: 250ms/4 misses/250msを根拠のないproduct値とし、hang/IPC/exit/OS pauseを区別していなかった。
- **採用した解決**: versioned `HeartbeatPolicyV1`候補へ格下げし、miss/suspect/exit/alive-hung/IPC stall/resume/starvation/AV-delay/termination/access failureを別状態にした。
- **更新文書 / section**: requirements FR-409/AC-028、architecture 4.6/17.2、testing 7.3、implementation Phase 2A、risks Q07/R-T13、checklist 8。
- **crash/race contract**: miss→`HEARTBEAT_SUSPECT`でKeep disabled、full identity付きexit証明→takeover candidate、alive/exit不明→`RECOVERY_TAKEOVER_BLOCKED`。exit未証明でreplacementを起動しない。
- **残余リスク**: worst-case latency/false-positive受容値は未決定。
- **Phase 1A / 1B blocker**: P1A No。P1B Yes（policy evidence/approvalまで）。
- **Phase 2Aで確認する事項**: jitter、CPU、sleep/resume、debugger、AV/EDR、disk stall、process handle wait、termination API/access、launch、false-positive count/rate。0 double actor/unproven takeoverと人間承認をproduct昇格条件にする。
- **対応状況 / 再レビュー**: RESOLVED / Yes。

### DD-RR-006 — wall clockとReact `remainingMs`

- **指摘ID / 元の重要度 / 関連TDR**: DD-RR-006 / Medium / TDR-004
- **再レビューの問題**: wall clockのstale用途が矛盾し、React countdownがacceptance oracleになり得た。
- **採用した解決**: wall clockはdiagnostic+boot/stale矛盾検出補助のみ、same-boot elapsed/deadlineはwatchdog `GetTickCount64`のみ、`remainingMs`はadvisory projectionとした。cross-boot tick直接比較を禁止した。
- **更新文書 / section**: `AGENTS.md` clock constraint、requirements FR-307/412/AC-022、architecture 10.1/16、UI 4/8、research 13、testing 3.3/7.2、checklist 11/12。
- **crash/race contract**: boot証明不能/rebootはconfirmation再開なしrecovery。UI crash/reload/resumeはstatus取得までKeep disabled。React正値でもwatchdog fresh tickがexpiredならRevert、React 0もterminal decisionではない。
- **残余リスク**: Fast Startup/WMI/clock jump/enterprise environmentは未実証。
- **Phase 1A / 1B blocker**: P1A No（read-only evidence対象）。P1B Yes（P2A evidenceまで）。
- **Phase 2Aで確認する事項**: same/cross boot、sleep/hibernate、wall jump、WMI failure/cross-check、takeover tick inheritance、UI oracle model。
- **対応状況 / 再レビュー**: RESOLVED / Yes。

### DD-RR-007 — maintenance operation fence

- **指摘ID / 元の重要度 / 関連TDR**: DD-RR-007 / Medium / TDR-006
- **再レビューの問題**: maintenance/update/repair/uninstallのbegin/complete/commit/rollback identity、intent/completion、reject、owner-unavailableが未定義だった。
- **採用した解決**: gate→record/WAL exact read-only照合→kind別intent/nonce→`MAINTENANCE_ACTIVE`→staged operation→signature/recovery-reader verification→completion/tombstone→gate releaseを固定した。
- **更新文書 / section**: requirements FR-411/AC-024/027、architecture 18.2/19.3/19.6、security 8.3/12/13、testing 7.5/14、implementation Phase 2A/8、checklist 13。
- **crash/race contract**: 各boundary crashはintent/active/failed-closedから再検証し、old recovery readerを先に削除しない。unreadable/mismatch/in-use/signature/owner unavailableでbinary operation 0件。
- **残余リスク**: NSIS/MSI rollback、AV/share、cross-user DACL未証明。
- **Phase 1A / 1B blocker**: P1A No。P1B Yes（P2A operation-fence evidenceまで）。
- **Phase 2Aで確認する事項**: all four operation begin/complete、epoch/boot/binary/terminal generation/actor/owner/nonce、intent/completion/reject code、installer crash/read denied/AV。Packaged installerはPhase 8。
- **対応状況 / 再レビュー**: RESOLVED / Yes。

### DD-RR-008 — Phase 1Aにnamed mutex/DACLが混入

- **指摘ID / 元の重要度 / 関連TDR**: DD-RR-008 / Low / TDR-005
- **再レビューの問題**: Phase 1A read-only範囲にGlobal/Local mutex ownership/create/open、abandonment、DACL/writeable cross-session objectが混入していた。
- **採用した解決**: Phase 1Aで全て明示禁止しPhase 2Aへ移管。1AはSID/logon/console/OS/display/read-only JSON/errorだけのexact human-approved rowsとした。
- **更新文書 / section**: implementation 1.1/Phase 1A、windows research 13、risks 8/9、checklist 15。
- **crash/race contract**: runtime N/A。Phase 1A allowlistにnamed-object/write rowがあればauthorization validation failure。
- **残余リスク**: exact API rowsは未承認。
- **Phase 1A / 1B blocker**: P1A design blocker NoだがQ01で開始不可。P1B No（この指摘単独）。
- **Phase 2Aで確認する事項**: Global/Local create/open/ownership/abandonment/SDDL/DACL/machine gate prototype。
- **対応状況 / 再レビュー**: RESOLVED / Yes。

### DD-RR-009 — command数の不整合

- **指摘ID / 元の重要度 / 関連TDR**: DD-RR-009 / Low / TDR-001
- **再レビューの問題**: migration文書に5 handler表記が残っていた。
- **採用した解決**: 6 command（snapshot/begin/presentation ACK/confirm/restore/status）へ統一した。
- **更新文書 / section**: migration section 10、architecture 5.1/5.3、security 3/13、testing 4/traceability、checklist 3。
- **crash/race contract**: runtime crash N/A。manifest/wrapper/Capabilityのexact six-command static inspectionをoracleにした。
- **残余リスク**: exact Tauri Permission IDはPhase 3 gate。
- **Phase 1A / 1B blocker**: P1A/P1B No（この指摘単独）。
- **Phase 2Aで確認する事項**: protocol mockが専用ACKをgeneric event/Confirmで代用しないこと。
- **対応状況 / 再レビュー**: RESOLVED / Yes。

### DD-RR-Q01 — Phase 1A開始record

- **指摘ID / 元の重要度 / 関連TDR**: DD-RR-Q01 / Question (execution gate) / 関連TDRなし
- **再レビューの問題**: Target Machine/OS/KB/GPU/display/connection/current state/roles/date/evidence/approvalに加え、exact call/flag allowlistとredactionを記入できる開始recordが不足していた。
- **採用した解決**: 全fieldを分離した未記入templateと、call row schema/forbidden audit/retention/immutable human approvalを追加し、approved row 0件を明記した。
- **更新文書 / section**: implementation 1.1/Phase 1A、risks 8、checklist 15。
- **crash/race contract**: runtime N/A。1項目でも未決定/未承認、approval readback mismatchなら`NOT EXECUTABLE`。候補API名を承認と読み替えない。
- **残余リスク**: 全値、人名、機材、日付、保存場所、approvalは未入力。本対応では補完していない。
- **Phase 1A / 1B blocker**: **P1A Yes**、したがって後続全Phaseも開始不可。
- **Phase 2Aで確認する事項**: N/A（Phase 1A開始前の人間記入/承認事項）。Phase 2Aは別の未記入recordを持つ。
- **対応状況 / 再レビュー**: RESOLVED（template整備のみ。実施記録は未完） / Yes。

## 15. DD-RRR summary

| ID | Severity | Status | Phase 1A blocker | Phase 2A blocker | Phase 1B blocker |
| --- | --- | --- | --- | --- | --- |
| DD-RRR-001 | High | RESOLVED | 指摘単独ではNo。全体のhuman review/開始recordは未完 | Design gapはNo。fault/writer evidenceと専用承認までは実行Yes | Yes。G2AでDecisionJournal evidence承認まで |
| DD-RRR-002 | Medium | RESOLVED | 指摘単独ではNo | Design gapはNo。protocol modelはP2A scope、専用承認までは実行Yes | Yes。P2A/G2A view-fence evidenceまで |
| DD-RRR-003 | Medium | RESOLVED | 指摘単独ではNo | Design gapはNo。schema/DACL/migration evidenceと専用承認までは実行Yes | Yes。P2A/G2A machine-schema evidenceまで |

設計上の未解決/部分解決Critical・Highはこの3件について0件である。これは`docs/tauri-design-rerereview.md`の判定を上書きする自己承認ではなく、再々レビュー提出用のresolution記録である。

## 16. DD-RRR-001 — deadline-coupled durable CASが実装不能/不明確

- **指摘ID**: DD-RRR-001
- **重要度**: High
- **問題**: 旧設計はdurable publication完了を15秒deadlineへcoupleし、Windows file APIだけでdeadline付きCASと一意なoutcomeを成立させる未証明primitiveへ依存していた。
- **採用した設計変更**: deadlineをdecision lock内のin-memory `KEEP_AUTHORIZED` entry期限へ変更し、Confirm accepted/committed/outcome unknownを分離した。terminal decisionはfixed A/B `DecisionJournalV1`でpublish/readbackする。
- **更新文書**: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/ui-design.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`、`docs/tauri-migration.md`、`docs/tauri-design-review-checklist.md`、本書。
- **更新セクション**: requirements用語/UC-03/FR-307/309/311/404/405/409/AC-020/026、architecture 6.2/7.3/7.4/10.1/11.1/11.3/16/17、UI 4/7/8、security 5.3/7/8/13、testing 6.1/7.2/7.3、implementation Phase 1B prerequisites/2A、risks R-T18/Q02。
- **normative algorithm/schema**: architecture 7.4。final readback後、decision lock取得→identity→AWAITING→Revert未勝利→GetTickCount64→`tick <= deadline`→`KEEP_AUTHORIZED`。その後A/B読込→older/inactive fixed slotへnext `KEPT_SESSION` full write→short-write reject→FlushFileBuffers→close/reopen→A/B最大valid generation readback→commit/success。
- **crash結果**: authorization前=Revert、authorization後slot前=Revert、partial/torn=prior Revert、full write後=readbackでKeepまたはRevert、valid flush/readback後=Keep、unreadable/conflict=FAILED_CLOSED。authorization後にfull slotがsurviveした場合はKeep。
- **残余リスク**: filesystem/storage/filter/power-loss挙動、flush stall、writer alive-but-hung、termination/lock reacquisition値は未実証。documented APIだけの絶対耐久性は保証しない。
- **Phase 1A blocker**: 指摘単独ではNo。Phase 1Aはread-only境界のままで、human review/開始record未完により全体として未承認。
- **Phase 2A blocker**: 設計gapとしてはNo。P2A開始にはP1A/G1A closureと専用承認が必要で、fixed-slot/fault/writer-fencing evidence不成立ならNo-Go。
- **Phase 1B blocker**: Yes。P2A/G2Aで一意なoutcomeとwriter fencingが承認されるまで開始不可。
- **対応状況**: RESOLVED（design only）。
- **再レビュー要否**: Yes。Storage/Windows/Safety reviewerによる再々レビューが必要。

## 17. DD-RRR-002 — root remountのaccepted boot-handshake path不足

- **指摘ID**: DD-RRR-002
- **重要度**: Medium
- **問題**: lifecycle表はroot remountでrotateするとしたが、page reloadを伴わないReact root remountをRust coreが受理するversioned pathと、ordinary resyncとの区別がなかった。
- **採用した設計変更**: 6 commandを維持し、`get_display_change_status`へ`StatusRequestV1`を追加した。modeを`BOOT_HANDSHAKE/ORDINARY_RESYNC/PRESENTATION_RESYNC`に分け、root remountを明示accepted pathにした。
- **更新文書**: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/ui-design.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`、`docs/tauri-migration.md`、`docs/tauri-design-review-checklist.md`、本書。
- **更新セクション**: requirements FR-207/410/AC-025、architecture 5.1/5.4、UI 4/8.2、security 3.2/5.3/13、testing 3.3/4、implementation Phase 2A、risks R-T17、checklist 3。
- **normative algorithm/schema**: `StatusRequestV1={protocolVersion,mode,frontendBootNonce,knownControllerInstanceId,knownViewRevisionDigest,sessionId?}`。BOOT受理時に旧binding失効→new CSPRNG viewRevision発行/controller bind→old stage失効→active presentationをStage 1再構築またはRevert。deadline非延長。
- **crash結果**: page-load Started/reload/navigation/crashで即失効、Finished後はBOOT待ち。root remountはexplicit BOOT、child remount/focus/event gapはORDINARY。old Stage 2 ACKとauthority transferは拒否。repeated BOOTはavailability DoSまででKeep authorityなし。
- **残余リスク**: Tauri/WebView2 lifecycle ordering、React error boundary/remount検出、repeated-handshake rate control、packaged XSS/renderer behaviorは未実装・未検証。
- **Phase 1A blocker**: 指摘単独ではNo。P1AにReact/Tauri UIは含めない。
- **Phase 2A blocker**: 設計gapとしてはNo。protocol modelはP2A evidence対象で、開始には別承認が必要。
- **Phase 1B blocker**: Yes。P2A/G2A view-fence evidenceと後続Tauri packaged E2E gateまで。
- **対応状況**: RESOLVED（design only）。
- **再レビュー要否**: Yes。Tauri/Security/Safety reviewerがmode semanticsとDoS boundaryを確認する。

## 18. DD-RRR-003 — MachineActorRecordV1 schema/alias混在

- **指摘ID**: DD-RRR-003
- **重要度**: Medium
- **問題**: field名、instance/process identity、wire state、`ACTIVE/PREPARING/CRITICAL_UNKNOWN`等のprojection aliasが複数箇所で混在し、serializer/recovery/installerが別解釈できた。
- **採用した設計変更**: architecture 19.2へcanonical fields、13 wire enum、field groups、state別required/optional/forbidden/previous/next/maintenance/recovery/actor/WAL表、schema compatibilityを統合した。他文書は参照だけにした。
- **更新文書**: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/windows-display-research.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`、`docs/tauri-migration.md`、`docs/tauri-design-review-checklist.md`、本書。
- **更新セクション**: requirements FR-411/AC-027、architecture 10/18/19.2〜19.6、security 8.3/13、testing 7.5、implementation Phase 2A、risks R-T19、checklist 10/13。
- **normative algorithm/schema**: architecture 19.2だけがwire正本。canonical 34 fields、wire stateは`UNKNOWN,ACTIVE_INTENT,ACTIVE_PREPARED,ACTIVE_WATCHDOG_READY,ACTIVE_APPLY_ARMED,ACTIVE_MUTATED,RECOVERY_REQUIRED,RESTORING,TERMINALIZING,TERMINAL_CLEAN,MAINTENANCE_INTENT,MAINTENANCE_ACTIVE,FAILED_CLOSED`。instance IDとprocess identityは別field。
- **crash結果**: ACTIVE_INTENT前/WAL先行、PREPARED link前、WATCHDOG_READY/APPLY_ARMED/MUTATED、terminal/TERMINALIZING、clean publication、maintenance/migration crashの各pointをcanonical stateで判定し、矛盾はFAILED_CLOSED。projection名からcleanを推測しない。
- **残余リスク**: exact SDDL、standard-user write/elevated read、old installer reader、binary rollback、schema migration/power-loss、same-process instance rotateは未実証。
- **Phase 1A blocker**: 指摘単独ではNo。P1Aはnamed-object/machine-record workを含めない。
- **Phase 2A blocker**: 設計gapとしてはNo。全state serializer、DACL、old reader/recovery binary、migration fault evidenceと専用承認が必要。
- **Phase 1B blocker**: Yes。P2A/G2A canonical schema/durability/DACL evidence承認まで。
- **対応状況**: RESOLVED（design only）。
- **再レビュー要否**: Yes。Recovery/Windows Security/Installer reviewerがcanonical tableを確認する。

## 19. DD-FR summary

| ID | Severity | Related DD-RRR | Status | Phase 1A blocker | Phase 2A blocker | Phase 1B blocker |
| --- | --- | --- | --- | --- | --- | --- |
| DD-FR-001 | Medium | DD-RRR-001 | RESOLVED (design only) | 指摘単独でNo。短い再確認と開始record/承認は別 | 設計gapとしてNo。P1A/G1A、DD-FR-002 freeze、P2A専用承認まで実行Yes | Yes。P2A/G2A baseline/rollover evidence承認まで |
| DD-FR-002 | Low | DD-RRR-001, DD-RRR-003 | RESOLVED_WITH_PRE_PHASE_2A_FREEZE_CONDITION | No | Yes。Phase 2A code/file/fixture/fault harness作成前のexact freeze | Yes。G2A closureまで |

このresolution記録上、DD-FRに関する未解決Critical/High/Mediumは0件である。これは`docs/tauri-design-final-review.md`の歴史的判定を自己承認で書き換えず、設計修正を短い独立再確認へ提出できることだけを示す。DD-FR-002はPhase 1A blockerではないが、freezeなしのPhase 2Aは`NOT EXECUTABLE`である。

## 20. DD-FR-001 — `DecisionJournalV1` baseline初期化/session rollover

- **指摘ID / 重要度 / 関連DD-RRR**: DD-FR-001 / Medium / DD-RRR-001。
- **問題**: Keep publicationは定義済みだったが、fixed-name fileのfirst creation、A/B uninitialized、current `REVERT_REQUIRED` root、old session/boot/display/owner/logon/lease、baseline crash、target slot、retention/reclaim/truncateが一つのalgorithmになっていなかった。
- **採用した修正**: architecture 7.4に`ProvisionCurrentDecisionBaselineV1`を追加し、existing machine→display→user→journal-writer lock順、file classification、fresh semantic layout、session-local root、slot target、publication、rollover/reclaim、mixed-slot decision、crash table、truncate policyを一つのnormative contractへ統合した。
- **normative algorithm**: current rootはexact `generation=1,previousGeneration=0(ROOT),stateVersion=1,REVERT_REQUIRED`。foreign identityはchain比較から除外しreclaim classificationへ渡す。fresh target=A、Keep target=反対slot。baseline full-write/checksum/flush/close/reopen A/B readback→owner WAL `DECISION_BASELINE_PROVISIONED`→machine `ACTIVE_INTENT` link→owner WAL `PREPARED`→machine `ACTIVE_PREPARED`の順で、readback前にwatchdog ready/apply GO/mutationへ進まない。
- **crash result**: provisioning中は全checkpointでmutation 0件。result unknownはclose/reopen A/Bでcurrent rootを再判定し、validならlinkへ継続、invalid/absentならexact create/reclaim intentの同targetだけretry、unreadable/conflict/intent unknownは`FAILED_CLOSED`。old slotをcurrent decisionにしない。
- **reclaim rule**: normal terminalは全lock、old actor absence、old WAL exact terminal、prior machine `TERMINAL_CLEAN`、terminal digest、identity readability、old recovery binary、retentionを証明し、durable `DECISION_SLOT_RECLAIM_INTENT`を残した後だけ1 slotずつreclaimできる。critical/blocked/unknown/corrupt/unsupportedはauto-reclaim禁止。V1はwhole-file delete/truncate/recreateをrolloverに使わない。
- **residual risk**: exact byte layout/checksum/create-open-share flags/local filesystem eligibilityとWindows storage/filter/power behaviorはDD-FR-002 freeze/Phase 2A evidence待ち。fixed summary retention/quotaとsupport repair運用も後続evidence/decisionを要する。
- **Phase 1A blocker**: 指摘単独ではNo。設計の短い再確認とPhase 1A開始record/専用人間承認は別gateで、本対応でrecord入力やPhase開始はしない。
- **Phase 2A blocker**: semantic design gapとしてはNo。P1A/G1A closure、DD-FR-002 freeze、exact P2A record/専用承認がない現在は実行Yes。
- **Phase 1B blocker**: Yes。P2A/G2Aでbaseline/rollover/reclaim fault evidenceが承認されるまで。
- **対応状況**: RESOLVED (design only)。
- **短い再確認の要否**: Yes。Storage/Recovery/Safety reviewerがgeneration root、classification、publication/link order、reclaim/critical retention、crash oracleを確認する。
- **更新文書**: `AGENTS.md`、`docs/requirements.md`、`docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`、`docs/tauri-design-review-checklist.md`、本書。

## 21. DD-FR-002 — pre-Phase 2A wire/serialization/test-oracle freeze

- **指摘ID / 重要度 / 関連DD-RRR**: DD-FR-002 / Low / DD-RRR-001、DD-RRR-003。
- **問題**: `DecisionJournalV1`のnumeric layout/checksum/file policy、`MachineActorRecordV1`のexact serialization/golden vector、one-shot workerのsame-process instance-rotate rejectionがPhase 2A実装前freezeとして明記されていなかった。
- **採用した修正 / normative gate**: implementation-plan Phase 2Aの先頭にpre-code freezeを追加し、integer width/endianness/bool-enum/UUID-digest、header/A/B offset/size/file length、checksum algorithm/coverage/self field、reserved/padding/trailing/short/sparse/truncate、create/open/share/access/local filesystem/schema/test vectorをversion付きに固定する。MachineActorRecordはwidth/encoding/bounds/checksum/optional-forbidden/length/unknown-trailing/migration/old-reader/golden vectorをfreezeする。workerはsame process identity/different instance record/frameをrejectし、old process exit後にだけnew process/instanceを作る。
- **crash result**: freeze前はPhase 2A file/code/fault artifactを作らない。freeze後の不一致/unknown/trailing/short/schema mismatchは自動migrationせずfail closed。same schemaVersionの意味をfault run中に変更しない。
- **reclaim rule**: DD-FR-001のsemantic one-slot reclaim/whole-file recreate不採用をwire profileにencodeし、truncate/create dispositionがこれを弱めないことをgolden/negative vectorで固定する。
- **2026-08-13 policy approval**: 人間ownerはDD-FR-002-D01〜D08のrecommended candidateを設計方針として承認し、文書上のfreeze candidate作成だけを許可した。`DD-FR-002-WIRE-PROFILE-V1-CANDIDATE-01`と4つのvector/oracle spec labelは`DESIGN_DIRECTION_APPROVED / WIRE_CANDIDATE_PENDING_FREEZE`であり、Phase 2A実装、fixture作成・実行、display mutationは未許可である。
- **Subsequent integrated freeze-evidence authorization**: 上記は2026-08-13時点の履歴recordとして保持する。その後、full-byte fixture、expected SHA-256、semantic manifest、artifact index、aggregate hashの生成・検証、D07 controlled filesystem/DACL evidence、D08 read-only Windows evidence、formal G1A evidence bundleの作成だけが許可された。CANDIDATE-02はworker境界混在とMAP checksum vector欠落で不合格、CANDIDATE-03は77 vectorのbytes/hash/index自己整合後もD02 canonical-source、D03/SID binding、MAP resume/cleanup、DJ/MAR coverage gapsで独立review不合格となった。いずれもfreezeせず、同じIDを上書きしない。active profileは`DD-FR-002-WIRE-PROFILE-V1-CANDIDATE-04`で、590 vectorのfull bytes/hash/index生成、self-verify、再現生成、全体独立static reviewまで完了した。statusは`FULL_BYTES_GENERATED / SHA256_COMPUTED / FULL_INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING`である。これはPhase 2A product/runtime code、Tauri/watchdog/worker integration、runtime serializer/WAL、fault harness、display mutationを許可せず、`FROZEN`、artifact approval、Phase 2A executableを意味しない。
- **D04 CANDIDATE-04 resolution approval（2026-08-13）**: human ownerは`DD-FR-002-D04-C04-RESOLUTION-PACKAGE-01`を一括承認し、D04 fixture再生成とCandidate 04全体の独立reviewを許可した。許可範囲内の再生成とreviewはCLEANで完了した。Phase 2A実装とdisplay mutationは明示的に未許可であり、この承認だけを`FROZEN`またはartifact全体承認へ昇格しない。
- **residual risk**: CANDIDATE-04のexact bytes/hash/indexとindependent static reviewはCLEANだが、human freeze approval、Reviewer/Approver、immutable approval referenceは未記録である。D07は`DIRECTORY_ANCHOR_UNPROVEN / NO_GO_RECORDED`、D08はactive/sleep-resume/restart/hibernate前後合計25件、cross-sleep/cross-hibernate tick/UTC advance、sleep/hibernateでBootTime不変、full restartでBootTime/BootId変化とtick reset、各batch内boot tuple一致まで観測した。tolerance evidenceとformal bundleはpendingである。G1Aはtemplate/validatorのみでformal result evidence pendingである。Windows filesystem/storage/filter/power、DACL、old binary behaviorはP2A/8 evidenceでNo-Go判定する。
- **Phase 1A blocker**: No。P1Aはread-onlyでDecisionJournal/MachineActorRecord/file write/process workを含まない。
- **Phase 2A blocker**: Yes。freeze artifact ID/golden vectors/reviewer approvalなしにPhase 2A code/file/fixture/fault harness作成へ進まない。
- **Phase 1B blocker**: Yes。P2A/G2A closureまで。
- **対応状況**: RESOLVED_WITH_PRE_PHASE_2A_FREEZE_CONDITION。
- **短い再確認の要否**: Yes。今回はgateの完全性とPhase 1A非blockerのみ確認し、numeric value自体はP2A前freeze reviewで確認する。
- **更新文書**: `docs/implementation-plan.md`、`docs/testing-strategy.md`、`docs/risks-and-open-questions.md`、`docs/tauri-design-review-checklist.md`、本書。
