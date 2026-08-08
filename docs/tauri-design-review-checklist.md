# DisplayDeck Tauri設計レビュー チェックリスト

最終更新: 2026-08-05  
用途: 次回Tauri設計review。全項目の結果、evidence、ownerを記録するまで実装開始判定を出さない。

## 1. 判定方法

各項目を次で記録する。

- `PASS-DESIGN`: design contractが具体的で矛盾なし。実機evidenceを意味しない
- `OPEN`: 未定義/矛盾/未決定
- `SPIKE`: designはあるが、開始前にapproved spikeが必要
- `N/A`: 理由とapproverがある場合だけ

Critical safety項目は`N/A`にできない。review resultはこの文書へ直接追記するか、別のdated review recordを作る。過去のreview-resolutionを上書きしない。

## 2. Product scope

- [ ] Tauri 2、React、TypeScript、Vite、Rust、WebView2がcurrent stackとして一貫している。
- [ ] Windows 10/11だけをtargetとし、別OS product backend/runtime switchがない。
- [ ] initial releaseがsingle active path、resolution/refresh、temporary applyへ限定されている。
- [ ] scale mutation、multi-monitor mutation、DLDSR-specific behavior、preset/launch/tray/updateが除外されている。
- [ ] scaleはread-only status rowで、Apply payload/commandに含まれない。
- [ ] session-only Keepとprofile persistenceの違いが明確で、DDR-Q01が未決定として残る。
- [ ] all-process/watchdog lossを15秒保証と誤記していない。

## 3. Tauri command境界

- [ ] public commandが`get_display_snapshot`、`begin_display_change`、`ack_display_change_presentation`、`confirm_display_change`、`restore_display_change`、`get_display_change_status`に限定される。
- [ ] queryをatomic snapshotにまとめる理由が妥当である。
- [ ] public prepare/apply分離を採用しない理由と内部durable phaseが明確である。
- [ ] 各commandにarg、result、error、allowed state、idempotency、timeout、concurrencyが定義される。
- [ ] frontendがraw width/Hz/device path/Win32 flag/executable/journal pathを渡せない。
- [ ] invoking window/local originをRust側でも検証する。
- [ ] eventsがhintだけで、terminal decision/Keep/Revertに使われない。
- [ ] presentation ACKが専用typed commandで、stage/token/generation/lease/view/nonce、timeout、duplicate/stale handlingを持つ。
- [ ] `viewRevision`がcore-issued 128-bit以上CSPRNG view-instance tokenで、初回/reload/crash/recreate/handshake/controller/presentation reconstructionとfocus/minimize/renderのrotate/non-rotate表を持つ。
- [ ] Stage 1 ACK権限をreload後viewへ移送せず、current view/session/lease/generation/stage token/deadline/payloadのexact tupleでだけACKする。
- [ ] `StatusRequestV1`の`BOOT_HANDSHAKE/ORDINARY_RESYNC/PRESENTATION_RESYNC`、frontendBootNonceの非authority性、root remount accepted path、active presentation再構築が定義される。
- [ ] event gap/focus/child remountのordinary resyncがviewをrotateせず、root remount/reload/renderer復旧と混同されない。

## 4. CapabilitiesとPermissions

- [ ] single main WebViewWindowだけにcustom command permissionをgrantする。
- [ ] registered app commandのdefault exposureをapp manifest/Capabilityで限定する設計がある。
- [ ] exact Permission identifierをselected Tauri version generated schemaで確認するgateがある。
- [ ] shell、process、filesystem、HTTP、opener、updater、tray、global shortcut等のfrontend permissionがない。
- [ ] remote Capability/originがない。
- [ ] Capability重複によるpermission mergeを検査する。
- [ ] frontendからsidecar spawnできない。

## 5. CSP/WebView2/content

- [ ] production CSPがself-onlyを基本とし、`unsafe-eval`、remote script/CDN、wildcardを許さない。
- [ ] Tauri internal IPCへ必要なCSP sourceだけをpackaged evidenceで追加する。
- [ ] external navigation、popup、新規WebView、download、file/data/javascript schemeをdenyする。
- [ ] dev server URL、debug capability、DevToolsがproduction artifactへ残らない。
- [ ] WebView2 version/install mode/support matrixがdecision gateにある。
- [ ] WebView2欠損/old/offline/proxy behaviorをinstaller testに含める。

## 6. Rust validation/unsafe

- [ ] command DTOのunknown field、length、alphabet、range、versionを検査する。
- [ ] monitor/mode tokenをfresh enumerationへexactに再解決する。
- [ ] sessionId/generation/leaseVersion/epoch/bootId/owner/logon/display/actor/state/deadlineをACK/confirm/restoreで照合する。
- [ ] native outputのsize/count/index/UTF-16/enum/rationalを検査する。
- [ ] exactly one expected observationなしでは`canApply=false`となる。
- [ ] Rust `unsafe`がWindows FFI/process boundaryに限定される。
- [ ] 各unsafe blockのpointer/buffer/union/handle/thread invariantとtest ownerがある。
- [ ] raw handle/pointer/native bufferがdomain/command/protocolへ漏れない。

## 7. Windows API selection

- [ ] `EnumDisplayDevicesW`、`EnumDisplaySettingsExW`、`ChangeDisplaySettingsExW`の役割と限界が区別される。
- [ ] `QueryDisplayConfig`、`DisplayConfigGetDeviceInfo`、`SetDisplayConfig`の役割とWindows差が区別される。
- [ ] `CDS_TEST`/`SDC_VALIDATE`がactual apply guaranteeではないと明記される。
- [ ] `CDS_UPDATEREGISTRY`/`SDC_SAVE_TO_DATABASE`/unsafe modeをinitial releaseで使わない。
- [ ] GDI candidateとCCD rational readbackのmapping ruleが推測でない。
- [ ] Windows 10/11 virtual refresh flag差を扱う。
- [ ] RDP/virtual/special display/HDR/color depth/driver dependencyをfail-closedまたはmatrix化する。
- [ ] Microsoft `windows` crateのexact version/feature/bindingをspike後に固定する。

## 8. Watchdog independence

- [ ] watchdogがTauri core/WebViewと別processである。
- [ ] watchdog自身がblocking display APIを呼ばず、one-shot workerへ委譲する。
- [ ] worker terminal frameだけでなくprocess exitを証明してから次operationを始める。
- [ ] Tauri forced kill/Task Manager/panic/normal quitでwatchdogが生存する実機gateがある。
- [ ] parent Job、handle inheritance、sidecar launcherをPhase 2Aで比較する。
- [ ] watchdog start/handshake/identity verification失敗時mutation 0件である。
- [ ] watchdog単独crash/hangをheartbeat/process handleで検出し、replacementのlock/lease/old-worker fencingとall-process loss境界が明確である。
- [ ] heartbeat miss、suspect、exit、alive-hung、IPC stall、resume、starvation/security-product delay、termination failureを区別し、250ms/4回等をPhase 2A候補値として扱う。
- [ ] `HeartbeatPolicyV1`の測定項目、false-positive evidence、人間承認、未承認時mutation disabledが定義される。
- [ ] Phase 2A測定にjitter/CPU/sleep-debugger/AV/disk/process-handle/termination access/launchが含まれ、0 unproven takeover/double actorを昇格条件にする。
- [ ] frontendへwatchdog path/argument/permissionを公開しない。

## 9. Recovery file/WAL

- [ ] C0/P0/R、target/topology、expected observation、support fingerprintがschema化される。
- [ ] dual-slot generation/digest/required-field/transition validationがある。
- [ ] flush/close/reopen verification後だけGOを渡す。
- [ ] worker identityをGO前にdurable化する。
- [ ] pointer/raw buffer/driver-private bytes/arbitrary pathを保存しない。
- [ ] fixed user local-data directory、DACL、reparse/hardlink/final pathを検討する。
- [ ] tamper/corruption/unknown schemaでarbitrary restoreしない。
- [ ] terminal前にjournalを削除せず、critical stateを保持する。
- [ ] startup recovery decision tableが全durable stateを一意に扱う。
- [ ] deadline前のdecision lock内`KEEP_AUTHORIZED` entryが唯一のConfirm linearization pointで、accepted/committed/outcome unknownが区別される。
- [ ] `DecisionJournalV1`が1 file/fixed header/fixed-size A/B/no active selector、canonical slot fields、`REVERT_REQUIRED/KEPT_SESSION`だけのwire decisionで定義される。
- [ ] architecture 7.4 `ProvisionCurrentDecisionBaselineV1`がFILE_ABSENT、A/B UNINITIALIZED、current baseline/terminal、old normal/critical、mixed/corrupt/unsupported/conflictingを排他的にclassifyする。
- [ ] current rootが`generation=1,previousGeneration=0(ROOT),stateVersion=1,REVERT_REQUIRED`、generationがsession-localで、foreign session/boot/display/owner/logon/leaseをchainへ接続しない。
- [ ] fresh fileがtrusted fixed path/filename、`CREATE_NEW`、exact symbolic length、valid header、A/B all-zero UNINITIALIZED、zero reserved/padding、no sparse/trailing/reparseで定義される。
- [ ] baseline write/flush/close/reopen/readback後にだけWAL `DECISION_BASELINE_PROVISIONED`→machine `ACTIVE_INTENT` link→WAL `PREPARED`/machine `ACTIVE_PREPARED`へ進み、それまでmutation 0件である。
- [ ] fresh baseline target A、Keep targetはbaseline反対slot、Keep readbackまでbaseline不変、partial/torn Keepでbaselineが残る。
- [ ] baseline provisioningのcreate/header/size/slot write/flush/close/reopen/readback/WAL link/machine link/PREPARED各crash pointが一意で、foreign slotをcurrent decisionにしない。
- [ ] normal terminal reclaimはmachine/display/user/writer locks、old actor absence、old WAL terminal、prior machine TERMINAL_CLEAN、digest/identity/retention、durable reclaim intentを要求する。
- [ ] critical/blocked/unknown/corrupt/unsupported evidenceのauto-reclaim、active/unresolved/unknown fileのtruncate/delete/recreateが禁止される。
- [ ] partial/torn/short write、generation gap/previous mismatch/conflict、flush/readback/response前crash、deadline直前entry、entry後I/O delay、outcome unknownのrecovery tableがある。
- [ ] `MoveFileExW`/`ReplaceFileW`をdeadline CASとせず、`FlushFileBuffers`/`FILE_FLAG_WRITE_THROUGH`単独をatomicity/CAS証明にしない。AV/filter/power-lossをPhase 2Aで証明できなければNo-Goとする。

## 10. Session/state/fencing

- [ ] IdleからFailedまでのstateとvalid/invalid transitionが定義される。
- [ ] apply中再apply、restore中confirm、terminal reapplyを拒否する。
- [ ] sessionId、epoch、owner nonce、operation sequence、generationを持つ。
- [ ] watchdog/worker PIDにcreation time/image/role/nonceを組み合わせる。
- [ ] machine-wide maintenance/mutation gate、per-display mutation lock、per-user/logon recovery lockのscopeと取得順が定義される。
- [ ] architecture 19.2に`MachineActorRecordV1` canonical fieldsと唯一の13-state wire enumがあり、他文書が再定義していない。
- [ ] 各wire stateのrequired/optional/forbidden fields、previous/next、maintenance、recovery、actor presence、WAL consistencyが定義される。
- [ ] controller/watchdog/workerのinstance IDとprocess identityが別fieldで、same process内instance rotateをfenceできる。
- [ ] machine ACTIVE_INTENT durable publication→per-user PREPARED/ACTIVE_PREPARED、およびper-user terminal→TERMINALIZING→worker/actor quiescence→machine TERMINAL_CLEANの順序と逆順releaseが定義される。
- [ ] unknown schema/enum、required不足、forbidden混入、old reader、recovery binary保持、schema migration crash、UI projection/wire混同をfail closedにする。
- [ ] machine record/per-user WALの片側欠落、不一致、read/ACL failureを含むcrash/consistency tableがある。
- [ ] workerはone-shot processでsame-process `workerInstanceId` rotate/reuseを禁止し、same process identity/different instanceのrecord/frameをrejectするnegative oracleがある。
- [ ] initial mutationが唯一のlocal active-console logonだけに限定され、Fast User Switching/RDP/second logonで未authorized sessionを拒否/restoreし、authorization後はold frontendをfenceしてjournal outcomeへ従う。
- [ ] abandoned mutexをfree permissionと扱わずjournal recoveryへ進む。
- [ ] PID reuse先をkillせず、query不能はblockedにする。
- [ ] double launch/double Keep/double restore/stale actorをtestする。

## 11. Apply/confirm/rollback

- [ ] watchdog ready、WAL durable、preflight success前にmutationしない。
- [ ] profile/registry不変のtemporary applyである。
- [ ] post-apply GDI/CCD readbackがexactly expectedでなければrollbackする。
- [ ] confirmationのtwo-stage presentation ack、2秒/残12秒proposalをreviewする。
- [ ] Confirming中もRevertをwatchdogへforwardし、`KEEP_AUTHORIZED`前はdecision-lock race、authorization後はordinary Revert拒否、valid `KEPT_SESSION` readback後はterminal rejectとなる。
- [ ] `GetTickCount64`、wall clock diagnostic、bootId、sleep/hibernate/resume、30秒pre-apply、60秒max lifetimeが一意に定義される。
- [ ] 15秒の測定点、Keep受付、rollback call-start、baseline SLOを人間が決める。
- [ ] 15秒がKeep意思の`KEEP_AUTHORIZED` entry期限であり、disk flush完了期限でない。React successはdurable commit後だけである。
- [ ] Keep前にcurrent=R/persisted=P0を確認する。
- [ ] normal rollbackがC0 exact、P0 fallbackがdegradedである。
- [ ] worker exit未確認では並行rollback callを出さずblockedになる。
- [ ] full-process lossのblind recoveryを各support cellでtestする。

## 12. UI/accessibility

- [ ] current/plannedを色だけでなくtext/semanticsで区別する。
- [ ] candidate多数時にselectを主controlにするruleがある。
- [ ] ApplyまでOS変更しないことがAPI call testで示される。
- [ ] transaction中controlをdisableする。
- [ ] confirmation window/overlayをshow/focus/topmostできない場合に即時rollbackする。
- [ ] Rust/React state mismatchをstatus commandで修復する。
- [ ] React `remainingMs`がadvisory projectionであり、absolute native deadlineを受け取らず、Keep可否oracleにならない。
- [ ] startup pendingを確認再開せずrestoreする。
- [ ] black screenでUI操作不能でもwatchdogが動く。
- [ ] Escape=Revert、Enter/initial focus policyをDDR-Q05として決定する。
- [ ] high contrast、200% zoom、Narrator、fixed 15秒をphysical Windowsでtestする。

## 13. Installer/sidecar

- [ ] Tauri bundlerにwatchdog/workerを全target architectureで同梱するplanがある。
- [ ] main/watchdog/worker/DLL/installerの署名publisher/timestamp policyがある。
- [ ] NSIS per-machine/per-userとMSIをsecurity/operationで比較する。
- [ ] protected install rootとruntime asInvokerの方針を決める。
- [ ] active/pending/degraded/failed/blocked recovery中のupgrade/uninstallを安全に拒否/復旧する。
- [ ] protected machine actor recordとall-session process discoveryにより、別userのpending/critical/unknown recovery中はmaintenanceを拒否し、installerが他owner journalをrestoreしない。
- [ ] update/repair/uninstallのbegin/complete fence、`MAINTENANCE_ACTIVE`、owner WAL read-only照合、old recovery reader保持、completion/rollback recordが定義される。
- [ ] maintenance/update/repair/uninstallの各begin/completeにmachineEpoch、bootId、binary version、terminal generation、actor、owner、nonce、intent/completion、reject codeがある。
- [ ] machine recordまたはper-user WALがunreadable/corrupt/stale/inconsistentならmaintenanceをfail closedで拒否する。
- [ ] clean install/repair/update/uninstall/WebView2/offline/AV/SmartScreenをtestする。

## 14. Windows 10/11実機matrix

- [ ] exact edition/build/KB/architectureをfreezeする。
- [ ] Windows 10 EOL、ESU/LTSC/consumer support方針を決める。
- [ ] GPU vendor/model/driver、display/firmware/connection、WebView2をexact cellにする。
- [ ] NVIDIAとAMD/Intelのどこまでをsupportするか明示する。
- [ ] 59.94/60、C0!=P0、HDR/DRR、current-not-listed、hotplug/remote/virtualを扱う。
- [ ] mandatory repetitionとzero-tolerance failure policyを維持する。
- [ ] untested cellをsupportedと表示しない。

## 15. 最終review判定

- [ ] `docs/requirements.md`、architecture、research、UI、security、test、plan、risks、migrationが矛盾しない。
- [ ] historical `docs/design-review.md`/`docs/review-resolution.md`とcurrent designのstatusを混同していない。
- [ ] Critical/High相当のopen design gapがない。
- [ ] Product/Safety/Security/Accessibility/Release ownerのdecision項目が列挙される。
- [ ] 最初の承認範囲がPhase 1A read-onlyだけか、別範囲か明記される。
- [ ] roadmapがP0→P1A→G1A→P2A→G2A→P1B→P2B→P3以降で、P2BをP1Bの前提にしていない。
- [ ] Phase 1A recordにexact call allowlist、field-by-field redaction/evidence、Target Machine、Operator/Evidence Owner/Reviewer、human approvalが未記入templateとして残り、named mutex/DACL作業を含まない。
- [ ] DD-FR-001が`RESOLVED`で、DD-FR-002が`RESOLVED_WITH_PRE_PHASE_2A_FREEZE_CONDITION`としてPhase 1A非blocker/Phase 2A pre-code blockerに記録される。
- [ ] Phase 2A code/file/fixture/fault harness作成前にDecisionJournal wire layout、MachineActorRecord serialization/golden vector、worker one-shot negative oracleをversion付きfreezeするgateがある。
- [ ] 設計承認と技術spike承認とproduct implementation承認を分離する。

## 16. Review result template

```text
Review date:
Reviewers / roles:
Document revisions:
PASS-DESIGN items:
OPEN items:
SPIKE items:
Human decisions recorded:
Phase authorized (if any):
Explicitly unauthorized phases:
Release impact:
Next review trigger:
```
