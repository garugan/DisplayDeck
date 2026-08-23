# DisplayDeck テスト戦略

最終更新: 2026-08-24

状態: exploratory read-only CLIのStep 1〜8はWindows実機観測済み。Gate Aは2026-08-24に承認され、Stage 1のnon-mutating実装、自動test、Windows packaged read-only smokeは完了した。実行必須範囲は`docs/implementation-plan.md` 5章の最小検証policyを正本とする。本書の広いmatrixと旧Phase名は将来support cellを追加するときの参考catalogであり、MVP開始条件や全件実行要求ではない。Gate B前のWindows変更は未承認。

## 1. 最優先目標

テストの第一目的は「modeを変更できる」ことではなく、失敗、black screen、WebView/Tauri crash、worker hang、stale recovery、process競合の下でも、別targetを変更せずC0/P0へ安全に戻るか、戻せないことを正しくblocked/failedとして残すことである。

成功runで過去の安全failureを相殺しない。rollback failure、worker exit未確認、別target変更、unknown journalからのmutation、rollback SLA違反はzero-tolerance release blockerとする。

## 2. テスト階層

| Layer | 主対象 | 実行場所 | Display mutation |
| --- | --- | --- | --- |
| React/TypeScript unit/component | draft、candidate UI、confirmation、error | CI/browser | なし |
| Tauri API wrapper contract | invoke/event DTO、mock | CI | なし |
| Rust domain/unit/property | validation、state、session、mapping、error | CI Windows runner中心 | なし |
| Tauri command/security | command allowlist、Capability、origin、serialization | packaged test/CI | なし |
| Recovery storage/watchdog simulation | WAL、clock、fault、protocol、fencing | CI Windows runner | fake workerのみ |
| Read-only Windows API integration | GDI/CCD列挙、identity、権限 | physical Windows | なし |
| Controlled mutation qualification | apply/readback/rollback/crash | dedicated physical lab | あり、別承認 |
| Packaged NSIS/MSI/WebView2 | sidecar、署名、install/repair/uninstall | physical/clean Windows | release scenarioのみ |

Tauriはmock runtimeによるunit/integration testとWebDriver系E2Eを提供するが、mock WebView/CIはphysical display/driver behaviorを証明しない。[Tauri Tests](https://v2.tauri.app/develop/tests/)、[Tauri WebDriver](https://v2.tauri.app/develop/tests/webdriver/)

## 3. React/TypeScript

### 3.1 State/draft

- snapshotからcurrent/planned viewを構築する。
- slider/selectがcandidate indexだけを選び、OS数値を合成しない。
- resolution変更時にexact refresh保持、最小差、同差なら低い候補になる。
- current-not-listed、candidate 0/1/多件、duplicate label、59.94/60を扱う。
- resetはdraftだけを戻し、mutation APIを呼ばない。
- stale snapshot/event generationでstatus再同期する。

### 3.2 Component

- current/planned/diffがtextとsemanticsで区別される。
- candidate 2〜8でslider+select、9件以上/zoomでselect主操作になる。
- scale rowがread-onlyでfocusable change controlを持たない。
- Loading/Validating/PreparingRollback/Applying/Restoring中のcontrol disable。authorization前ConfirmingはKeepをdisableしRevertを有効、`ConfirmCommitInProgress`は両方をdisableして「確定処理中」を表示する。
- no change/multi-path/remote/unsupportedでApply disabled。
- exact/degraded/failed/blocked error文言とseverity。

### 3.3 Confirmation

- stage 1はRevert enabled/focused、Keep disabled。
- matching generationのstage 2だけKeepをenableする。
- authoritative statusの`remainingMs`だけを`performance.now()`相当で短時間表示補間し、Reactがabsolute native deadlineを受け取ったり再計算したりしない。event停止やbrowser clock driftで延長せず、正値をKeep可否、0表示をterminal Revertのoracleにしない。
- Escape/closeはrestore API、stale Enter/Keepは送らない。
- presentation ack timeout、focus failure、window hiddenをsimulationする。
- `ack_display_change_presentation`のStage 1/2、rotated token/generation、same-payload duplicate、duplicate mismatch、old view/reload、late ACKをsimulationする。
- initial root mount、root remount、frontend boot再開始、reload、navigation、renderer crash/recreate、controller変更、presentation reconstructionの`BOOT_HANDSHAKE`でcore-issued `viewRevision`がrotateし、child remount/focus/minimize/`ORDINARY_RESYNC`ではrotateしない。`StatusRequestV1` version/mode、same-boot-nonce duplicate、128-bit以上のCSPRNG/token alphabet/length、raw-log非出力も検査する。
- Stage 1 ACK後reloadしたnew viewからのStage 2-only ACK、old renderer/XSSからのlate ACK、old viewとnew presentation tokenの組合せを拒否し、Revertへ収束する。
- active presentation中のroot remountはold stage bindingを失効し、deadlineを延長せずStage 1から再構築する。`PRESENTATION_RESYNC`でold Stage 2 authorityを移送できないことを検査する。
- boot-handshake matrixは初回mount、root remount、child remount、focus resync、reload、navigation、renderer crash、same/different nonceのrepeated BOOT_HANDSHAKE、old viewからのStage 2 ACK、active presentation中root remountを個別caseにする。
- Rust statusがRestoring/terminalになればoverlay local stateを破棄する。
- sleep/hibernate resumeではstatus取得前にKeepを再enableせず、expiredならRestoringへ収束する。

### 3.4 Accessibility/visual

- keyboard-only、focus trap、accessible name/value text、live region量。
- 200% zoom、high contrast、text scale、reduced motion。
- Narrator実機testはWindows gateで行い、CI DOM assertionだけで合格にしない。
- Enter=Keep policyはDDR-Q05決定に応じtestを固定する。決定前はsafe baselineのfocused-button behaviorをtest designとする。

## 4. Tauri API wrapperとmock

### 4.1 `tauriApi.ts` contract

- 6 command以外のinvokeをfeature codeから呼べず、presentation ACKをevent/generic invoke/Confirmで代用できない。
- `get_display_change_status`の`StatusRequestV1`だけでBOOT/ORDINARY/PRESENTATION modeを表現し、7番目のcommandやunversioned handshakeを作らない。
- DTO decodeでunknown state/version/errorをfail closed表示する。
- command reject、event gap、late event、duplicate eventをstatus callで収束させる。
- raw journal/path/native errorがfrontend typeに存在しない。

### 4.2 Deterministic mock scenarios

- normal snapshot、59.94/60、candidate多数、current-not-listed
- stale/hotplug/multi-path/remote/virtual/scale unknown
- watchdog start failure、journal failure、preflight reject、apply mismatch
- timeout/explicit revert/Tauri loss/worker hang
- exact/degraded/failed/blocked/unknown journal
- old session confirm、double begin、double restore、late Keep

mockはfrontend development用で、Windows以外のproduct backendではない。production buildへmock selection switchが残らないことを検査する。

## 5. Rust domain・command

### 5.1 入力validation

- missing/unknown field、wrong type、overlength、invalid alphabet、integer overflow、zero denominator。
- stale snapshot、expired token、wrong monitor/mode membership、support fingerprint mismatch。
- raw width/Hz/path/flagを送ろうとしてもDTOへ入らない。
- remote/virtual/multi-path/classification unknownでmutation request 0件。
- native outputのinvalid count/index/size/enum/UTF-16を拒否する。

### 5.2 State transition/session

- 全valid transitionと全invalid edge。
- Applying中再begin、Restoring中confirm、terminal reapplyを拒否。
- sessionId collision simulation、old generation、different session、deadline race。
- Confirming中もrestoreをwatchdogへforwardし、`KEEP_AUTHORIZED`前のdecision-lock race、authorization後のordinary Revert拒否、DecisionJournal commit後拒否、confirm/restoreの冪等性、同一sessionのterminal一意性を検証する。
- event late/out-of-order時のprojection。

### 5.3 Candidate mapping

- GDI complete tupleのdigest stability。
- GDI integer refreshとCCD rationalのexact match/mismatch。
- exactly 0/1/2 expected observationで`canApply`がfalse/true/false。
- qualification evidenceとexact support fingerprint binding。
- preferred超、current-not-listed、color/orientation difference分類。

### 5.4 Error conversion

- Win32/Tauri/IO/protocol errorをfixed code/severity/retryへ変換。
- rollback errorがapply errorより優先される。
- raw path/device/nonce/stackがserialized error/logへ漏れない。

### 5.5 Unsafe boundary

- safe wrapper単位でvalid/invalid structureをtestする。
- count/size arithmetic、null/unterminated UTF-16、unknown union type、invalid indexをproperty/fuzz testする。
- handle ownership/close/inheritanceをWindows integrationで検査する。
- unsafe block coverageをinventory化し、未test blockをrelease blockerにする。

## 6. Recovery storage

- fresh dual slot、片slot torn write、両slot invalid、bad digest、unknown major/minReader。
- generation wrap/rollback、same generation conflict、invalid state transition。
- required-field mask不足、prohibited raw/path field、range overflow。
- flush/reopen failure、disk full、access denied、AV sharing violation。
- PREPARED/APPLY_INTENT/APPLIED_VERIFIED/PRESENTING_STAGE1/STAGE2/AWAITING/Confirming/REVERT_DECIDED/restore intent/全terminalのstartup table。
- C0!=P0、persisted drift、terminal observation drift、topology change。
- preferences file破損がrecovery journalへ影響しない。
- DACL/reparse/hardlink/final path verification。exact Windows API方法はPhase 2Aで確定する。

fault injectionは各durable write、flush、close、reopen、worker spawn、identity write、GO、call return、terminal frame、process signaled、readback writeの間でprocessを停止する。

### 6.1 `DecisionJournalV1`

- `ProvisionCurrentDecisionBaselineV1` first-baseline matrix: file absent、fresh exact-length file、A/B両方UNINITIALIZED、slot Aへ`generation=1,previousGeneration=0(ROOT),stateVersion=1,REVERT_REQUIRED`、slot BはUNINITIALIZED。file-global generationを継承しないことを検査する。
- fresh file fault: create-intent前/後、`CREATE_NEW`前/成功直後、actual volume/file ID取得前/後、post-create durable checkpoint前/partial/flush/reopen後、full-zero write前/partial/full、header partial/full、sparse/trailing/short file、fresh readback前/失敗、slot A書込み前/partial/full-flush前、flush error/成功後crash、close後-reopen前、WAL `DECISION_BASELINE_PROVISIONED` link前、machine `ACTIVE_INTENT` link前、`PREPARED`後-apply GO前を個別injectする。全caseでmutation call 0件とforeign evidence採用0件をoracleにする。
- baseline result unknownはclose/reopen A/Bだけでvalid current rootの有無を決め、validなら追加baseline writeなし、invalid/absentならactual file IDをbindしたexact post-create checkpointまたはreclaim intentの同targetだけretryする。post-create checkpoint前のexisting file、unreadable/conflictは`FAILED_CLOSED`にする。
- rollover normal terminal: old `REVERT_REQUIRED` + normal terminal WAL、old `KEPT_SESSION`、old A/B両terminal、old baseline+Keep、old boot/display/owner/logon/lease、reclaim intent後crash、new baseline partial後crash、old evidence保持、normal terminal one-slot reclaim/audit summaryを検査する。
- rollover block: unsupported schema、checksum mismatch、same-generation conflict、old `FAILED_CLOSED`、`RECOVERY_REQUIRED`、`RESTORING`、outcome unknown、old actor alive/判定不能、old MachineActorRecord nonterminal/unreadable/mismatch、old WAL unreadable/nonterminal、owner/boot/display unknownでreclaim/new mutation 0件とcritical evidence保持を検査する。
- mixed-slot matrix: old session+current baseline、old session+current partial、current baseline+current Keep partial、different boot A/B、different display A/B、different owner/logon/lease A/B、same identity/same generation conflict、current identity slotなし、both invalid、current baseline only、current Keep+current baselineを検査する。foreign generationをcurrent max比較に入れない。
- truncate/recreate oracle: active/unresolved、unknown schema、checksum mismatch、missing expected file、old critical evidenceでwhole-file truncate/delete/recreate 0件。first fileのin-place completionはactual file IDをbindしたvalid post-create checkpointがあるcaseだけに限定する。
- target invariant: fresh baseline=A/Keep=B、baseline=B/Keep=A、Keep writeからreadbackまでbaseline full-slot digest不変、partial/torn Keep後もbaseline valid、terminal Keep後もimmediate cleanup 0件を検査する。
- slot A正常/slot B正常、Aのみ破損、Bのみ破損、両方破損。
- partial slot、torn slot、checksum mismatch、invalid magic/schema/recordLength/slotIndex。
- generation rollback/wrap/gap、`previousGeneration` mismatch、impossible `stateVersion`、同一generation内容競合。
- `REVERT_REQUIRED` provisionがwrite/flush/readbackできなければapply GO 0件。
- fixed offset `WriteFile` short write、`FlushFileBuffers` error、close/reopen/readback error。
- full write後flush前crash、flush成功後crash、readback前crash、readback後response前crash。
- deadline直前`KEEP_AUTHORIZED`、deadline exact/後のauthorization拒否、authorization後I/O遅延/flush stall。
- `KEEP_AUTHORIZED`後actor lossでnew slotなし、partial slot、full valid slotの3結果をRevert/Revert/Keepへ分類する。
- duplicate Confirm、terminal Keep後Revert、writer alive-but-hung、writer exit後replacement、lease/instance fencing、concurrent writer 0件。
- antivirus/EDR/filter driver/share interference、disk full、access denied、power-loss相当simulation fault injection。simulation結果を実電源断evidenceと扱わず、storage/controller/filterを含むphysical power/reboot testはsupport cell別の別evidenceとする。
- outcome unknownでpossible Keepを上書きせず、close/reopen後のA/Bからnew valid Keep/no new generation/unreadable-conflictをKeep/Revert/FAILED_CLOSEDへ分類する。

### 6.2 DD-FR-002 freeze-candidate vector specifications

次は方針承認済みのcandidate vector specificationである。**履歴（2026-08-13）**では、vector ID、semantic input、canonicalization、expected classification、side-effect oracleを文書で固定しただけで、full byte列、expected SHA-256、artifact hashは未作成かつfixture作成・実行は未許可だった。**現況（CANDIDATE-04 artifact generated / static review clean）**では、active profileのbounded full-byte fixture、expected SHA-256、semantic manifest、artifact index、aggregate hashの生成・検証だけを許可し、その590 vector artifactについて独立static reviewはCLEANである。`DD-FR-002-D04-C04-RESOLUTION-PACKAGE-01`は2026-08-13に一括承認され、statusは`FULL_BYTES_GENERATED / SHA256_COMPUTED / FULL_INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING`である。これはPhase 2A product/runtime implementation、Tauri/watchdog/worker integration、runtime serializer/WAL、fault harness、display mutationを許可しない。

D04 fixtureの正本はcanonical JSON `D04TUPV1` tuple witnessである。各正負artifactは固定key順の`tupleProjection`実体とそのSHA-256を持ち、top-level `recordState / operationKind / operationNonce / P / Q / O`およびstate別`E / terminalPair / initialProvision`とのbyte-exact一致をoracleで検査する。リンクされた`MARV1`はbinary envelope/state layout sampleだけで、`machineActorLayoutOnly=true`を要求する。generic MARのP/Q/nonce/ownerをD04 tuple-specific sourceとして照合または主張してはならない。

最初のfull-byte reviewで`CANDIDATE-02`のworker境界混在とMAP checksum vector欠落が判明したため、同IDを上書きせず`CANDIDATE-03`を作成した。CANDIDATE-03はGO replayとterminal-after-frame、MAP semantic rejectとchecksum rejectを分け、D02 inner evidenceとD03 SID cross-linkを追加し、77 vectorのbytes/hash/index自己整合を示した。しかし独立reviewはD02 canonical subject/observation source coverage、D03/SID binding coverage、MAP resume/cleanup scenario、DJ/MAR negative/cross-link coverageの不足を確認したため、CANDIDATE-03はfreeze不可である。CANDIDATE-04はこれらを別IDで拡張した。candidate spec labelは`DJV1-VECTORS-V1-CANDIDATE-04-SPEC`、`MARV1-VECTORS-V1-CANDIDATE-04-SPEC`、`MAPRV1-VECTORS-V1-CANDIDATE-04-SPEC`、`WORKER-ONESHOT-ORACLE-V1-CANDIDATE-04-SPEC`とし、exact 590-vector catalogは生成器が出力したmanifest/indexを候補正本とする。各statusは`FULL_BYTES_GENERATED / SHA256_COMPUTED / FULL_INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING`、worker oracleはさらに`CODE_NOT_CREATED`とする。semantic manifestだけが次の13 fieldをこの順で持ち、`expectedSideEffects.displayMutation=0`を例外なく要求する。

```text
vectorId
family
positiveOrNegative
requiredDecisionSet
semanticInputProfile
canonicalizationRule
mutationDescriptor
expectedParse
expectedClassification
expectedRecoveryDisposition
expectedSideEffects
byteFixtureStatus
sha256Status
```

`mutationDescriptor`は元semantic inputから一つのnegative変化を記述する診断fieldで、実fileやfixture bytesを変更する命令ではない。`expectedSideEffects`は少なくとも`displayMutation,processLaunch,fileCreate,fileDelete,fileTruncate,fileWrite`のbounded countを持ち、文書candidateでは安全側のexpected valueだけを記録する。fixture bytes/hashは現行の限定authorization範囲でのみ生成・検証し、生成中または未生成をpass扱いせず、生成完了だけを`FROZEN`/Phase 2A authorizationと読み替えない。

semantic manifestとは別に、freeze artifact inventoryは`DD-FR-002-ARTIFACT-INDEX-V1-CANDIDATE-01`とする。全vectorの13-field objectを`vectorId` bytewise ascendingで1行1 canonical JSON + LF（最終LFあり）へまとめた単一`semantic-manifest.jsonl`をsemantic正本にする。top-level key順は`indexSchema,profileId,semanticManifestSha256,entries`、entry key順は`vectorId,relativePath,byteLength,fixtureSha256,linkedVectorIds,linkedFixtureSha256s`である。`entries`は`vectorId` bytewise ascending、link IDもbytewise ascending distinctとし、`linkedFixtureSha256s`は同じindexのlink先`fixtureSha256`と順序・件数までexact一致させる。`relativePath`はartifact rootからの`/`区切りrelative pathだけ、`byteLength`は16-char lowercase hex u64、全SHA-256は64-char lowercase hexとする。index外のapproval recordは`semantic-manifest.jsonl` full bytesの`semanticManifestSha256`、canonical index full bytesの`artifactIndexSha256`、およびarchitecture 19.7のdomain-separated ordered-entry preimageによる`aggregateFixtureSetSha256`を同じ`profileId`へbindする。artifact indexは13-field semantic manifestの代替ではなく、index自身/sidecar hash/aggregate hashを自らのhash preimageへ含めない。

- `DJV1` familyはfresh header + zero A/B、baseline A / Keep B、baseline B / Keep A、captured tick 0をpositive specにし、partial/torn/short/trailing/reserved、unknown decision、wrong physical slot index、checksum、generation/previous mismatch、cross-session/boot/display/owner/lease identityをnegative specにする。
- `MARV1` familyは13 stateのminimal required record、stateごとのcomplete optional group、3つの`FAILED_CLOSED` subvariant、provisioned-clean profileをpositive specにし、forbidden/partial group、JSON `null`/number/escape/order/duplicate key、unknown state/WAL/result/detail/evidence kind、zero/old lease、A bytes copied to B、payload/checksum/trailing、旧direct fresh sentinelをnegative specにする。
- `MAPRV1` familyは`MAPRV1-P-000`のfresh header + zero A/Bと、`CREATE_INTENT -> POST_CREATE_CHECKPOINT -> MACHINE_INTENT_PUBLISHED -> MACHINE_ACTIVE_PUBLISHED -> MACHINE_CLEAN_OBSERVED -> TERMINAL_RETAINED`のalternating slot chainをpositive specにする。CREATE_NEW直後、checkpoint前後、same/different file identity、partial header/slot、DACL/anchor/attribute mismatch、unknown state、premature resume/delete/recreate、`FAILED_CLOSED`からのcleanupをnegative specにする。
- worker oracle familyはsame process identity/different instance、same instance/different role/operation/nonce、PID reuse with different creation time、old process not signaled、old lease、GO replay、terminal後frameを全てrejectし、process launch/display mutation/file writeのexpected countを各boundaryで固定する。

minimum semantic vector ID catalogは次とする。range表記は各memberが独立manifest/vector IDを持つ意味で、1つの集約fixtureを意味しない。

| Vector ID / range | Kind | Semantic profile | Expected high-level classification |
| --- | --- | --- | --- |
| `DJV1-P-001` | positive | header + zero A/B + exact durable/reopened post-create checkpoint link | `FRESH_UNINITIALIZED` |
| `DJV1-P-002` / `003` | positive | baseline A/Keep B、baseline B/Keep A | exact current chain |
| `DJV1-P-004` | positive | captured tick=0 boundary | valid captured value |
| `DJV1-N-*` | negative | envelope/length、checksum、slotIndex/offset、generation chain、foreign identity、create-checkpoint violationとCANDIDATE-03 reviewで不足したnegative/cross-link coverage | reject / fail-closed classification |
| `MARV1-P-STATE-*` | positive | 13 stateのminimal required payload | exact state parse |
| `MARV1-P-FC-*` | positive | 3 FAILED_CLOSED subvariant | exact critical classification、no live authority |
| `MARV1-P-PROVISIONED-CLEAN` | positive | D06 owner-bound ordinary clean | clean inspection only |
| `MARV1-N-GROUP-*` | negative | forbidden key、partial optional、partial E | reject |
| `MARV1-N-JSON-*` | negative | null/number/escape/whitespace/order/duplicate/unknown/trailing key | reject |
| `MARV1-N-ENUM-*` | negative | unknown record/WAL/result/detail/evidence kind | reject without coercion |
| `MARV1-N-FENCE-*` | negative | zero/old lease、actor mismatch、slot copy | reject / no authority |
| `MARV1-N-PROVISION-*` | negative | old direct sentinel、hybrid sentinel、wrong completion variant、zero required digest | reject |
| `MAPRV1-P-000` | positive | fresh header + zero A/B | exact `FRESH_UNINITIALIZED` classification only |
| `MAPRV1-P-STATE-01..06` | positive | 6-state alternating bootstrap chain | exact checkpoint/terminal classification |
| `MAPRV1-N-*` | negative | pre-checkpoint crash、identity/DACL/anchor/attribute mismatch、unknown state、resume scenario、premature resume/delete/recreate、`FAILED_CLOSED` cleanup、checksum mismatch | fail closed、target write/delete 0 |
| `BOOTIDV1-P-001` | positive | precomputed canonical raw input tuple → expected raw 32-byte digest | exact domain-separated digest only |
| `OWNERSIDV1-P-*` / `OWNERSIDV1-N-*` | positive / negative | actual canonical SID bytes → owner SID digest → DJ/MAR/MAP/lock-name cross-link、およびboundary/invalid representation | exact domain-separated digest/cross-link、またはreject |
| `WOSV1-N-001..009` | negative | instance/role/operation、PID reuse、old process、old lease、GO replay、terminal-after-frameを各々single mutationとして分離 | reject、display mutation 0 |

各range memberのexact ID、`semanticInputProfile`、single mutation、expected counterはCANDIDATE-04 generatorのmanifest行で確定する。上表の`*`は候補familyを示すだけで、rangeを1 fixtureへ合成してはならない。現行の限定authorizationは、各memberを個別artifactとして生成・hash・index検証することだけを許可し、runtime/fault fixtureの生成・実行、product code、display mutationを許可しない。

header + zero A/Bだけでmatching `DECISION_JOURNAL_POST_CREATE_CHECKPOINT`が無いDJV1 fileはpositive fresh vectorにせず、`DJV1-N-006`のunbound zero-file profileとしてrejectする。MARV1のQ-without-P、P/Q wrong tagged variant、nested `schema/kind/operationNonce/result` width/case/literal mismatch、P/Q/top-level P groupの`schema/kind/operationNonce`不一致も`MARV1-N-GROUP/JSON` rangeへ必須で含める。
- `OwnerWalLinkStateV1`は`0x0000..0x0014`の定義済みlink/nonterminal code、`0x0020..0x0028`のterminal codeを全件round-tripし、`0x0015..0x001f`、`0x0029..0xffff`、その他table外codeをrejectする。`ABSENT_EXPECTED`はmachine link sentinelだけで、owner WAL frameへencodeできないことを検査する。
- in-memory `KEEP_AUTHORIZED`、MachineActorの13 wire state、DecisionJournal decision、UI projection、heartbeat observation、file classification、`JOURNAL_CORRUPT_OR_UNKNOWN`を`ownerWalState`へencodeできないことをcompile/domain testとnegative bytesで検査する。同名`KEPT_SESSION`も別schemaのnumeric codeを共有せず、digest linkだけを許す。
- generic `RESTORE_INTENT`をrejectし、`RESTORE_CURRENT_INTENT` / `RESTORE_RESULT` / `RESTORE_PERSISTED_INTENT`の順序、C0 failure detail、P0 fallback conditionを別vectorにする。preflight/apply error detailを新しいstateへ暗黙昇格させない。
- canonical JSONはmaster key order、ASCII subset、whitespace/escape/duplicate keyなし、全scalar stringをpositive vectorにする。JSON number、boolean、`null`、array、unknown/trailing key、大小文字違いhex、width違い、optional groupの部分出現をrejectする。
- `SidValueV1`はpresent SIDのlength 8/68 boundary、Windows SID validation、unused tail zeroを検査する。`kind=0000 ABSENT_EXPECTED`の型decodeはpresentと混同しないことを確認するが、D06採用後のvalid MachineActor stateでは出現をrejectする。present+length 0、absent+nonzero length/bytes、invalid SID、nonzero tail、string SID/account nameもrejectする。
- process/actor objectはPIDだけで同一視せず、creation time、signed image identity、role、process nonce、logical instance IDを個別に変えたnegative vectorを持つ。finalizing watchdogはcandidate上`WATCHDOG` roleのままで、未定義`FINALIZER` roleを受理しない。
- `OperationResultV1`は`NONE`をvalid MachineActor V1でrejectし、reserved/unknown resultをfail closedにする。全operation-kind tagged completionでcommon prefix=`schema,kind,operationNonce,result,actor`を必須とし、Qの`schema/kind/operationNonce`をPとtop-level P groupへexact bindする。wrong variant/key、zero required digest、actor mismatchをrejectする。result単独でKeep、restore、maintenance成功、startup authorityを決めず、operation kind / owner WAL state / terminal digest / MachineActor stateのtuple不一致をrejectする。
- `LINKED_OWNER_NONTERMINAL_UNCERTAINTY`はH/O/P/Q/E complete、T forbidden、Q=`OUTCOME_UNKNOWN`、Oのlast valid nonterminal generation exact一致だけを許す。missing Q、terminal T混入、success result、unreadable WALからのowner state推測をrejectする。
- D06はnonzero epoch/lease/bootの`MAINTENANCE_INTENT(INITIAL_PROVISION) -> MAINTENANCE_ACTIVE`から、designated ownerとMAINTENANCE actor/tagged completionをbindしたordinary clean recordだけを許す。complete O/T typed-absence profileは許可するが、旧direct案のzero epoch/lease、zero bootId、absent SID、NONE P/Q、またはその一部を混ぜたhybrid、generic empty string、欠落required keyをrejectする。
- `UNKNOWN` / `FAILED_CLOSED`はunknown numeric enumのcoercion先にしない。nonzero bounded `lastErrorClass + lastErrorDetailCode + preservedEvidenceDigest`を必須にし、detail/class/evidence-kind mismatch、partial E、zero inner/outer digest、unknown detail、ordinary operation failureのcritical昇格をrejectする。
- D03 `ownerSidDigest`はdomain separator、actual SID lengthのu32 little-endian、actual binary SIDだけをhashし、fixed-capacity tail/JSON/string SIDがdigestを変えないpositive/negative vectorを持つ。MachineActor owner SID、DecisionJournal digest、lock-name digestの三者をexact cross-linkする。
- D06 provision matrixはseparate bootstrap recordの`CREATE_INTENT`前後、MachineActor `CREATE_NEW`直後、actual volume/file ID取得、`POST_CREATE_CHECKPOINT` partial/flush/reopen、same/different file ID、full-zero/header、最初のvalid `MAINTENANCE_INTENT`、`MAINTENANCE_ACTIVE`、owner-bound ordinary clean publicationを個別に停止する。checkpoint前のexisting fileはresume/delete/recreate 0件、checkpoint後はsame identityだけin-place resumeを許す。provisioned cleanのdisplay/logon/session/WAL/terminal typed absenceと、初回runtime `ACTIVE_INTENT`のactual値を混同しない。
- D05/D07はSYSTEM creator、single designated runtime SID、object別ACE mask/order、protected/no-inheritance、別SID/broad principal/forbidden rights、file ID/DACL/attribute/anchor readbackをmanifestへ持つ。directory anchor algorithmまたはWindows実証が未確定のcellはexpected classification=`DIRECTORY_ANCHOR_UNPROVEN`、file create/write/delete/display mutation全て0件とする。
- D08 static wire-freeze laneは、precomputed `BootIdV1` raw input tuple（canonical UTC FILETIME、parsed version/build）とそのexpected raw 32-byte digestだけを扱う。fixtureはWMI query、provider timeout/retry、`GetTickCount64`、precise UTC sample、tolerance値を含めず、時変tick/UTC sampleがhash preimageへ入らないことだけを検査する。WMI boot UTC/version/buildを別actorがreadすること、reboot/build mismatch、WMI strict-parse mismatch、tick/UTC cross-check failure、sample span/tolerance boundary、clock-jump/provider errorは`D08-WINDOWS-EVIDENCE-LANE-V1`の`EVIDENCE_PENDING`項目であり、static wire vectorとして作成・pass・freeze扱いにしない。
- D08の文書候補`DD-FR-002-D08-TOLERANCE-CANDIDATE-01`は`maxBootIdentitySampleSpanMs=250`、`maxBootUtcDelta100ns=500000`（50 ms）、clock-jump rule=`REJECT_NON_MONOTONIC_OR_PREDICTED_BOOT_DELTA_EXCEEDS_LIMIT`である。local captureは`t1 >= t0`、`u1 > u0`、sample span上限、`abs((u1-t1*10000)-(u0-t0*10000))`上限をchecked arithmeticで検査し、一つでも満たさなければ`BOOT_IDENTITY_UNPROVEN`とする。これはWindows 10 `10.0.19045`一台の25件から作ったlab candidateであり、static fixture、runtime template、acceptance resultを変更しない。
- CANDIDATE-01の昇格前に、sample spanとUTC deltaの`limit-1 / limit / limit+1`、tick/UTC逆行、checked-arithmetic境界、WMI timeout/provider error/strict-parse mismatch、Version/Build mismatch、clock change、対象Windows/CPU/負荷/security-product cellを独立evidenceで確認する。高速スタートアップ有効cellは未観測なので、そのcellをsupportへ含める場合だけ別実機evidenceを要求する。未実施または不成立のcellではsame-boot authorityとdisplay mutationを0件にする。

## 7. Watchdog/worker

### 7.1 Protocol

- handshake/version/session/epoch/owner/sequence/nonceの一致・不一致。
- partial length/payload、oversize、deep JSON、unknown field/type、sequence gap、trailing bytes。
- stdout/stderr同時flood、retention/total budget、discard drain。
- terminal前EOF、terminal後extra data、exit code mismatch、terminal-to-exit timeout。
- one-use GO replay、GO前operation、別role requestを拒否。
- workerはone-shot processで、同一`workerProcessIdentity={PID,creation time,image,role,processNonce}`のまま`workerInstanceId`をrotateしたrecord/frameを拒否する。worker reuse 0件、old worker process object signaled後だけnew worker process/new instanceを作ること、PID再利用をcreation timeで別processとして検出することをnegative testにする。

### 7.2 Deadline/decision

- shortened logical clockとproduction 15秒実測を分ける。
- Win32 `GetTickCount64` adapterをproduction contractとし、created/ready/t0/ACK/deadline/`keepAuthorizedTickMs`/decision write tickを区別する。wall timeはdiagnosticとboot/stale矛盾検出補助だけで、live acceptanceへ使わない。
- final readback完了後、decision lock取得前/identity検証中/Revert先行/tick直前/`tick == deadline`/`tick > deadline`/in-memory transition直後を個別injectし、`KEEP_AUTHORIZED`だけがlinearization pointであることを検査する。
- Keep just before/at/after deadline、EOF vs Keep、timeout vs manual Revertを同じdecision lockで競合させる。deadline前lock取得だけ、request到着、React remaining正値ではauthorizationしない。
- `KEEP_AUTHORIZED`後にdeadlineを進め、slot write/flush/readbackを遅延させてもordinary timeout/Revertが起動せず、valid terminal readbackならKeepになることを検査する。
- acceptedとcommittedを別oracleにし、accepted/`ConfirmCommitInProgress`ではReact success 0件、valid `KEPT_SESSION` readback後だけsuccess 1件とする。
- `MoveFileExW`/`ReplaceFileW`をdeadline CASとしてmock/実装契約に使わず、`FlushFileBuffers`/`FILE_FLAG_WRITE_THROUGH`単独をatomicity/CAS oracleにしない静的検査を行う。
- 30秒pre-apply lease、60秒maximum lifetime超過で未mutation abort/未authorized mutation restoreになる。`KEEP_AUTHORIZED`後はmaximum tickでdecisionを逆転せずwriter-hang testへ移る。
- wall-clockを前後変更してもdeadlineが変わらず、15秒超suspend/hibernate後の最初のloopでKeep拒否/restoreになる。sleep中のcall-startは保証外としてresume-to-call-startを別計測する。
- bootId same-boot、reboot、Fast Startup、clock change、WMI failure/cross-check mismatch、watchdog restartで誤継承しない。
- presentation ack未完、stage/generation mismatchで即時rollback。`KEEP_AUTHORIZED`後のpresentation/event failureはlive arbitrationを逆転させずjournal outcomeへ従う。

### 7.3 Parent/worker failure

- WebView crash、Tauri core graceful exit、panic、forced kill、Task Manager End task、control pipe切断。
- worker crash/hang、termination request、process signaled、late result。
- `HeartbeatPolicyV1` candidate（250ms emission/4 misses/250ms graceful stop/250ms launch targetを含む）を固定製品値にせず、heartbeat jitter/latency distribution、p50/p95/p99/max、false-positive count/rate、CPU高負荷、sleep/resume、debugger pause、AV・EDR scan/interference、disk stall、process-handle wait、termination API latency/result/access、replacement launchをPhase 2Aで測る。
- product値への昇格は、approved finite cell/scenarioとsample countが揃い、unproven takeover/double actorが0件、false-positiveとworst-case latencyの受容基準をSafety/Windows/Product ownerがimmutable evidence ID付きで承認した場合だけとする。未達/未承認bundleではmutation disabledをtestする。
- heartbeat miss/suspect、authoritative process exit、alive-but-hung、IPC stall、resume、CPU/scheduler starvation、security-product delay、termination access denied/timeoutを別々に注入する。exit未証明でreplacementを起動せず、exact identity termination、lock reacquisition、leaseVersion increment、confirmationを再開しないlive takeoverを確認する。
- watchdog crashをPREPARED/APPLY_INTENT/PRESENTING/AWAITING/`KEEP_AUTHORIZED`直後/decision slot write中/flush後/readback後/REVERT/restore intentへinjectし、DecisionJournal readbackからabort/restore/Keep/blockedが表どおりになる。
- old watchdogがlate frameを送る/再び動く、old workerが生存/query denied、replacementが失敗する場合にfresh operation 0件となる。
- Tauri coreとwatchdogを同時に失うall-process lossは15秒保証外として別testし、blind recovery procedureを検証する。

### 7.4 Fencing/multiple launch

- simultaneous Tauri instances、two watchdogs、abandoned mutex。
- old worker PID reuse、creation time mismatch、image mismatch、OpenProcess/query denied。
- old worker exit未確認でfresh operation 0件。
- stale actorからのKeep/reapply/restore frame破棄。
- old session journalとnew sessionの混同なし。
- old generation/lease/boot/controller/watchdog/presentation token/command nonce、別owner SID/logon、wrong displayIdをoperationごとに拒否。
- Fast User Switching、second interactive user、RDP sessionの開始/切替でnew mutation 0件、未authorized live transactionはRevert、`KEEP_AUTHORIZED`後はold frontend command 0件かつjournal outcomeへ収束する。

### 7.5 Machine record / maintenance durability

- separate installer provision recordのcreate intent、MachineActor file `CREATE_NEW`、actual file ID/DACL/owner post-create checkpoint、first valid `MAINTENANCE_INTENT(INITIAL_PROVISION)`、`MAINTENANCE_ACTIVE`、owner/actor/completionをbindしたordinary `TERMINAL_CLEAN`、flush/close/reopen、standard-user `OPEN_EXISTING`更新、別user拒否、repair/update/uninstallの各境界を一つのfresh-provision matrixで検査する。途中crashでmissing/partial/identity不一致をdirect clean sentinelへ推測せず、Phase 2A mutation/process launch 0件にする。
- architecture 19.2の全13 wire stateをcanonical field orderでserialize/deserializeし、UI projection名をwireへencodeできないことを検査する。
- 各stateのrequired field不足、forbidden field混入、optional group片側欠落、unknown enum、unknown schema、recordLength/checksum mismatchを`FAILED_CLOSED`へ分類する。
- machine gate取得、ACTIVE_INTENT partial/full write、ACTIVE_PREPARED/WATCHDOG_READY/APPLY_ARMED/MUTATED link、WAL terminal、RESTORING/TERMINALIZING、TERMINAL_CLEAN partial/publication、lock releaseの各境界へcrashを注入する。
- machine record write/flush/reopen遅延、ACTIVE_INTENT前にper-user WAL PREPAREDが先行するfault、generation/bootId/owner mismatch、abandoned mutex、owner evidence取得失敗、actor判定不能を個別注入する。
- ACTIVE_INTENT前のWAL PREPAREDと、owner WAL terminal/actor quiescence前のTERMINALIZING/TERMINAL_CLEANがprotocol checkerで必ず拒否される。
- controller process同一/`controllerInstanceId`変更、watchdog process同一/`watchdogInstanceId`変更を別caseにし、old instance operationだけをfenceしてlive process identityを誤killしない。
- `UNKNOWN`/全`ACTIVE_*`/`RECOVERY_REQUIRED`/`RESTORING`/`TERMINALIZING`/`TERMINAL_CLEAN`/maintenance states/`FAILED_CLOSED`と、WAL absent/partial/nonterminal/terminal/unreadable/mismatchを直積で検査し、unsafe combinationでmaintenance/new mutationが0件になる。
- standard user、別interactive user、elevated installer/updater/repair/uninstall、SYSTEMのDACL matrixをexact support cellで検証する。owner WAL read denied、sharing violation、reparse、ACL tamper、record checksum/schema mismatchはfail closedになる。
- update/repair/uninstallをbegin、staging、binary replace、completion-record、rollbackの各境界で停止し、old recovery readerを早期削除せず、record/WAL/actor不整合でbinary operationを開始しない。
- old installer readerがnew schema/stateを拒否し、new writerが`recoveryBinaryVersion`のold recovery binaryを保持すること、schema migration crash中のupdate/uninstall禁止を検査する。
- maintenance/update/repair/uninstallごとにmachineEpoch/boot/binary/recovery binary/terminal generation/actor/owner/nonce/intent/completion/reject codeを照合し、owner unavailable、installer crash、antivirus/file-lock interferenceでfail closedになる。

### 7.6 Rollback result

- `KEEP_AUTHORIZED`前のtimeout、explicit Revert、parent loss、apply mismatchからC0 exact restore。authorization後のparent/writer lossはDecisionJournal readback oracleに従う。
- exact failure時のvalidated P0 fallbackと`RESTORED_DEGRADED`。
- P0 fallback failure、persisted drift、target ambiguityでcritical terminal。
- restore/confirmのduplicate messageが追加mutationを起こさない。

## 8. Windows API read-only実機test

- `EnumDisplayDevicesW`とCCD source/target/device pathのcross-map。
- current GDI/CCDとWindows Settings表示の一致。
- `EnumDisplaySettingsExW` normal/raw mode、duplicate、current/registry、current-not-listed。
- rational refresh 59.94/60、DRR/virtual refresh、source/target dimension。
- preferred mode、DLDSR-like、HDR/advanced color、bitsPerPel、orientation observation。
- hotplug/dock/GPU switching時のbuffer retryとstale detection。
- local standard user、RDP、Fast User Switching、virtual display、multi-path。
- Windows 10/11とWebView2 runtime差。

read-only spikeでは`CDS_TEST`を含むmutation関連callも、承認範囲でない限り実行しない。API名がtestでも、明示承認されたPhase 1Bへ分離する。

## 9. Controlled mutation実機test

専用lab、out-of-band capture、blind recovery手順、指定Operator/Evidence Owner、別承認が前提。

- valid resolution change、valid refresh change、combined change。
- invalid resolution/refresh拒否とmutation 0件。
- `CDS_TEST`/`SDC_VALIDATE`とactual applyの結果差。
- expected exact/mismatch/driver-adjusted readback。
- 15秒timeout、manual revert、Tauri forced kill、worker failure。
- C0!=P0 exact restore、P0 fallback、persisted不変。
- black screen想定でkeyboard/mouseを使わずwatchdog restore。
- HDR/color depth/VRR/DRR/ICC等のbefore/after非意図変更。
- sign-out/reboot/driver reset/full-process lossの保証境界。
- 2-stage presentation ACK、Confirm/Revert競合、watchdog単独crash/takeover、cross-session fence。

## 10. Black screen test

### 準備

- remote desktopを唯一の観測/復旧手段にしない。
- out-of-band camera/capture、別端末、電源操作、reboot/sign-out手順を準備する。
- baseline/expected/timeoutを事前記録し、operatorが画面を見ないblind procedureを用意する。
- production相当signed packaged artifactとexact hardware manifestを使う。

### Scenario

- unsupported/marginal modeをproduct pathへ入れず、lab qualification tokenだけで試す。
- main window invisible/off-screen、WebView freeze、Tauri kill、watchdog control loss。
- worker hang/late exit、watchdog crash、全process kill。
- cable transient、display power off/on、dock removal、GPU driver reset。

### Pass

- watchdog生存範囲ではdeadline contract内にrollback callを開始し、C0/P0 exact readbackへ到達する。
- old workerが生存する場合は並行callせずblockedとなる。
- Tauri core+watchdog同時loss/all-process loss範囲ではapproved blind procedureでP0へ戻る。watchdog単独lossはreplacement takeoverでC0へ戻す。
- manual interventionが必要、別target変更、結果不明ならcell failureである。

## 11. Multiple monitor test

初期版はmutationを拒否することをtestする。

- 2+ active pathsでcandidate selection token/Applyを出さない。
- clone/extend、primary/sub、inactive connected、dock hotplugでsingle pathへ誤分類しない。
- multi-path中にold single-path tokenでbeginしてもRust側でstale拒否。
- pending transaction中のhotplugでは同一target以外を推測変更しない。

将来multi-monitor mutationはfull CCD plan、compensation order、V2 command、別threat modelと別matrixを要求する。

## 12. Release candidate hardware manifest

次はcoverage classであり、exact machineではない。Phase 0/1で各採用classをexactly one以上の物理cellへinstantiateする。Windows 10/11の双方をproduct targetとするため、少なくとも各OS laneに1つのprimary GPU/display cellが必要である。GPU vendorを限定しないと公称する場合はNVIDIA/AMD/Intel各vendorのcellが必要になる。

| Coverage ID | 必須目的 | 初期扱い |
| --- | --- | --- |
| W10-BASE | 承認済みWindows 10 edition/build/KB、standard user、single path | 必須。ESU/LTSC/consumer方針未決定 |
| W11-BASE | 承認済みWindows 11 edition/build/KB、standard user、single path | 必須 |
| GPU-PRIMARY | 想定primary vendor/model、high-refresh external display | 必須、exact vendor未決定 |
| GPU-ALT | primaryと異なるAMD/Intel/NVIDIA vendor | broad supportを公称する場合必須 |
| INTERNAL | integrated GPU/internal panel | laptopをsupportする場合必須 |
| DOCK | USB-C/Thunderbolt dock、internal disabled single path | dockをsupportする場合必須 |
| RR-FRACTIONAL | 59.94/60または同種のrational ambiguity | 必須 |
| C0-NE-P0 | currentとpersistedが異なるbaseline | 必須 |
| WEBVIEW2 | selected runtime/install mode | 必須 |

各manifest cellはOS edition/build/KB、CPU arch、GPU vendor/model/device ID、driver exact version、display vendor/model/firmware、sanitized EDID fingerprint、connector/port/dock、WebView2 version、C0/P0、requested/expected tuple、transition/fault ID、artifact hash/signature、strategy/schema/protocol、owner/approverを持つ。「latest」「または」「以上」、range、placeholderを許可しない。

## 13. Mandatory repetitionsとzero tolerance

frozen cellの全transitionについてKeep 3回、explicit revert 3回、timeout revert 5回を行う。各cellで最も厳しいapproved `TRANS-STRESS`のtimeout revertを50回連続で行う。

- WebView/Tauri crash/pipe disconnect: 各10回
- worker/watchdog/PID reuse/process query fault: 採用GPU classごと各10回
- mutex/lock/suspend-resume: 各5回
- dock/hotplug: 該当cellで各10回
- full-process lossからblind reboot/sign-out/physical recovery: 各cell5回

一度でも次が起きれば、root cause修正またはcell/scope除外までrelease blockerとする。

- 別monitor/pathまたは意図しない属性を変更
- rollback decision/call-start SLA違反
- exact/degradedに分類できないbaseline mismatch
- rollback failure、worker exit未確認、approved blind procedure以外のmanual recovery
- journal/epoch/owner不明、protocol fault継続、stale actor operation

安全failureはwaiverで合格にしない。scope除外はProduct ownerとSafety reviewerの共同承認を要する。

## 14. Packaged application/installer/WebView2

- NSIS per-machine/per-user、MSI clean install、upgrade、repair、uninstall。
- active/pending/degraded/failed/blocked journal中にbinaryを削除しない。
- machine maintenance/mutation gateとprotected machine actor recordを全user/sessionで確認し、active/pending/critical/unknown/owner-unavailable時にinstall/repair/upgrade/uninstallを拒否する。
- User A transaction中のUser B/admin maintenance、Fast User Switching、abandoned gate、boot変更、stale record、別user journal非復元を検証する。
- watchdog/workerが全architecture artifactへ同梱され、signed publisher/hashが一致する。
- install path space/non-ASCII、standard user runtime、UAC境界。
- Tauri core forced kill後にwatchdogが同じJob/process treeで終了しない。
- WebView2 present/missing/old、online bootstrapper、offline/embedded候補、proxy/no network。
- production Capability/CSP/navigation/DevToolsとdev URLの不在。

## 15. CIでできること/できないこと

### CIで行う

- React unit/component/accessibility static test
- API wrapper/mock/state/error test
- Rust format/lint/unit/property/fuzz/unsafe inventory
- Tauri command DTO、Capability/Permission manifest、CSPのstatic/packaged inspection
- journal/watchdog/worker protocolのfake-clock/fake-worker fault test
- Windows CI runnerでread-only wrapper testとpackage build（承認されたphase以降）
- requirement/test/evidence traceabilityのmissing/duplicate/orphan検査

### CIだけではできない

- physical monitorのmode enumeration fidelityとvisibility
- real GPU driverのapply/hang/readback/rollback
- black screen、cable、dock、HDR、DLDSR、DRR、color pipeline
- Task Manager/process Job/installer/AV/SmartScreenの全挙動
- exact Windows 10/11 hardware support qualification
- blind physical recovery、Narratorでの固定15秒accessibility

cloud VMやvirtual displayでpassしてもphysical display supportを証明しない。

## 16. Evidence

各runは`EVD-{RC}-{CELL}-{TEST}-{RUN}`を持ち、artifact hash/signature、OS/KB/WebView2、driver/firmware/GPU/display/connection、C0/P0/R/observed/rollbackのsanitized structure、`GetTickCount64` tickとwall diagnostic timestamps、journal generation/leaseVersion/epoch、Operator/Evidence Owner/Reviewer、out-of-band video reference、結果/逸脱を保存する。failed runを再実行で上書きしない。

## 17. Requirement traceability

Statusは全て`planned`であり、evidenceは未生成である。

### Functional requirements

| Requirement | Stable test | Level / phase | Evidence |
| --- | --- | --- | --- |
| FR-001 | WIN-QUERY-001 | read-only Windows / P1A | planned |
| FR-002 | WIN-ID-001 | Rust + physical / P1A | planned |
| FR-003 | WIN-MODE-001 | Rust + physical / P1A | planned |
| FR-004 | MAP-EXACT-001 | Rust + qualification / P1A/P1B | planned |
| FR-005 | MAP-RATIONAL-001 | Rust + physical / P1A | planned |
| FR-006 | SNAP-STALE-001 | Rust/React / P3-P5 | planned |
| FR-007 | SCALE-READ-001 | Rust + physical / P1A/P9 | planned |
| FR-101 | UI-CANDIDATE-001 | React / P4 | planned |
| FR-102 | UI-REFRESH-001 | React / P4 | planned |
| FR-103 | UI-DIFF-001 | React/a11y / P4 | planned |
| FR-104 | UI-SCALE-001 | React / P4 | planned |
| FR-105 | MAP-CURRENT-001 | Rust/React / P1A/P4 | planned |
| FR-106 | UI-BUSY-001 | React/command / P4-P6 | planned |
| FR-201 | CMD-SURFACE-001 | security / P3-P5 | planned |
| FR-202 | CMD-VALIDATE-001 | Rust command / P3-P5 | planned |
| FR-203 | CMD-ALLOWLIST-001 | Tauri security / P3 | planned |
| FR-204 | TX-SINGLE-001 | Rust/watchdog / P2A-P6 | planned |
| FR-205 | TX-SESSION-001 | Rust/storage / P2A-P5 | planned |
| FR-206 | TX-STALE-001 | Rust/protocol / P2A-P6 | planned |
| FR-207 | EVENT-RESYNC-001 | React/Tauri / P4-P5 | planned |
| FR-208 | UI-PRESENT-CMD-001 | React/Tauri/watchdog / P2A-P6 | planned |
| FR-301 | REC-PREPARE-001 | watchdog fault / P2A/P2B | planned |
| FR-302 | REC-SCHEMA-001 | storage / P2A/P2B | planned |
| FR-303 | REC-GO-001 | watchdog/worker / P2A/P2B | planned |
| FR-304 | WIN-PREFLIGHT-001 | controlled physical / P1B | planned |
| FR-305 | WIN-TEMP-001 | controlled physical / P1B | planned |
| FR-306 | WIN-READBACK-001 | controlled physical / P1B | planned |
| FR-307 | REC-DEADLINE-001 | watchdog + physical / P2A/P2B/P7 | planned |
| FR-308 | UI-PRESENT-001 | React/Tauri/physical / P4-P7 | planned |
| FR-309 | TX-KEEP-001 | watchdog/physical / P2A/P2B/P7 | planned |
| FR-310 | TX-ABORT-001 | fault / P2A/P2B/P6 | planned |
| FR-311 | REC-LIFETIME-001 | watchdog clock / P2A/P2B | planned |
| FR-401 | REC-EXACT-001 | watchdog/physical / P2A/P2B/P7 | planned |
| FR-402 | REC-DEGRADED-001 | watchdog/physical / P2B/P7 | planned |
| FR-403 | REC-AMBIG-001 | Rust/watchdog / P2A/P2B/P6 | planned |
| FR-404 | REC-TRIGGER-001 | watchdog / P2A/P2B | planned |
| FR-405 | REC-IDEMPOTENT-001 | Rust/watchdog / P2A/P2B | planned |
| FR-406 | REC-LIMIT-001 | design/physical / P0/P7 | planned |
| FR-407 | REC-BLIND-001 | physical / P7 | planned |
| FR-408 | UI-CRITICAL-001 | React/Rust / P4-P6 | planned |
| FR-409 | REC-TAKEOVER-001 | watchdog/process / P2A/P2B-P7 | planned |
| FR-410 | UI-VIEW-FENCE-001 | React/Tauri/watchdog / P2A-P6 | planned |
| FR-411 | REC-MACHINE-WAL-001 | storage/process/installer / P2A-P8 | planned |
| FR-412 | REC-CLOCK-ORACLE-001 | watchdog/React / P2A-P6 | planned |
| FR-413 | REC-BASELINE-PROVISION-001 | storage/watchdog fault / P2A | planned |
| FR-414 | REC-DECISION-ROLLOVER-001 | storage/security fault / P2A | planned |
| FR-501 | ENV-SINGLE-001 | Rust/physical / P1A/P7 | planned |
| FR-502 | ENV-REMOTE-001 | Rust/physical / P1A/P7 | planned |
| FR-503 | REL-MATRIX-001 | release audit / P7 | planned |
| FR-504 | REL-W10-001 | human decision / P0/P7 | planned |
| FR-505 | REL-MAINT-001 | process/installer / P2A/P8 | planned |

### Acceptance criteria

| Acceptance | Stable test | Evidence |
| --- | --- | --- |
| AC-001 | WIN-QUERY-001 + WIN-ID-001 | planned |
| AC-002 | UI-CANDIDATE-001 + UI-NOMUTATE-001 | planned |
| AC-003 | UI-DIFF-001 | planned |
| AC-004 | CMD-FAILCLOSED-001 | planned |
| AC-005 | REC-PREPARE-001 + REC-GO-001 | planned |
| AC-006 | MAP-EXACT-001 + WIN-READBACK-001 | planned |
| AC-007 | REC-TRIGGER-001 + REC-EXACT-001 | planned |
| AC-008 | REC-QUIESCE-001 | planned |
| AC-009 | TX-KEEP-001 | planned |
| AC-010 | REC-C0P0-001 | planned |
| AC-011 | REC-CRASHPOINT-001 | planned |
| AC-012 | TX-FENCE-001 | planned |
| AC-013 | SEC-CAPABILITY-001 | planned |
| AC-014 | SEC-INTEGRITY-001 | planned |
| AC-015 | UI-SCALE-001 | planned |
| AC-016 | REL-MATRIX-001 + REL-INSTALL-001 | planned |
| AC-017 | REC-BLIND-001 | planned |
| AC-018 | DOC-DECISION-001 | planned |
| AC-019 | UI-PRESENT-CMD-001 | planned |
| AC-020 | TX-CONFIRM-REVERT-001 | planned |
| AC-021 | REC-TAKEOVER-001 | planned |
| AC-022 | REC-CLOCK-BOOT-001 | planned |
| AC-023 | TX-FENCE-MATRIX-001 | planned |
| AC-024 | ENV-CROSSSESSION-001 + REL-MAINT-001 | planned |
| AC-025 | UI-VIEW-FENCE-001 + UI-PRESENT-CMD-001 | planned |
| AC-026 | REC-DECISION-JOURNAL-001 + REC-CRASHPOINT-001 | planned |
| AC-027 | REC-MACHINE-WAL-001 + REL-MAINT-001 | planned |
| AC-028 | REC-HEARTBEAT-POLICY-001 + REC-TAKEOVER-001 | planned |
| AC-029 | REC-BASELINE-PROVISION-001 + REC-CRASHPOINT-001 | planned |
| AC-030 | REC-DECISION-ROLLOVER-001 + REC-DECISION-JOURNAL-001 | planned |

## 18. Release gate

releaseには、全traceability行のevidence、frozen finite manifest、mandatory repetition、zero-tolerance項目0件、未決定事項の人間decision、signed packaged artifact、watchdog independence、blind recoveryの承認が必要である。CI greenだけではreleaseできない。
