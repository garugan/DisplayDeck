# DisplayDeck リスクと未決定事項

最終更新: 2026-08-05  
状態: Tauri移行後。全項目は未検証。明示されたowner decisionなしに確定しない。

## 1. 評価尺度

| Level | 意味 |
| --- | --- |
| Critical | black screen/誤target変更/復元不能/安全保証破綻。未解消ならmutation release不可 |
| High | support、security、data durability、配布成立性を大きく損なう |
| Medium | UX、保守、限定環境での機能低下。明示対策またはscope除外が必要 |
| Low | 初期releaseを止めないが、誤解/運用costを増やす |

Status:

- `Open`: 設計/実機evidence/人間decisionが不足
- `Mitigated by design`: controlは定義したが実装/evidenceなし
- `Decision required`: product/support/budget/SLA/accessibilityの人間判断
- `Spike required`: exact Windows/Tauri/package behaviorを実証して決める

## 2. 最大の技術的リスク

最大のriskは、Win32 display operationがdriver/kernel側で長時間blockまたは結果不明となり、watchdogが旧workerの終了を証明できず、安全にC0 restoreを開始できないことである。

設計はwatchdogとone-shot workerを分離し、worker exit未確認では並行Win32 callを禁止する。しかし、これは競合変更を防ぐ一方、15秒内の復元を保証できないblocked stateを生む。Phase 1B/2Bで1件でも再現しroot causeを除去できないsupport cellは除外する。並行callで見かけ上戻す回避策は採らない。

## 3. 技術的リスク

| ID | Level | Risk | 現在の対策 | 残る判断/検証 |
| --- | --- | --- | --- | --- |
| R-T01 | Critical | Tauri core終了時にwatchdogも同じJob/process treeで終了 | independent sidecar、private pipe、lifecycle spike | packaged NSISでTask Manager/panic/killを実証 |
| R-T02 | Critical | watchdogがblocking Win32 callで停止し期限を監視できない | one-shot worker separation | worker process exit/quiescenceとdriver hang実機test |
| R-T03 | Critical | journal write crashでbaseline/intentが不明 | dual-slot WAL、flush/reopen、write-ahead GO | storage/AV/power fault evidence |
| R-T04 | Critical | stale watchdog/worker/recoveryがlater sessionへoperation | mutex、epoch、nonce、creation time/image identity | PID reuse/OpenProcess denied/Job競合 |
| R-T05 | Critical | GDI candidateとCCD readbackが曖昧 | exactly-one mapping、lab qualification | 59.94/60、virtual refresh、vendor/driver matrix |
| R-T06 | High | Tauri event lossでReactとRust state不一致 | eventはhint、status commandを正本 | WebView suspend/reload/E2E |
| R-T07 | High | sidecar protocol deadlock/log flood | framing/budget、stdout/stderr並行drain | actual Tauri/process pipe behavior |
| R-T08 | High | recovery fileが改ざん/置換され任意modeへ誘導 | ACL、digest/schema、fresh target/current validation | reparse/hardlink/DPAPI-HMAC比較、same-user threat受容 |
| R-T09 | High | Rust `unsafe` defectでmemory/handle corruption | FFI boundary限定、safe wrapper、fuzz/inventory | selected crate/API binding review |
| R-T10 | Medium | Tauri 2 Permission/Capability identifier/version差 | generated schemaでfreeze | Phase 3 exact version |
| R-T11 | Medium | frontend mockがproductionへ残る | build-time compositionとartifact test | Phase 3/4 review |
| R-T12 | Medium | startup recoveryとfresh app起動がrace | startup recovery first、OS-wide lock | multi-instance/process startup test |
| R-T13 | Critical | watchdog単独crash/hangでrollback ownerを失う | provisional `HeartbeatPolicyV1`、process handle、replacement watchdog、leaseVersion、old actor/worker fencing | Phase 2Aでmiss/hung/IPC/resume/starvation/AV/exitを測定しproduct値を人間承認。termination denied/replacement failure |
| R-T14 | Critical | sleep/clock/boot差でKeep期限が延長またはstale sessionを再開 | `GetTickCount64`、fixed t0/deadlines、bootId cross-check、未authorized resumeはrestore、authorized後はjournal outcome | Win10/11 sleep/hibernate/Fast Startup/WMI evidence |
| R-T15 | Critical | Fast User Switching/別session actorが同じdisplay stackへ競合 | initial single active-console logon、machine gate、per-display/user locks、authorization前session change Revert、後はfrontend fence | Global namespace/DACL/session notification実機test |
| R-T16 | Critical | installerが別userのlive recovery binaryを置換/削除 | machine gate、protected actor record、terminal-clean all-session gate、owner journal非代行 | NSIS/MSI/upgrade/uninstall/FUS evidence |
| R-T17 | High | root remount/reload/crash前WebViewまたはXSSが新presentationをACK | `StatusRequestV1` BOOT_HANDSHAKE accepted path、core-issued CSPRNG `viewRevision`、controller/session/lease/generation/stage exact binding、ordinary resync分離、authority transfer禁止 | Phase 2A/React E2Eでroot/child remount、repeated handshake、navigation/crash/old renderer injection |
| R-T18 | Critical | Confirm authorization、deadline/Revert、slot I/O crashが競合しKeep/Revertが二重確定またはoutcome unknown | decision lock内`KEEP_AUTHORIZED` entry、fixed A/B `DecisionJournalV1`、valid terminal readbackだけをcommit、unknownはreadback、writer fencing | Phase 2Aでshort/torn write、flush/readback、AV/filter/power-loss、alive-hung writer/replacementを実証。不成立cellはNo-Go |
| R-T19 | Critical | machine actor recordとper-user WALがcrash/schema aliasで矛盾しmaintenanceがrecovery binaryを削除 | architecture 19.2のsingle canonical field/wire schema、ACTIVE_INTENT→PREPARED link、terminal→TERMINALIZING→quiescence→TERMINAL_CLEAN | Phase 2A/8で全state/field rule、old reader/recovery binary、migration crash、DACL/FUS/update fault evidence |
| R-T20 | High | fixed-name `DecisionJournalV1`のfirst creation/session rollover/reclaim crashでcurrent rootとold evidenceを混同または喪失 | architecture 7.4 `ProvisionCurrentDecisionBaselineV1`、session-local generation 1/ROOT、one-slot reclaim intent、critical evidence retention、whole-file recreate不採用 | Phase 2Aでfile/create/write/flush/power/AV/full corruptionのfault evidence、DD-FR-002 wire freeze |

## 4. UX上のリスク

| ID | Level | Risk | 対策/未決定 |
| --- | --- | --- | --- |
| R-U01 | Critical | 画面が映らずUI操作不能 | UI非依存watchdog、blind recovery。watchdog loss境界はQ02 |
| R-U02 | High | confirmationがbehind/off-screen/focusなし | same-window overlay、show/focus/topmost、専用commandの2-stage ACK。missing/stale/lateはrestore |
| R-U03 | High | 15秒がNarrator/運動障害に短い | Q02/Q05の人間判断。固定値変更は安全SLAとセットでreview |
| R-U04 | High | Enterを誤ってKeepとして押す | Q05。safe baselineはRevert focus/Enter focused button |
| R-U05 | Medium | candidate多数でsliderが操作困難 | 9件以上等でselect主操作。閾値usability test |
| R-U06 | Medium | session-only Keepを永続保存と誤認 | explicit copy。Q01承認前はrelease不可 |
| R-U07 | Medium | scale rowが変更できると期待 | read-only label/理由、Apply diffへ含めない |
| R-U08 | Medium | DLDSR-like candidate除外を機能欠落と感じる | optional read-only diagnosticsはQ06。unsafe candidate tokenなし |
| R-U09 | High | degraded/failedを「戻った」と誤認 | distinct critical copy、diagnostic ID、Apply禁止 |

## 5. Windows API上のリスク

| ID | Level | Risk | 影響/対応 |
| --- | --- | --- | --- |
| R-W01 | Critical | `ChangeDisplaySettingsExW` dynamic applyがP0/HDR/colorへ想定外影響 | before/after readback、non-persistent flag、cell除外 |
| R-W02 | Critical | C0 allowlisted fieldだけでexact restore不能 | current-not-listed/hard-excluded、mutation中止 |
| R-W03 | High | `CDS_TEST`/`SDC_VALIDATE`成功とactual apply不一致 | preflightを保証とせずwatchdog/readback |
| R-W04 | High | `SetDisplayConfig`がbest-mode logicでtupleを調整 | `SDC_ALLOW_CHANGES`原則不使用、exact readback |
| R-W05 | High | Windows 10/11 virtual refresh flag差 | runtime version check、separate support cells |
| R-W06 | High | RDP/virtual displayをphysical targetと誤分類 | local console/single path/exact classifier、ambiguous拒否 |
| R-W07 | High | driver updateでqualification fingerprint失効 | exact manifest、updateごと再qualification |
| R-W08 | High | HDR/advanced color/VRR/DRRの非意図変更 | read-only observation、変化/unknown cell除外 |
| R-W09 | Medium | scale current percentをDPIから誤推測 | known/unknown/unsupported、mutationなし |
| R-W10 | High | documented scale setter/rollbackがない | Phase 9別spike、registry/undocumented API不採用 |

## 6. 配布上のリスク

| ID | Level | Risk | 対策/決定 |
| --- | --- | --- | --- |
| R-D01 | High | per-user writable installでwatchdog置換 | signed per-machine NSIS first candidate。Q04 |
| R-D02 | High | unsigned/low-reputation artifactをAV/SmartScreenが妨害 | Authenticode/timestamp予算・publisher運用。Q04 |
| R-D03 | Critical | upgrade/uninstallが別userのpending recovery binaryを削除 | machine gate、all-session actor record/process discovery、terminal-clean拒否contract、installer test |
| R-D04 | High | sidecarがarchitecture別に欠落/誤命名 | Tauri bundler packaged artifact inspection |
| R-D05 | Medium | WebView2 downloadがoffline/proxyで失敗 | bootstrapper/embedded/offline比較。Q08 |
| R-D06 | Medium | MSI/NSISでinstall/repair behavior差 | NSIS first、MSI comparison spike |
| R-D07 | High | Windows 10 EOLへの配布責任 | ESU/LTSC/consumer方針。Q03 |
| R-D08 | High | x64/arm64双方を十分testできない | architectureをexact matrixへ限定。Q03 |

### 6.1 `DecisionJournalV1` lifecycle: 設計決定と未実証事項

| Topic | 設計で決定済み | Phase 2A evidence / residual decision |
| --- | --- | --- |
| fixed journalの長期retention | normal terminal full slotは`TERMINAL_CLEAN`、actor exit、次sessionのdemand-driven reclaimまで保持。critical/blocked/unknownは自動reclaimなし | normal audit summaryのbounded retention/quota、support export/archive運用をfreeze |
| old terminal evidenceの肥大化 | full slotは2つに限定し、normal reclaim前にbounded digest/identity/reasonをoperational historyへ残す | summary chain/quota上限、diagnostic bundleとprivacy/access policy |
| file全体が破損/unreadable | 自動truncate/delete/recreateを禁止し`FAILED_CLOSED`。support-approved evidence-preserving procedureまでmutation block | bit/sector/power/filter faultでのclassification、support repair/schema migration procedure |
| disk full | baseline/Keep/reclaim writeを成功とせず、close/reopen oracle後にno mutation/判定不能はfailed closed | allocation、full write、flush、metadata updateごとのfault injection |
| antivirus/EDR lock | actorまたはfile ownershipを推測せず、sharing/access/stallはretry budgetまたはblocked | productごとのlatency/share/filter matrix、false-positive/retry budget |
| unsupported filesystem/volume | approved local fixed volume/filesystem以外はmutation disabled。sparse/network/removable/cloud/reparseは拒否 | exact eligible list、file identity/final path/sparse detection APIとgolden evidence |
| power-loss後のbaseline判定 | current valid generation 1/ROOTのreopen readbackだけを採用。old identityをcurrent decisionにしない | power-loss相当/OS crash/storage cacheでsurvivalとFAILED_CLOSED分類 |
| schema migration | unknown schema自動初期化と上書きを禁止。old recovery reader/binaryをmigration完了まで保持 | DecisionJournal/MachineActorRecordのold/new reader golden vector、migration crash |
| old recovery binary | old schema/evidenceを読めない間はupdate/uninstallで削除しない | signed binary coexistence、installer/AV/share/rollback evidence |
| normal terminal evidence retention期間 | wall-clock日数ではなく、`TERMINAL_CLEAN` + actor exit + next-session demandというlogical minimumを固定。次sessionがなければ残す | product/supportがこれより長い期間を要求するか、quotaとprivacyをfreeze |
| userによるjournal削除 | machine/WAL historyがprior fileを要求するにもかかわらずmissingなら`FILE_ABSENT`にせず`CORRUPT_OR_UNREADABLE/FAILED_CLOSED` | ACL、delete/share、user tamper、owner support recoveryのevidence |
| cleanup crash | terminalizationとcleanup/reclaimを別operationにし、両slotを同時に変更しない | cleanup checkpointごとのprocess/OS crash、audit summary survival |
| reclaim crash | target/full digestをdurable intentにbindし、current partialは同targetにだけretry。他slotのold evidenceを保持 | intent write/flush、partial baseline、baseline link、Keep target overwriteごとのfault injection |

DD-FR-001で上記semantic lifecycleは決定した。2026-08-13にDD-FR-002-D01〜D08のrecommended design directionは承認され、numeric layout、checksum、SYSTEM/single-owner DACL、separate MachineActor provision、boot/evidence binding、worker one-shot oracleの文書candidateを作成した。CANDIDATE-03は77 vectorのbytes/hash/index自己整合後もD02 canonical-source、D03/SID binding、MAP resume/cleanup、DJ/MAR coverage gapsで独立review不合格となり、freezeしていない。D04のnormal/critical owner terminal集合、EMPTY exclusion、uninstall deferral、initial-provision structural/readiness splitをまとめた`DD-FR-002-D04-C04-RESOLUTION-PACKAGE-01`は2026-08-13にhuman ownerが一括承認した。active CANDIDATE-04は590 vectorのfull bytes/hash/indexを生成し、self-verify、再現生成、Candidate 04全体の独立static reviewがCLEANである。statusは`FULL_BYTES_GENERATED / SHA256_COMPUTED / FULL_INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING`である。D07は`DIRECTORY_ANCHOR_UNPROVEN / NO_GO_RECORDED`、D08はactive/sleep-resume/restart合計15件がcapture/validationを通過し、cross-sleep tick/UTC advance、sleepでBootTime不変、full restartでBootTime/BootId変化とtick reset、restart内boot tuple 5/5一致を観測したが、tolerance evidenceとformal bundleはpendingである。G1Aはformal result evidence pendingである。これはPhase 1A blockerではないが、Phase 2A code/file/fixture/fault harness作成前にhuman freeze approvalを記録する。Phase 2A実機/fault evidenceが不成立なcellはPhase 1B No-Goである。

## 7. 未決定事項

### Q01 — Keepの永続性（旧DDR-Q01）

- **Status**: Decision required
- **Question**: 初期版をsession-only Keepとし、reboot後の永続性を約束しない案を承認するか。
- **Safe proposal**: P0を変更せずsession内だけRを維持する。profile persistenceはpost-v1。
- **Owner**: Product + Safety
- **Block**: mutation release

### Q02 — 15秒保証と全process loss（旧DDR-Q02/DDR-003）

- **Status**: Decision required
- **Question**: 15秒をKeep受付期間、rollback decision、最初のOS call開始、baseline完了のどこまで外部保証するか。watchdog loss/all-process lossを保証に含めるか。
- **Safe proposal**: verified readback durable`t0`から`GetTickCount64`で15秒をKeep意思の`KEEP_AUTHORIZED` entry期限とする。deadline前のdecision lock raceはRevert class/Confirmの先行linearizationで決め、authorization後のdisk I/Oは15秒外、React successはdurable terminal readback後だけとする。deadline後call-start target、commit I/O hang/heartbeat値はPhase 2A測定候補でproduct値ではない。watchdog単独lossはlive takeover、core+watchdog/all-process/OS crash/power lossは15秒外、各cellでblind P0 recovery必須。
- **Owner**: Product + Safety + Legal/Support
- **Block**: mutation release/copy

### Q03 — Windows 10/11 support matrix（旧DDR-Q03を新方針で再open）

- **Status**: Decision required
- **Fixed**: product targetはWindows 10と11。
- **Open**: exact editions/builds/KB、ESU/LTSC/unsupported consumer Windows 10、x64/arm64、GPU vendors、driver/display/connection。
- **Safe proposal**: target名だけで公称せず、finite manifest合格cellだけsupportする。
- **Owner**: Product + Release + Security
- **Block**: Phase 1 equipment、release statement

### Q04 — Installer、署名、install mode（旧DDR-Q04）

- **Status**: Decision required
- **Question**: NSIS per-machine/per-user、MSI必要性、certificate/timestamp/revocation、protected install budget。
- **Safe proposal**: signed NSIS per-machine、protected root、runtime asInvoker、machine maintenance/mutation gateとall-session actor recordによるterminal-clean gate。MSI比較。
- **Owner**: Release + Security + Budget owner
- **Block**: Phase 8/public release

### Q05 — Confirmation focus/Enter/accessibility（旧DDR-Q05）

- **Status**: Decision required
- **Question**: user requestのEnter=Keepをglobal shortcutにするか、safe baselineのRevert初期focus/Enter focused buttonとするか。固定15秒をaccessibility上受容するか。
- **Safe proposal**: Revert focus、Keep default/global shortcutなし、Escape/close=Revert。
- **Owner**: Product + Accessibility + Safety
- **Block**: UI acceptance/mutation release

### Q06 — DLDSR-like candidate表示（旧DDR-Q06）

- **Status**: Decision required after Phase 1A
- **Question**: preferred超/分類不能candidateをread-only diagnosticsへ表示するか。
- **Fixed safety floor**: exact mapping/qualificationなしにselection tokenを発行しない。DLDSRと推測命名しない。
- **Owner**: Product + Safety
- **Block**: diagnostics UXのみ。mutation可否のsafe floorは確定

### Q07 — Watchdog起動方法とindependence

- **Status**: Spike required
- **Fixed design**: watchdog単独lossはTauri core monitorがreplacement watchdogをfixed signed pathから起動し、machine/display/user lock、old exit/worker quiescence、leaseVersionでtakeoverする。confirmationは再開しない。
- **Question**: Tauri sidecar Rust API、Rust standard process、Win32 `CreateProcessW` wrapperのどれがfixed path、handle allowlist、parent-loss survival、Job separation、承認済みheartbeat/termination/replacement policyを満たすか。
- **Safe proposal**: bundlerは同梱だけに使い、frontend shell Permissionなし。Phase 2Aで候補値、false-positive、hang/exit判定を比較して人間承認し、証明不能ならmutation中止。
- **Owner**: Architecture + Safety + Security
- **Block**: Phase 2B/6

### Q08 — WebView2 distribution

- **Status**: Decision required after Phase 3/8 evidence
- **Question**: download bootstrapper、embedded bootstrapper、offline installer、fixed runtime、minimum version。
- **Safe proposal**: `skip`をdefaultにせず、offline要件/size/security servicingを比較する。
- **Owner**: Release + Product + Security
- **Block**: installer release

### Q09 — Initial API strategy

- **Status**: Spike required
- **Question**: single-path applyをGDI hybridで確定するか、CCD supplied configを採るか。
- **Safe proposal**: GDI列挙/`CDS_TEST`/dynamic apply + CCD exact readbackをbaseline候補にし、Phase 1Bで副作用が少ない方だけ採る。
- **Owner**: Windows + Safety
- **Block**: Phase 2B/6

### Q10 — Journal authenticity

- **Status**: Spike/security decision required
- **Question**: ACL+digest+fresh revalidationに加え、DPAPI-protected HMAC等を採用するか。same-user attackerをどこまでthreatに含めるか。
- **Safe proposal**: HMAC有無にかかわらずjournalからarbitrary modeをblind applyしない。unknown/tamperedはfail closed。
- **Owner**: Security + Safety
- **Block**: Phase 2 closure

### Q11 — Scale future implementation

- **Status**: Deferred
- **Question**: documented setter、候補、relogin/Explorer、rollbackが成立するか。
- **Safe proposal**: initial release read-only。Phase 9で不成立なら実装しない。
- **Owner**: Product + Windows + Safety
- **Block**: initial releaseをblockしない

## 7.1 確定したinitial cross-session/maintenance境界

- mutationは唯一のlocal active-console interactive logonかつRDP/second logonなしに限定する。Fast User Switching/session changeは`KEEP_AUTHORIZED`前なら即Revert、authorization後はold frontend authorityをfenceしてjournal commit/recoveryだけを許す。
- machine-wide maintenance/mutation gateをtransaction全期間保持し、per-display lock、per-user/logon recovery lockの順で取得する。abandoned lockはrecovery inspection triggerで、maintenance許可ではない。
- per-user recovery WALはowner SIDだけが復元する。別user/admin/installerはrestoreせず、owner不在のpending/critical/unknown stateではmaintenanceを拒否する。
- protected machine actor recordは全session discovery/maintenance blocking用で、C0/P0/Rを含めず、restore authorityにはしない。wire `ACTIVE_INTENT`をWAL PREPAREDより先にdurable化し、owner WAL terminalと全actor/worker quiescence後に`TERMINALIZING -> TERMINAL_CLEAN`へ進める。
- elevated trusted maintenance actorはreferenced owner WALをterminal照合用にread-only openする。unreadable、record/WAL不一致、ACL/reparse/tamper疑いではupdate/repair/uninstallをfail closedにし、restore/cleanupはしない。
- 複数interactive sessionでのmutation、Windows service、owner不在recovery、all-process loss 15秒保証は初期非対応である。support拡張は別design reviewとevidenceを必要とする。

## 8. 実装開始前に人間が判断すべき事項

Phase 1A開始前:

- Q03 exact Windows 10/11 machines/editions/buildsとGPU/display
- API/DLL/allowed argument/flag/timeout/field/redactionを行単位でfreezeしたread-only call allowlist、forbidden-call audit、data redaction/retention、Operator、Evidence Owner、Reviewer、Target Machine、実施日、evidence location、immutable approver/result
- Tauri改訂設計の再レビュー結果

Phase 2A開始前（Phase 1A/G1A closure後）:

- display mutationなしのcoordination/storage/process spike専用human approval
- `KEEP_AUTHORIZED`/DecisionJournal A/B fault model、canonical machine record/WAL order、DACL、maintenance fence、commit-writer/heartbeat測定plan
- Target Machine/Operator/Evidence Owner/Reviewer/evidence locationのPhase 2A record

Phase 1B/2B mutation前（Phase 2A/G2A closure後）:

- Q01、Q02のsafety boundary仮承認
- exact transition、blind recovery、lab access、stop authority
- Q09 candidate API/flagとhard exclusion
- Phase 1A result review、TDR-001〜006 resolution再レビュー、watchdog/recovery/clock/fencing/maintenance承認
- exact Operator/Evidence Owner/Reviewer/Target Machineとout-of-band別操作経路、human mutation approval

Product foundation/mutation統合前:

- Q05 confirmation/accessibility
- Q07 watchdog launch independence
- Q10 journal/security model

Release前:

- Q01〜Q05の最終decision
- Q03 finite support statement
- Q04/Q08 installer/signing/WebView2
- all zero-tolerance evidenceとknown limitation

## 9. 技術spike routing

### Phase 1A read-only

R-T05、R-W05〜W10、Q03/Q06/Q09の観測部分を扱う。displayを変更しない。

### Phase 2A coordination/storage/process（no mutation）

R-T01/R-T03/R-T04/R-T07/R-T08/R-T12〜R-T19、Q07/Q10を扱う。Phase 1A/G1A後、Phase 1Bより前に実施し、displayを変更しない。

### Phase 1B controlled mutation

R-T02、R-W01〜W04、Q01/Q02/Q09の実API部分を扱う。別承認とphysical recovery必須。

### Phase 2B watchdog controlled recovery

R-T01〜T04、R-T13〜R-T19、Q01/Q02/Q07/Q09/Q10のreal transition統合を扱う。Phase 1B後かつ別mutation承認を要する。

### Phase 3/8 packaging

R-T10、R-D01〜D08、Q04/Q08を扱う。

### Phase 9 scale

R-W09/W10、Q11を扱う。initial releaseと分離する。

## 10. Risk受容原則

- Critical safety failureをwaiverでpassにしない。修正またはsupport cell/scope除外とする。
- 「未再現」「別machineでは成功」「retryで成功」はclosureではない。
- unsupported/unknownをbest effort mutationへ変えない。
- product targetとverified supportを区別する。
- historical reviewの解消記録は、Tauri implementationで再検証されるまでevidence済みを意味しない。

## 11. Decision log template

```text
Decision ID:
Related risk/question:
Date:
Owner / approvers:
Decision:
Alternatives considered:
Evidence IDs:
Exact support scope:
Safety impact:
Security/privacy impact:
Rollback/revisit trigger:
Documents updated:
```
