# DisplayDeck Tauri設計 最終独立レビュー

レビュー日: 2026-08-05  
レビュー種別: DD-RRR-001〜DD-RRR-003修正の独立再評価  
実施範囲: 設計レビューのみ  
実施していないこと: 既存設計文書の修正、Phase開始記録の入力、ソース/Cargo/Tauri projectの作成、build/test、Windows API実行、file API実験、display setting変更、Phase 1A/2A/1B/2Bの開始

## 1. 最終判定

**APPROVED_WITH_MINOR_DESIGN_CONDITIONS**

現行設計には新しいCritical/Highは見つからなかった。前回Highであったdeadline-coupled durable CASは、deadlineをdisk I/O完了条件から外し、watchdogのdecision lock内で`KEEP_AUTHORIZED`へ入る時点をConfirm acceptanceのlinearization pointにすることで、Windows APIが提供しない「deadline付きdurable CAS」への依存を解消している。`BOOT_HANDSHAKE`のaccepted pathと`MachineActorRecordV1`のcanonical schemaも、元指摘の中心を解消している。

ただし、`DecisionJournalV1`について、current session用`REVERT_REQUIRED` baselineの初回作成と、旧session/旧boot/別displayのslotが残る固定名fileから次sessionへ移るnormative algorithmがない。この不足はfail-closed floorにより直ちにunsafe Keepを許すHighではないが、実装者がgeneration root、slot reclaim、old-slot validation、crash後の再開を選ぶ余地があり、Phase 2Aのfault oracleを一つにできないMediumの設計gapである。

したがって、設計修正ループを完全終了したとは判定しない。後述のMedium 1件を設計文書へ反映して再確認した後、Phase 1A開始記録の入力へ進める。これはPhase 1A自体の開始承認ではない。Phase 2A、Phase 1B、Phase 2Bも未承認のままである。

件数:

- Critical: 0件
- High: 0件
- Medium: 1件
- Low: 1件
- Question: 0件

## 2. 確認した文書

指定された現行文書はすべて存在し、すべて確認した。欠落文書はない。

| 文書 | 扱い | 主な確認対象 |
| --- | --- | --- |
| `AGENTS.md` | repository gate / safety正本 | 設計revision限定、watchdog/worker/clock/journal/machine record制約 |
| `docs/requirements.md` | 現行要件 | FR-207/307/309/404/409〜412、AC-025〜028 |
| `docs/architecture.md` | 現行architecture正本 | 5.4、7.4、10〜12、16〜19 |
| `docs/windows-display-research.md` | Windows API調査 | Phase 1A/1B/2A境界、file API保証の扱い |
| `docs/ui-design.md` | UI契約 | accepted/committed表示、BOOT/ORDINARY resync、presentation再構築 |
| `docs/security.md` | security契約 | command surface、journal path、writer fencing、machine schema参照 |
| `docs/testing-strategy.md` | test計画 | DecisionJournal、BOOT_HANDSHAKE、MachineActorRecord、phase routing |
| `docs/implementation-plan.md` | phase/gate正本 | P1A read-only、P2A no-mutation、G2A前のP1B禁止 |
| `docs/risks-and-open-questions.md` | residual risk | R-T17〜R-T19、Q02/Q07/Q10 |
| `docs/tauri-migration.md` | migration record | 6 command、watchdog ownership、現行Tauri boundary |
| `docs/tauri-design-review-checklist.md` | review checklist | command、WAL、fencing、phase gate |
| `docs/tauri-design-review.md` | 過去Tauri review | TDR指摘の履歴として確認 |
| `docs/tauri-design-rerereview.md` | 直前review | DD-RRR-001〜003の元指摘と判定 |
| `docs/tauri-review-resolution.md` | resolution自己記録 | 設計変更の所在確認。RESOLVED表記を結論根拠には不採用 |

`docs/design-review.md`と`docs/review-resolution.md`は旧Electron設計の履歴であり、今回の現行wire/protocol判定の正本にはしていない。

このworkspaceにはGit metadataが存在しなかったため、commit差分ではなく、2026-08-04更新の現行文書本文と相互参照を直接照合した。

## 3. DD-RRR別判定

| ID | 判定 | 理由 | Phase 1A | Phase 2A | Phase 1B |
| --- | --- | --- | --- | --- | --- |
| DD-RRR-001 | `PARTIALLY_RESOLVED` | `KEEP_AUTHORIZED` acceptance、accepted/committed分離、A/B publication、outcome unknownは解消。current-session baselineの初期化/rollover algorithmだけがMediumで残る | 技術範囲は非block。設計loop closure前なので開始記録completionは保留 | baseline/byte-layout freezeとfault oracle確定までblock | G2A evidence承認までblock |
| DD-RRR-002 | `RESOLVED_WITH_EVIDENCE_CONDITIONS` | `StatusRequestV1`、3 mode、root remount accepted path、page lifecycle、authority非移送が一意 | 非block | protocol modelとlifecycle evidenceが必要 | P2A/G2Aおよびpackaged Tauri evidenceまでblock |
| DD-RRR-003 | `RESOLVED_WITH_EVIDENCE_CONDITIONS` | architecture 19.2だけが34 fields/13 wire states/state別contractの正本。他文書は参照/projectionに限定 | 非block | serializer/DACL/old-reader/migration fault evidenceが必要 | G2A evidence承認までblock |

`RESOLVED_WITH_EVIDENCE_CONDITIONS`は、設計上の安全結果は一意だが、Phase 2AのWindows実証に失敗すれば後続Phaseへ進めない、という意味に限定している。

## 4. 指摘事項

### DD-FR-001 — `DecisionJournalV1` baselineの初期化・session rolloverが未定義

- **ID**: DD-FR-001
- **重要度**: Medium
- **関連するDD-RRR**: DD-RRR-001
- **対象文書**: `docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`
- **対象セクション**: architecture 7.4 `DecisionJournalV1 wire layout` / `Normative publication algorithm` / recovery algorithm、security 7.2〜7.3、testing 6.1/7.4
- **問題**: architectureは「mutation前にcurrent session用のvalid `REVERT_REQUIRED` generationをwrite/flush/close/reopen検証する」と要求するが、そのbaselineを作るnormative algorithmを持たない。初回file作成時のgeneration/`previousGeneration`、A/B双方未初期化、旧sessionのterminal slotが残る場合、bootId/displayId/owner/leaseが違う場合、どのslotをreclaimするか、baseline publication中にcrashした場合の再開規則が未指定である。
- **具体的なrace/crash/互換性問題**: 旧session S0の`KEPT_SESSION`と`REVERT_REQUIRED`がA/Bに残る状態で、新session S1がbaselineを作る。実装Aはgenerationをsessionごとにrootへresetし旧slotをidentity mismatchとして除外する。実装Bはfile-global generationを継続しcross-session `previousGeneration` chainを作る。実装Cは旧fileをtruncate/recreateする。baseline slotのpartial write直後にcrashしたとき、Aは旧terminalを別session evidenceとして保持、Bはcross-session chain、Cは両slot喪失になり得る。いずれも安全側に実装できるが、現文書だけではstartup actorが`REVERT_REQUIRED`、old terminal、`FAILED_CLOSED`のどれを選ぶか一意でない。
- **Highにしない理由**: current session/boot/lease/display/ownerのexact validationと「current sessionのvalid baseline readback前はapply GO禁止」というfloorがある。このfloorを守れば旧sessionのKeepをcurrent sessionのKeepとして採用せず、曖昧時はmutation 0件/failed closedにできる。主な影響は実装分岐、second transaction不能、fault oracle不一致であり、現時点でunsafe mutationを必然化しない。
- **推奨修正**: architecture 7.4に`ProvisionCurrentDecisionBaselineV1`を追加し、少なくとも次を固定する。
  1. fresh file headerとA/B uninitialized markerのexact状態、初回`generation`、rootを表す`previousGeneration`値。
  2. machine/display/user/journal-writer lockとold actor absenceを検証してからだけbaseline provisioningすること。
  3. 旧session/旧boot/別display slotはcurrent chainへ接続せず、linked operational WALとmachine `TERMINAL_CLEAN`を証明できる場合だけ規定slotをreclaimすること。証明不能ならfail closed。
  4. current `REVERT_REQUIRED` slotをfull-write/checksum/flush/close/reopenし、exact identity/root chainをreadbackするまでapply GOを禁止すること。
  5. baseline partial/torn/flush/readback crashは「current sessionは未provision、mutation 0件」とし、旧terminal evidenceをcurrent decisionに昇格させないこと。
  6. current baseline確立後の`KEPT_SESSION` target slotを一意にし、Keep partial write後にもcurrent baselineが必ず残ること。
  7. old normal terminalのretention/reclaim、critical/blocked evidenceの上書き禁止をoperational WAL rolloverと整合させること。
- **修正しない場合の影響**: Phase 2Aの同一fault imageに複数oracleが生じ、旧session terminalの永久block、evidence喪失、またはversion間readerのchain不一致が起きる。安全側実装ではmutation不能となる。
- **Phase 1A blockerか**: Phase 1Aのread-only技術範囲そのものはNo。ただし本レビューのdesign-loop closureとPhase 1A開始記録completionにはYes。
- **Phase 2A blockerか**: Yes。fault harness作成前にalgorithmを一意にする必要がある。
- **Phase 1B blockerか**: Yes。Phase 2A/G2Aでbaseline/rollover evidenceが承認されるまで開始不可。
- **分類**: 設計修正。その後にPhase 2A実証が必要。

### DD-FR-002 — fixed-slot wire layoutとworker-instance test oracleをPhase 2A前にfreezeする必要がある

- **ID**: DD-FR-002
- **重要度**: Low
- **関連するDD-RRR**: DD-RRR-001、DD-RRR-003
- **対象文書**: `docs/architecture.md`、`docs/testing-strategy.md`
- **対象セクション**: architecture 7.4/19.2、testing 6.1/7.5
- **問題**: `DecisionJournalV1`はcanonical field orderまで定義するが、各fieldのexact wire type/width/byte order、header/slot offset/size、checksum coverage、reserved/padding初期値、exact file length、trailing bytes/EOF/sparse/truncate policyをまだ固定していない。`MachineActorRecordV1`もsemantic field/state contractは一意だが、checksum coverageを含むserialization profileは未freezeである。またtest計画はcontroller/watchdogのsame-process instance rotateを明記する一方、one-shot workerについて「same processでinstance rotateを禁止する」caseが明記されていない。
- **具体的な互換性問題**: 同じschema versionでwriterが64-bit little-endian、old recovery readerが別width/coverageを想定すると、同じslotを一方がvalid、他方がcorruptと判定し得る。file tail/short fileをreaderごとにaccept/rejectするとoutcome unknownの分類が変わる。worker processが生存したまま`workerInstanceId`だけをrotateできる実装はone-shot invariantを破り、old resultをnew instanceへbindし得る。
- **推奨修正**: Phase 2A開始前のdesign-freeze artifactとして、numeric layout、endianness、offset、exact length、checksumの包含/除外byte、reserved/padding zeroing、unknown/trailing data、create/open disposition、eligible local volume/filesystem policyをversion付きで固定する。workerはone-shotであるためsame-process instance rotateを明示rejectし、そのtestを追加する。
- **今回すべてのbyte layoutを確定すべきか**: Phase 1A開始記録へ進む前にnumeric offsetまで確定する必要はない。Phase 2Aはこのwire formatを実装してfault injectionするPhaseなので、Phase 2A code/file作成前にfreezeし、G2Aでevidenceとともにreviewする必要がある。Phase 2Aの実装者が試行中に同じschema versionの意味を変更することは許されない。
- **修正しない場合の影響**: Phase 2A evidenceが再現不能になり、old reader/recovery binary compatibilityとchecksum testが実装固有になる。
- **Phase 1A blockerか**: No。
- **Phase 2A blockerか**: Yes（pre-execution wire freeze）。
- **Phase 1B blockerか**: Yes（G2A closureまで）。
- **分類**: 設計freeze事項。Windows上の実挙動は別途実証事項。

## 5. DD-RRR-001 詳細評価

### 5.1 Confirmのlinearization point

architecture 7.4は次の7 stepを一意に定義している。

1. authoritative watchdogがdecision lockを取得する。
2. session/boot/epoch/lease/state/generation/controller/watchdog/owner/logon/display/nonce/readback tupleを再検証する。
3. stateがexact `AWAITING_CONFIRMATION`であることを確認する。
4. Revert classが先に勝っていないことを確認する。
5. lock保持中にwatchdogが`GetTickCount64`を直接読む。
6. confirmation deadlineとmaximum session lifetimeを確認する。
7. lock保持中に`AWAITING_CONFIRMATION -> KEEP_AUTHORIZED`へone-way transitionする。

判定:

- lock取得だけではConfirm成立にならない: **Yes**
- deadline前にstep 7へ到達する必要がある: **Yes**
- `KEEP_AUTHORIZED`後にtimeout/manual Revert/EOF/presentation failure/session changeがordinary winnerにならない: **Yes**
- `KEEP_AUTHORIZED`をReact successとして公開しない: **Yes**
- restart後にmemory stateだけでKeepを成立させない: **Yes**
- Tauri core local stateでwinnerを決めない: **Yes**
- watchdogがauthoritative decision owner: **Yes**
- duplicate Confirmがterminal journal resultへ収束する: **Yes**

この変更により、前回問題の「durable I/O completionとdeadlineを同じatomic primitiveへcoupleする」必要はなくなった。

### 5.2 acceptedとcommitted

| 段階 | 現行contract | 判定 |
| --- | --- | --- |
| Confirm accepted | decision lock内で期限内に`KEEP_AUTHORIZED`へ遷移。journal I/Oは未完了でもよい。React success不可 | 一意 |
| Confirm commit in progress | ordinary Revert/timeoutを開始せず、writerがA/B publicationを継続 | 一意 |
| Confirm committed | new `KEPT_SESSION` full write、`FlushFileBuffers`、close/reopen、A/B readback、identity/chain/digest検証完了 | 一意 |
| React success | committed後だけ | 一意 |

acceptedでsuccessを返す、I/O開始をcommitにする、`FlushFileBuffers` returnだけでcommitにする、readback前に正常終了する、deadline後にacceptedを取消す、という旧問題は現行contractにはない。

### 5.3 `DecisionJournalV1` file structure

次は定義済みである。

- per-user Recovery directoryの固定名1 file
- fixed file header
- fixed offset/fixed-size slot A/B
- active-slot selector、rename-based head、別head fileなし
- 各slotは自己完結し単独検証可能
- readerがA/Bを検証し、current identity内の最大valid generationを選ぶ
- same-generation conflictは`FAILED_CLOSED`
- slotIndexとfixed offset/recordLength不一致を拒否

file header内にmutable active selectorを復活させた記述はない。

一方、初回baselineと旧sessionからのrolloverはDD-FR-001、numeric wire/file shapeはDD-FR-002の条件である。

### 5.4 baseline Revert

設計上の安全floorは明確である。

- current session用のvalid `REVERT_REQUIRED`をmutation前に作る。
- full write、flush、close/reopen、readbackが成功しなければapply GOを渡さない。
- new Keepがpartial/torn/invalidならcurrent sessionのprior valid baselineからRevertする。
- baselineも読めなければ`FAILED_CLOSED`であり、推測Keep/Revertをしない。

ただし、初回generation、root chain、A/B双方未初期化、old session/boot/display slotのreclaim規則がないため、「旧valid slotがcurrent `REVERT_REQUIRED`として常に残る」ことを全session lifecycleでまだ機械的に証明できない。よってbaselineは**single-session内では保証されるが、fixed fileのsession rolloverを含めては条件付き**である。

### 5.5 slot fields

要求されたsemantic fieldはarchitecture 7.4に存在する。

- magic/schemaVersion/slotIndex/recordLength
- generation/previousGeneration/stateVersion/decision
- sessionId/bootId/leaseVersion
- controllerInstanceId/watchdogInstanceId
- displayId/ownerSidDigest/logonId
- confirmationDeadlineTickMs/keepAuthorizedTickMs/decisionWrittenTickMs
- candidateDigest/expectedDisplayModeDigest
- payloadChecksum/headerChecksum

`owner identity digest`は`ownerSidDigest`と`logonId`へ分けており、単一曖昧digestより強いfenceである。`KEEP_AUTHORIZED`をwire decisionへ保存しないことも明確である。

wire type/width/endianness/bounded length/checksum byte range/reserved/padding/file-size policyはDD-FR-002のpre-Phase 2A freeze項目である。unknown schemaはfail closedだが、同一schema内unknown/reserved bytesの扱いもfreeze時に固定する必要がある。

### 5.6 publication algorithm

architecture 7.4の8 stepは次を一意にする。

1. current writerがdecision/journal-writer ownershipとactor/sessionを検証。
2. header/A/Bをfixed offsetから読み、schema/length/checksum/identity/chainとexpected generation/stateVersionを比較。
3. current maximumではない規定slotへnext `KEPT_SESSION`を構築。
4. fixed-size slot全体を1回のbounded `WriteFile`対象とし、short writeをfailure化。
5. `FlushFileBuffers`。
6. close、fixed path/identity検証、reopen。
7. A/B再読込でnew generationが唯一の最大valid chainであることを検証。
8. committedへ進みReact successを返す。

`KEEP_AUTHORIZED`後はtimeout/Revert classがdecisionを逆転させず、watchdog/core monitorはwriter processとheartbeatを監視する。replacementはold writer exit、machine/display/user/journal-writer locks、worker quiescence、lease/instance fencing前にwriteしない。したがって同時writerを許すcontractではない。

文面上、step 1で得たlockの解放点は明記されていないが、step 1〜7のjournal-writer ownership継続とreplacement write禁止からsafe readingは一つである。DD-FR-001を直す際、「decision lockはstep 7 transition後にrelease可能だがjournal-writer lockはreadback/outcome transferまで保持する」等、lockごとのlifetimeを明文化すると実装reviewが容易になる。

### 5.7 crash table

| Crash/failure point | 現行結果 | 評価 |
| --- | --- | --- |
| `KEEP_AUTHORIZED`前 | Revert | 一意 |
| authorization後/slot write前 | memory authority消失、valid Keepなし、Revert | 一意 |
| partial/torn/short write | new slot invalid、prior current baselineからRevert | baseline rollover条件付き |
| full write/flush前 | reopen時new slot validならKeep、invalid/不在ならprior Revert | 一意 |
| flush後/readback前 | new slot validならKeep、読めなければ`FAILED_CLOSED` | 一意 |
| readback後/React response前 | Keep | 一意 |
| terminal Keep後Revert | terminal reject | 一意 |
| duplicate Confirm | 同じterminal Keep result | 一意 |
| 両slot破損/same-generation conflict/unreadable | `FAILED_CLOSED` | 一意 |

15秒は`KEEP_AUTHORIZED` entry期限であり、期限内accepted後のI/Oはdeadline外で継続する。React countdownはsecurity decisionではなく、successはcommitted後だけである。

### 5.8 outcome unknown

次が一意に定義されている。

- write/flush/close/reopen/readback failure直後にRevert slotで上書きしない。
- old writerの停止/exit、locks、lease/instance、worker quiescenceを証明する。
- writer ownership移送前にreplacementはwriteしない。
- close/reopen後のA/Bだけからvalid Keep / no Keep / unreadable-conflictを判定する。
- valid new KeepならKeep、current baselineだけならRevert、読めなければ`FAILED_CLOSED`。
- alive-but-hung writerからownershipを奪わず、termination不能はcritical/blocked。

alive-but-hung threshold、termination permission、AV/EDR/filter delay、power-lossはPhase 2Aの実証事項であり、現在のdesignが成功を推測する余地ではない。

### 5.9 Microsoft公式資料との照合

Microsoftの[`FlushFileBuffers`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)は、指定fileのbuffered informationをdeviceへ書き出すAPIである。deadline評価、expected generationとのCAS、multi-slot atomicityはcontractに含まれない。現行設計はそれらを主張していない。

Microsoftの[`WriteFile`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-writefile)はbytes-writtenを返し、synchronous handleではoperation completionまでreturnしない一方、multi-sector writeのatomicityを一般保証しない。現行設計がshort writeを拒否し、multi-sector slotをatomicだとせず、checksumとprior slot/readbackで扱う方針は整合する。

Microsoftの[File buffering](https://learn.microsoft.com/en-us/windows/win32/fileio/file-buffering)はphysical diskのhardware cachingがuser-mode/systemの直接制御外であることを明記する。現行設計がpower-loss絶対耐久性をdocumented APIだけで断定せず、filesystem/storage/support cellをPhase 2A evidenceへ送る点は妥当である。

Microsoftの[`CreateFileW`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)は`FILE_FLAG_WRITE_THROUGH`、`FILE_FLAG_NO_BUFFERING`、creation disposition、truncateの効果を個別に定義するが、それらだけでCASやtorn-write排除を保証しない。現行設計に`MoveFileExW`/`ReplaceFileW`/`FILE_FLAG_WRITE_THROUGH`をdeadline CASとして扱う過剰主張はない。

## 6. DD-RRR-002 詳細評価

### 6.1 `StatusRequestV1`

architecture 5.4は次を定義する。

- `protocolVersion`: exact 1
- `mode`: `BOOT_HANDSHAKE` / `ORDINARY_RESYNC` / `PRESENTATION_RESYNC`
- `frontendBootNonce`: duplicate識別だけ。authority/identity/Keep/ACK tokenではない
- `knownControllerInstanceId`: optional last-seen hint。current bindingを上書きしない
- `knownViewRevisionDigest`: optional bounded digest。authority移送に使わない
- `sessionId`: optional status filter。別session操作権限ではない

`frontendBootNonce`のreuse、same nonce/different payload、different nonce repeated handshakeも扱われ、得られる追加影響はbinding invalidationによるavailability DoSまでである。

### 6.2 BOOT_HANDSHAKE accepted path

accepted useは初回root mount、root remount、frontend boot処理の明示的再開始、renderer復旧である。受理時に旧bindingを失効し、new CSPRNG `viewRevision`をcurrent controllerへbindする。active presentationでは旧stage authorityを失効し、deadlineを延長せずStage 1から再構築し、間に合わなければRevertする。

`page-load Started`は旧bindingを即失効し、`Finished`はtokenを自動発行せず最初のBOOTを待つ。root remountはpage-loadを伴わなくてもfrontendがBOOTを明示送信する。child remount/focus/minimize解除/event gapは`ORDINARY_RESYNC`、current presentation表示取得は`PRESENTATION_RESYNC`であり、どちらもrotate/authority transferしない。external navigationはdenyされる。

このpathは一意である。repeated BOOTからpresentation token、ACK authority、Confirm nonceは生成されない。

### 6.3 command surface

現行public commandは次の6件で全現行文書が一致する。

1. `get_display_snapshot`
2. `begin_display_change`
3. `ack_display_change_presentation`
4. `confirm_display_change`
5. `restore_display_change`
6. `get_display_change_status`

status commandはOS display settingを変更しない。`BOOT_HANDSHAKE`だけapplication security stateであるview bindingを更新するが、display mutation、Keep、Revert、stage ACKを実行しない。architecture command表とsecurity command表にはこの例外が明記されており、「OSにはread-only、application authority bindingにはstateful」と解釈が一意である。

### 6.4 判定

**DD-RRR-002: RESOLVED_WITH_EVIDENCE_CONDITIONS**

残るものはTauri/WebView2 page lifecycle ordering、React error-boundary/root remount、renderer crash、old renderer、repeated handshake、navigationのPhase 2A protocol simulationとPhase 3〜6 packaged E2E evidenceである。設計上のHigh/Medium gapではない。

## 7. DD-RRR-003 詳細評価

### 7.1 normative schema

architecture 19.2は「本節だけをnormative wire schemaの正本」と明記する。全文検索上、他の現行文書はarchitecture 19.2を参照するか、durable sequence、test case、UI/diagnostic projectionを説明するだけで、競合するfield table/state enumを正本として定義していない。`docs/tauri-review-resolution.md`に34 field/13 stateの再掲があるが、これはresolution履歴でありarchitecture 19.2だけが正本であると自己限定している。

### 7.2 canonical 34 fields

architecture 19.2には要求された34 fieldがすべて存在する。

- header/state: `recordSchemaVersion`, `recordLength`, `recordStateVersion`, `recordState`
- machine/boot/display: `machineEpoch`, `bootId`, `activeDisplayId`
- owner: `ownerSid`, `ownerLogonId`, `ownerSessionId`
- WAL link: `ownerWalPathDigest`, `ownerWalGeneration`, `ownerWalState`, `ownerTerminalDigest`
- controller: `controllerInstanceId`, `controllerProcessIdentity`
- watchdog: `watchdogInstanceId`, `watchdogProcessIdentity`
- worker: `workerInstanceId`, `workerProcessIdentity`
- binary: `binaryVersion`, `recoveryBinaryVersion`
- time: `createdTickMs`, `updatedTickMs`, `createdWallClock`, `updatedWallClock`
- operation: `operationKind`, `operationNonce`, `operationIntent`, `operationCompletion`
- terminal/error/integrity: `terminalGeneration`, `lastErrorClass`, `payloadChecksum`, `headerChecksum`

各process identityは`pid,processCreationTime,signedImageIdentity,role,processNonce`で、instance IDとは別fieldである。controller/watchdogはsame processのままinstance rotate可能で、`recordStateVersion`を進めてold instanceをfenceする。workerはone-shot processなのでsame-process instance rotateを許す理由はなく、DD-FR-002のtest条件どおりrejectを固定するのが整合的である。

raw owner WAL pathは保存せずdigestだけを持ち、digestから任意pathを復元しない。actual pathはtrusted fixed pathからderiveし、digestは照合にだけ使う。

### 7.3 canonical wire enumとprojection分離

wire enumは次の13値だけである。

`UNKNOWN`, `ACTIVE_INTENT`, `ACTIVE_PREPARED`, `ACTIVE_WATCHDOG_READY`, `ACTIVE_APPLY_ARMED`, `ACTIVE_MUTATED`, `RECOVERY_REQUIRED`, `RESTORING`, `TERMINALIZING`, `TERMINAL_CLEAN`, `MAINTENANCE_INTENT`, `MAINTENANCE_ACTIVE`, `FAILED_CLOSED`

`ACTIVE`, `PREPARING`, `CRITICAL_UNKNOWN`, `CLEAN`, `PENDING`、alias、大小文字違い、旧wire stateはdecodeしない。これらはUI/diagnostic projectionに限定される。

### 7.4 state-specific field contract

architecture 19.2はH/O/C/W/X/T/P/Q/Eのfield groupを定義し、全13 stateについてrequired、optional、forbidden、allowed previous/next、maintenance、recovery action、actor presence、WAL consistencyを一表で固定する。optional groupの片側だけ存在するrecordも拒否する。

unknown schema/enum、required欠落、forbidden field、group部分欠落、record length/checksum mismatch、owner WAL generation/state/digest mismatch、process/instance mismatchはfail closedである。old readerは解釈不能schema/stateでmaintenance/update/repair/uninstallを拒否し、new writer導入時はold schemaを読めるsigned recovery binaryをmigration完了まで保持する。schema migration途中のread uncertaintyはcommit/uninstall禁止である。

### 7.5 判定

**DD-RRR-003: RESOLVED_WITH_EVIDENCE_CONDITIONS**

残るものはexact serialization/checksum profileのpre-Phase 2A freeze、standard-user/elevated-reader DACL、全state serializer、same-process instance fencing、old installer/recovery reader、migration crashのPhase 2A/8 evidenceである。field/state名の分散という元のMedium指摘は解消している。

## 8. テスト戦略の評価

### 8.1 `DecisionJournalV1`

現行test計画は、A/B正常、片slot/両slot破損、partial/torn/short write、checksum、invalid magic/schema/length/index、generation rollback/wrap/gap、previous mismatch、same-generation conflict、flush/close/reopen/readback error、各crash point、deadline前後、authorization後I/O stall、writer hung/replacement、AV/EDR/filter、disk full、power-loss相当、outcome unknown、concurrent writer 0件を含む。

不足はDD-FR-001に対応する次の明示caseである。

- A/B双方uninitializedからのfirst baseline
- initial generation/root previousGeneration
- old session terminal A/Bからnew session baselineへ移行
- old bootId、different displayId、different owner/logon/leaseのslot
- baseline provision partial/flush/readback crash
- normal terminal reclaimとcritical terminal retention

### 8.2 BOOT_HANDSHAKE

初回mount、root remount、child remount、focus、reload、navigation、renderer crash、same/different nonce repeated BOOT、old view Stage 2 ACK、active presentation root remountを含む。`frontendBootNonce`再利用はsame nonce/byte-identicalとsame nonce/different payloadにより実質的にcoveredである。expected resultをtest IDへ明記すれば十分である。

### 8.3 MachineActorRecordV1

全13 wire state、serialize/deserialize、required/forbidden/optional-group、unknown enum/schema、length/checksum、controller/watchdog instance/process分離、old installer/recovery binary、schema migration crash、UI projection混同防止を含む。

workerはone-shotでありsame-process instance rotateを許可しないcontractと読むのが安全であるため、`workerProcessIdentity`同一で`workerInstanceId`だけ変更したrecord/frameをrejectするcaseをDD-FR-002として追加する。

## 9. 設計保証とPhase 2A evidenceの分離

設計上すでに固定されたもの:

- `KEEP_AUTHORIZED`のlinearization pointとdeadline semantics
- accepted/committed/outcome unknownの意味
- active selectorなしのA/B publicationとreadback oracle
- BOOT/ORDINARY/PRESENTATION modeとauthority transfer禁止
- MachineActorRecordのcanonical fields/states/state-specific fail-closed rules
- writer/worker/replacementのexit/lock/lease fencing floor
- Phase 1Aがdisplay/file/process mutationを行わないこと

Phase 2Aでのみ実証できるもの:

- fixed-offset A/Bのpartial/torn/short writeとstorage/sector/filter挙動
- `FlushFileBuffers`、close/reopen、power-loss相当後のsurvival/readback
- AV/EDR/share/file-lock、local filesystem/volume差、hardware cacheの影響
- alive-but-hung writerのtermination可否、process exit証明、lock reacquisition
- parent loss/Job/handle inheritance、worker quiescence、PID reuse/query denied
- heartbeat jitter/false-positive/exit-to-takeoverと`HeartbeatPolicyV1`
- Global/Local named object、exact SDDL、standard user/FUS/RDP/elevated maintenance
- BOOT_HANDSHAKE protocol simulationと後続packaged WebView lifecycle
- old reader/recovery binary、schema migration crash、same-process instance fencing

Phase 2A evidenceが失敗したsupport cellはPhase 1B No-Goである。実証前の項目を現在のHigh design gapには数えていない。

## 10. Phase 1A read-only境界

Phase 1Aのallowlist候補は次に限定されている。

- display/current/available modeのread-only列挙
- display identity、current user SID/logon/session、active console sessionのread-only取得
- OS edition/version/build/KB、clock/boot evidenceのread-only取得
- bounded sanitized JSON evidenceとread-only error処理
- Rust/`windows` crate binding compile確認

Phase 1Aから次が明示除外されている。

- `DecisionJournalV1`/operational WAL/`MachineActorRecordV1`の作成・更新
- `WriteFile`/`FlushFileBuffers`/file durability/power-loss test
- BOOT_HANDSHAKE protocol prototype
- named object/mutex/DACL/SDDL/Global/Local writable object
- watchdog/worker/process spawn/termination/heartbeat/takeover
- `CDS_TEST`/`SDC_VALIDATE`/`ChangeDisplaySettingsExW`/`SetDisplayConfig`
- temporary apply/restore/profile/registry/display mutation

DecisionJournal、BOOT_HANDSHAKE protocol model、MachineActorRecordはPhase 2Aへrouteされ、Phase 1Aへ混入していない。Phase 1A boundaryはread-onlyのままである。

## 11. Phase authorization

- **Phase 1A開始記録**: DD-FR-001修正と短い再確認後に入力へ進める。現時点ではまだ入力しない。
- **Phase 1A**: 未承認、`NOT EXECUTABLE`。開始記録の全欄freeze、exact read-only call allowlist、forbidden-call audit、roles/evidence/approval、人間のPhase-specific authorizationが別途必要。
- **Phase 2A**: 未承認。P1A/G1A closure、DD-FR-001、DD-FR-002 wire freeze、exact P2A record、fault allowlist、専用human authorizationが必要。
- **Phase 1B**: 未承認。P2A/G2A closure、exact lab cell/transition、blind recovery、out-of-band経路、人間のdisplay mutation承認が必要。
- **Phase 2B**: 未承認。P1B closureとPhase 2B専用mutation承認が必要。
- **Phase 3以降**: 未承認。

## 12. 最大の残余リスク

今回の3件に限る最大の残余riskは、alive-but-hung decision writerまたはstorage/filter stallにより、`KEEP_AUTHORIZED`後のoutcomeを長時間確定できず、writer exitもownership transferも証明できないことである。設計はこの場合に並行write/restoreせずblockedを選ぶため二重decisionを避けるが、availabilityと復元時間はPhase 2A evidence次第である。

製品全体の最大riskは引き続き、Win32 display callがdriver/kernel内でblockし、old worker quiescenceを証明できないため、競合しないrollback callを安全に開始できないことである。このriskは設計文書の追加だけでは閉じず、Phase 1B/2Bのexact support cellで一度でも成立すれば当該cellをNo-Goにする必要がある。

## 13. 最終まとめ

1. **DD-RRR-001**: `PARTIALLY_RESOLVED`。`KEEP_AUTHORIZED` linearization、accepted/committed、publication/outcome unknownは解消。baseline初期化/session rolloverにMedium 1件。
2. **DD-RRR-002**: `RESOLVED_WITH_EVIDENCE_CONDITIONS`。accepted BOOT path、ordinary resyncとの区別、authority非移送は一意。
3. **DD-RRR-003**: `RESOLVED_WITH_EVIDENCE_CONDITIONS`。architecture 19.2だけが34 fields/13 stateのwire正本。
4. **Critical件数**: 0件。
5. **High件数**: 0件。
6. **Medium件数**: 1件。
7. **`KEEP_AUTHORIZED`のlinearization pointは一意か**: Yes。decision lock内の7-step transitionだけ。
8. **accepted/committedは分離されているか**: Yes。acceptedはmemory state、committedはnew valid `KEPT_SESSION`のclose/reopen A/B readback後。
9. **DecisionJournal baseline Revertは保証されるか**: current single session内のmutation preconditionとしてはYes。first creation/session rolloverを含む完全保証はDD-FR-001修正後。
10. **two-slot publication algorithmは一意か**: Keep publicationはYes。baseline provisioning/rolloverはNo。
11. **crash結果は一意か**: Keep publication中はYes。baseline provisioning crashは追加表が必要。
12. **outcome unknown時の処理は一意か**: Yes。possible Keepを上書きせず、old writer fencing後のA/B readbackでKeep/Revert/FAILED_CLOSED。
13. **Windows APIに過剰な保証を置いていないか**: 置いていない。Flush/WriteThrough/Move/Replace/NTFSをdeadline CAS、atomicity、絶対power-loss耐久性としていない。
14. **BOOT_HANDSHAKEのaccepted pathは一意か**: Yes。initial/root remount/frontend restart/renderer recoveryの明示BOOTだけ。
15. **root remountとordinary resyncは区別されているか**: Yes。rootはBOOT、child/focus/minimize/event gapはORDINARY。
16. **MachineActorRecordV1 wire schemaは一つか**: Yes。architecture 19.2だけ。
17. **UI projectionとwire enumは分離されているか**: Yes。projection aliasをwire decodeしない。
18. **Phase 2Aで必要な実証事項**: storage/flush/power/filter、writer/worker exit/quiescence、heartbeat/takeover、DACL/FUS/RDP/maintenance、BOOT protocol、old-reader/migration。numeric wire formatはPhase 2A前にfreeze。
19. **Phase 1A境界はread-onlyのままか**: Yes。DecisionJournal/BOOT protocol/MachineActorRecord/file write/process/named object/mutationは含まない。
20. **Phase 1A開始記録へ進めるか**: DD-FR-001を修正し短い再確認を終えた後に進める。現時点ではまだ進めない。
21. **Phase 1A自体はまだ開始しないこと**: 未承認。開始記録完成と専用human authorizationが必要。
22. **Phase 2Aへはまだ進めないこと**: 未承認。P1A/G1A closure、wire freeze、専用record/approvalが必要。
23. **Phase 1B/2Bへはまだ進めないこと**: 未承認。P2A/G2AおよびP1B closureと個別mutation approvalが必要。
24. **最大の残余リスク**: alive-but-hung writer/workerまたはdriver/storage stallでquiescence/outcomeを証明できず、安全な次operationを開始できないこと。
25. **最終判定と理由**: `APPROVED_WITH_MINOR_DESIGN_CONDITIONS`。新規Critical/Highはなく、前回Highのdeadline-CAS問題は実装可能なacceptance/commit分離へ改められた。一方、固定名DecisionJournalのbaseline provisioning/session rolloverにMediumの実装分岐が残るため、設計loop終了とPhase 1A record completionの前に限定的な追補が必要である。
