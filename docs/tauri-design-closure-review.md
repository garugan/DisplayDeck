# DisplayDeck Tauri設計 closure独立再確認

レビュー日: 2026-08-06  
レビュー種別: DD-FR-001修正の限定独立再確認  
実施範囲: 設計レビューのみ  
実施していないこと: 既存設計文書の修正、Phase 1A開始記録の入力、ソース/Cargo/Tauri projectの作成、build/test、Windows API/file API実行、display setting変更、Phase 1A/2A/1B/2Bの開始

## 1. 最終判定

**APPROVED_FOR_PHASE_1A_RECORD_COMPLETION**

DD-FR-001が問題にした初回baseline作成、session-local root、旧sessionからのrollover、mixed slot、provisioning crash、Keep target、normal/critical evidence retentionは、`docs/architecture.md` 7.4の`ProvisionCurrentDecisionBaselineV1`を中心に一つのfail-closed contractへ収束している。元のMediumが許していたfile-global chain、foreign slotのcurrent authority化、無条件truncate/recreate、baseline不明のままのmutationという実装分岐は残っていない。

新しいCritical/High/Mediumの設計gapは見つからなかった。classification名の完全な排他性には後述のLow 1件があるが、重なる候補はいずれもnew session/reclaim/mutation拒否とevidence保持へ収束するため、旧DD-FR-001の安全結果やPhase 1A record completionを再びblockするものではない。

DD-FR-002はPhase 1A blockerではなく、Phase 2A code/file/serializer/fixture/fault harness作成前のversioned freeze artifact、review、approvalを必須とするgateへ隔離されている。

この判定が許すのはPhase 1A開始記録の入力・completionへ進むことだけである。Phase 1Aの実行承認ではない。Phase 1A/2A/1B/2Bは現在も未承認・未開始である。

件数:

- Critical: 0件
- High: 0件
- Medium: 0件
- Low: 1件
- Question: 0件

## 2. 確認した文書

指定された文書はすべて存在し、自己判定を結論根拠にせず本文のnormative contractを照合した。

| 文書 | 主な確認対象 |
| --- | --- |
| `AGENTS.md` | repository gate、baseline/worker安全制約、Phase非承認 |
| `docs/requirements.md` | UC-03、FR-301/309/413/414、AC-029/030 |
| `docs/architecture.md` | 4.7、7.4、11.1、19.2〜19.6 |
| `docs/security.md` | fixed path、cross-session injection、reclaim/retention、maintenance fence |
| `docs/testing-strategy.md` | first baseline、rollover、mixed slot、crash、worker negative oracle、traceability |
| `docs/implementation-plan.md` | Phase 1A read-only境界、pre-Phase 2A DD-FR-002 freeze、G2A gate |
| `docs/risks-and-open-questions.md` | R-T20、journal lifecycle、Phase 2A実証事項 |
| `docs/tauri-design-review-checklist.md` | DD-FR closure条件とPhase gate |
| `docs/tauri-review-resolution.md` | DD-FR自己記録の所在。記録されたstatus自体は独立判定根拠に不採用 |
| `docs/tauri-design-final-review.md` | DD-FR-001/002の元問題、severity、closure条件 |

## 3. 指摘事項

### DD-CR-001 — journal classificationのfail-closed priorityはあるが、wire imageから名称への全順序がない

- **ID**: DD-CR-001
- **重要度**: Low
- **関連指摘**: DD-FR-001、DD-RRR-001
- **対象文書**: `docs/architecture.md`
- **対象セクション**: 7.4 `File classification`、`Mixed-slot decision rules`
- **問題**: tableは排他的classificationを宣言する一方、`OLD_CRITICAL_OR_BLOCKED_EVIDENCE`に`unreadable/checksum conflict`を含め、別行に`CORRUPT_OR_UNREADABLE`と`CONFLICTING_GENERATION`を置いている。priorityはこれらがnormal reclaimより強いことまでは定めるが、相互の名称選択順序までは定めていない。
- **具体的なcrash/rollover状況**: slot Aがvalid old normal terminal、slot Bがforeign identityのchecksum-invalid recordで、linked WALもblockedを示すimageは、`MIXED_SESSION_SLOTS`、`OLD_CRITICAL_OR_BLOCKED_EVIDENCE`、`CORRUPT_OR_UNREADABLE`の複数候補に読める。同一identity/same-generation different payloadが外部critical recordへlinkする場合も`OLD_CRITICAL_OR_BLOCKED_EVIDENCE`と`CONFLICTING_GENERATION`が重なり得る。
- **現在の安全結果**: いずれの読みによってもauto-reclaim、新session mutation、update/repair/uninstall、whole-file recreateは禁止され、evidenceを保持して`FAILED_CLOSED`へ収束する。unsafe Keep、foreign authority、evidence destructionという結果分岐はない。
- **推奨修正**: Phase 2A code/file/fault oracle作成前に、syntactic/file-integrity class、schema class、generation-conflict class、linked external critical class、normal/mixed classのtotal precedenceを1表で固定する。または`OLD_CRITICAL_OR_BLOCKED_EVIDENCE`を「構造・checksum・supported schemaがvalidなslotが外部critical stateへlinkする場合」に限定し、corrupt/schema/generation conflictを専用classだけへ割り当てる。
- **修正しない場合の影響**: mutation safetyは維持されるが、diagnostic code、audit summary、fault-test expected class、support routingが実装ごとに異なり得る。
- **Phase 1A record blockerか**: No
- **Phase 1A execution blockerか**: No。この指摘とは別に開始recordと人間承認が未完了のため、現在のPhase 1A実行自体は不可。
- **Phase 2A blockerか**: Yes。classification/fault oracleを作る前に上記total precedenceをfreezeする。
- **Phase 1B blockerか**: Yes。Phase 2A/G2A closureまで。
- **設計修正か実証事項か**: Lowの設計明確化。Windows実証で決める事項ではない。

## 4. DD-FR-001再確認

### 4.1 判定

**DD-FR-001: VERIFIED_RESOLVED**

DD-CR-001はclassification labelのLow精度問題であり、旧DD-FR-001が問題にしたbaseline/rollover/reclaim/crashの安全結果を分岐させないため、この判定を`PARTIALLY_RESOLVED`へ戻さない。

### 4.2 前提lockとactor fence

- lock順はmachine-wide maintenance/mutation gate → per-display mutation lock → per-user/logon recovery lock → journal-writer lockで固定され、別順序は導入されていない。
- journal-writer handle/lockを先に閉じ、user → display → machineの逆順で解放する。
- PID、creation time、signed image、role、nonce/instanceでold controller/watchdog/worker/finalizer absenceとin-flight worker 0件を証明する。
- linked operational WAL、`MachineActorRecordV1 ACTIVE_INTENT`、prior `TERMINAL_CLEAN` evidence、boot/owner/logon/display/session/leaseを検証し、missing/unknown/contradictoryなら停止する。

### 4.3 fresh fileとroot

- trusted current-owner Recovery directoryと固定名`DecisionJournalV1`だけを使う。
- durable `DECISION_JOURNAL_CREATE_INTENT`後に`CREATE_NEW`相当を使い、collisionで上書きしない。
- exact symbolic length、valid header、slot A/B全byte `0x00`の`UNINITIALIZED`、reserved/padding zero、non-sparse、no trailing/short/reparseを要求する。
- first-creation中断はexact intent/prior absence/same file identityが揃う場合だけin-place retryし、既存evidence fileをtruncate/recreateしない。
- current rootは`generation=1,previousGeneration=0(ROOT),stateVersion=1,REVERT_REQUIRED`で、session-localである。foreign generationの比較・継承は禁止される。
- first Keepは`generation=2,previousGeneration=1,stateVersion=2`で、wrap/gap/rollback/conflictは`FAILED_CLOSED`である。

### 4.4 target、publication、crash

- fresh baselineはslot A、slot BはUNINITIALIZED。baseline=AならKeep=B、baseline=BならKeep=Aである。
- classification/reclaim intent → target → complete record/zeroing/checksum → bounded write/short reject → flush → close → trusted reopen → header/A/B readback → root検証 → WAL `DECISION_BASELINE_PROVISIONED` → machine `ACTIVE_INTENT` link → WAL `PREPARED` → machine `ACTIVE_PREPARED`のdurable順がある。
- root readback前に`WATCHDOG_READY`、`APPLY_GO_ARMED`、display mutationへ進まない。WALとmachine recordの順序はarchitecture 19.4と一致する。
- 指定14 checkpointはcurrent baseline、mutation件数、recovery、retry、reclaim、failed-closed、foreign adoptionを個別に定義する。全checkpointでmutationは0件、foreign adoptionも0件である。
- result unknownはclose/reopen A/Bだけで判定し、valid current rootならlinkへ進み、root不在ならexact intentの同targetだけretryし、unreadable/conflict/intent unknownは`FAILED_CLOSED`にする。

### 4.5 rollover、mixed slot、retention

- rolloverは全lock、old actor absence、prior machine `TERMINAL_CLEAN`、old WAL exact terminal、digest一致、identity可読、unresolved recoveryなし、maintenance intentなし、old recovery binary保持、retention充足をすべて要求する。
- normal terminalはdurable `DECISION_SLOT_RECLAIM_INTENT`とbounded audit summaryを先に残し、1 slotずつdemand-driven reclaimする。old generationをcurrent chainへ継承せず、両slotを同時に失わない。
- critical/blocked/corrupt/unsupported/outcome-unknown/actor-unknown evidenceが1つでもあればnew sessionとauto-reclaimを拒否する。maintenance/update/repair/uninstallも`TERMINAL_CLEAN`とreadable terminal evidenceを証明できなければbinary/evidenceを削除できない。
- mixed slotはcurrent identity完全一致slotだけをcurrent chain候補にし、foreign generationを比較せずreclaim/retentionへ渡す。current identity slotがなければcurrent decisionを推測しない。
- active/unresolved file、unknown schema、checksum mismatch、critical evidenceのtruncate/delete/recreateを禁止し、V1 rolloverでwhole-file recreateを採用しない。
- Keep write開始からreadback完了までbaseline slotを変更せず、partial/torn/invalid Keepでもbaselineが残る。terminal Keep後もbaselineを直ちに削除しない。

## 5. DD-FR-002再確認

**DD-FR-002: VERIFIED_PRE_PHASE_2A_FREEZE_CONDITION**

- Phase 1A blockerではない。
- Phase 2Aのcode、file、serializer、fixture、fault harnessを1 byteでも作る前にversioned artifact、golden/negative vector、reviewer approvalを要求する。
- Phase 2A中に同じ`schemaVersion`の意味を変更できない。
- `DecisionJournalV1`はnumeric width/endianness/enum/digest、header/A/B offset/size/file length、checksum coverage/self-field、padding/reserved、trailing/short/sparse、create/open/share/access、filesystem eligibility、schema/minReader、serialization vectorsをfreezeする。
- `MachineActorRecordV1`はnumeric width/endianness/encoding/bounds/record length、checksum、optional/forbidden/unknown/trailing、migration/old reader/old recovery binary、state golden vectorsをfreezeする。
- workerは1 process = 1 role = 1 operation = 1 `workerInstanceId`で、same-process rotate/reuseを禁止する。同一process identity/異instanceをrejectし、PIDとcreation timeを照合し、old process exact exit後だけnew workerを作るnegative oracleがある。
- Phase 2A/G2Aでfault、storage、DACL、worker fencing evidenceとともに再確認し、不成立cellはPhase 1B No-Goである。

## 6. Phase 1A境界

Phase 1Aはread-onlyのままである。範囲はdocumented display/session/OS query、bounded JSON evidence、Rust + `windows` crate binding compile/return確認に限られる。

Phase 1Aでは`CDS_TEST`、`SDC_VALIDATE`、`ChangeDisplaySettingsExW`、`SetDisplayConfig`、temporary apply/restore、DecisionJournal/baseline/WAL/MachineActorRecordの作成、file/named-object/lock prototype、watchdog/worker/process spawn/termination、BOOT_HANDSHAKE prototypeを行わない。候補API名はapproved allowlistではなく、現在approved rowは0件である。

したがってPhase 1A開始記録の入力・completionへ進めるが、exact Target Machine、call allowlist、forbidden-call audit、redaction/retention、Operator/Evidence Owner/Reviewer、evidence location、immutable approval ID、Phase-specific human authorizationが揃うまでPhase 1Aは`NOT EXECUTABLE`である。

## 7. 最大の残余リスク

最大の残余リスクは、fixed-slot write/flush/close/reopen、power-loss相当、disk full、AV/EDR/filter/share interference、file identity/DACL、old-reader/migration、worker exit/fencingが実Windows support cellで未実証なことである。これは現時点のMedium設計gapではなく、DD-FR-002 freeze後のPhase 2A evidence gateである。証明不能なfilesystem/support cellはPhase 1B No-Goとする。

## 8. 最終まとめ

| # | 確認項目 | 結果 |
| --- | --- | --- |
| 1 | DD-FR-001 | `VERIFIED_RESOLVED` |
| 2 | DD-FR-002 | `VERIFIED_PRE_PHASE_2A_FREEZE_CONDITION` |
| 3 | Critical件数 | 0 |
| 4 | High件数 | 0 |
| 5 | Medium件数 | 0 |
| 6 | journal分類は一意か | mutation/reclaim/evidence保持の安全結果は一意。名称の完全排他性だけDD-CR-001 Low |
| 7 | fresh file初期状態は一意か | Yes |
| 8 | generation rootは一意か | Yes。`1/0(ROOT)/stateVersion 1/REVERT_REQUIRED` |
| 9 | baseline target slotは一意か | Yes。fresh=A、Keep=baseline反対slot |
| 10 | baseline publication順序は一意か | Yes。readback→WAL link→machine link→PREPARED |
| 11 | provisioning crash結果は一意か | Yes。retry可能条件または`FAILED_CLOSED`へ収束 |
| 12 | provisioning中のmutationは常に0件か | Yes |
| 13 | session rollover条件は一意か | Yes。全証明のAND gate |
| 14 | old normal terminalのreclaimは安全か | Yes。durable intent、1 slot、audit summary、other evidence保持 |
| 15 | critical/blocked evidenceを保持するか | Yes。auto-reclaim/new session/maintenanceを拒否 |
| 16 | mixed slotの結果は一意か | 安全結果はYes。diagnostic class名だけDD-CR-001 Low |
| 17 | truncate/recreate方針は一意か | Yes。V1 whole-file recreate不採用 |
| 18 | Keep中にbaselineが保持されるか | Yes。opposite slotのreadback完了まで不変 |
| 19 | DD-FR-002はPhase 2A前freezeへ隔離されているか | Yes |
| 20 | worker one-shot invariantは一意か | Yes |
| 21 | Phase 1A境界はread-onlyのままか | Yes |
| 22 | Phase 1A開始記録の入力へ進めるか | Yes。このreview後にrecord completionへ進める |
| 23 | Phase 1A自体はまだ開始しないこと | Confirmed。record/承認未完で`NOT EXECUTABLE` |
| 24 | Phase 2A/1B/2Bへ進めないこと | Confirmed。すべて未承認・未開始 |
| 25 | 最大の残余リスク | Windows storage/filter/power/DACL/process fencingの未実証。Phase 2A/G2A No-Go gate |
| 26 | 最終判定と理由 | `APPROVED_FOR_PHASE_1A_RECORD_COMPLETION`。Critical/High/Mediumの設計gapがなく、DD-FR-001は安全結果まで一意、DD-FR-002は後続freezeへ隔離済み |
