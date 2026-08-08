# DisplayDeck Tauri設計 再々レビュー

レビュー日: 2026-08-04  
レビュー種別: DD-RR-001〜DD-RR-003修正の独立再評価  
実施範囲: 設計レビューのみ  
実施していないこと: 既存設計文書の修正、Phase記録の入力、ソース/Cargo/Tauri projectの作成、build/test、Windows API実行、display setting変更、Phase 1A/1B/2A/2Bの開始

## 1. 結論

**最終判定: NOT_APPROVED**

現行設計は前回より大幅に具体化されているが、DD-RR-002の中心であるdeadline-coupled durable commitをWindows上で成立させるexact publication algorithmがまだ定義されていない。`DurableDecisionCasV1`は満たすべき結果とNo-Go条件を列挙している一方、候補APIのどの戻り値・durability境界・`GetTickCount64` sampleを組み合わせれば「deadline前にdurable publicationが完了した場合だけvalid Keep」とできるかを定義していない。このためDD-RR-002は`PARTIALLY_RESOLVED`であり、Highが1件残る。

加えて、DD-RR-001にはroot remount/boot-handshake restartをRust coreが識別してrotateするexact経路、DD-RR-003には`MachineActorRecordV1`のfield/state名を一つのnormative schemaへ統一するMedium条件が残る。

件数:

- Critical: 0件
- High: 1件
- Medium: 2件
- Low: 0件
- Question: 0件

関連修正そのものが新たに導入したCritical/Highは0件である。現在のHigh 1件はDD-RR-002の元の安全要件を、抽象primitiveだけではまだ閉じられていないことによる。

この判定はPhase 1A、Phase 2A、Phase 1B、Phase 2Bの開始を一切承認しない。DD-RR-002の設計修正後に再レビューを行い、その後もPhase 1A開始recordの全欄記入とPhase 1A専用の人間承認が別途必要である。

## 2. 確認した文書

指定された現行文書をすべて確認した。

| 文書 | 結果 | 主な参照箇所 |
| --- | --- | --- |
| `AGENTS.md` | 確認済み | gate、non-negotiable safety constraints、research discipline |
| `docs/requirements.md` | 確認済み | FR-203/208/309/404/409〜412、AC-025〜028 |
| `docs/architecture.md` | 確認済み | 5.4、7.3/7.4、10〜11、16〜19 |
| `docs/windows-display-research.md` | 確認済み | 13 Phase 1A/1B/2A |
| `docs/ui-design.md` | 確認済み | 4、8、9 |
| `docs/security.md` | 確認済み | 3、5.3、7、8、11〜13 |
| `docs/testing-strategy.md` | 確認済み | 3.3、6、7、8、14、17 |
| `docs/implementation-plan.md` | 確認済み | 1.1、roadmap、Phase 1A/1B/2A/2B |
| `docs/risks-and-open-questions.md` | 確認済み | R-T13〜R-T19、Q02/Q07/Q10、7.1、8〜9 |
| `docs/tauri-migration.md` | 確認済み | 5、10〜15 |
| `docs/tauri-design-review-checklist.md` | 確認済み | 3、8〜15 |
| `docs/tauri-design-review.md` | 過去のTauri初回reviewとして確認済み | TDR-001〜012とQuestions |
| `docs/tauri-review-resolution.md` | 自己判定を承認根拠にせず確認済み | DD-RR-001〜009/Q01 |

`docs/tauri-design-rereview.md`はrepositoryに存在しない。この事実は`docs/tauri-review-resolution.md:5`の記録とも一致する。

`docs/design-review.md`と`docs/review-resolution.md`はElectron設計の履歴であり、今回の3件を判定するための正本にはしていない。現行文書だけで論点を追跡できたため、履歴上の結論を現在のTauri設計へ転用していない。

## 3. DD-RR別判定

| ID | 判定 | 理由 | Phase 1A blocker | Phase 2A blocker | Phase 1B blocker |
| --- | --- | --- | --- | --- | --- |
| DD-RR-001 | `RESOLVED_WITH_CONDITIONS` | lifecycle、binding、失効、duplicate/stale resultは一意。ただしroot remount/boot-handshake restartをcoreが認識するexact invoke/lifecycle経路が未指定 | No | Yes（設計明確化とprotocol model） | Yes |
| DD-RR-002 | `PARTIALLY_RESOLVED` | candidate/head分離、decision owner、crash tableは解消。deadline前durable completionだけをvalid publishにするWindows algorithmが未定義 | NoだがPhase 0 review gateを満たさない | Yes | Yes |
| DD-RR-003 | `RESOLVED_WITH_CONDITIONS` | durable順序、lock順、crash/maintenance表は一意。normative record schemaのfield/state名統一が必要 | No | Yes（schema修正とDACL/durability evidence） | Yes |
| DD-RR-004 | `VERIFIED_RESOLVED` | `P0→P1A→G1A→P2A→G2A→P1B→P2B→P3以降`で一貫し、循環なし | No | execution record/approvalのみ | Yes（G2Aまで） |
| DD-RR-005 | `RESOLVED_WITH_CONDITIONS` | candidate値とproduct値を分離し、hang/exit等を分類。`HeartbeatPolicyV1`のPhase 2A evidenceと人間承認が条件 | No | evidence blocker | Yes |
| DD-RR-006 | `RESOLVED_WITH_CONDITIONS` | same-boot clockとUI oracleは統一。bootId/Fast Startup/sleep等のPhase 2A evidenceが条件 | No | evidence blocker | Yes |
| DD-RR-007 | `RESOLVED_WITH_CONDITIONS` | operation別fenceは定義済み。exact DACL/NSIS/MSI/AV behaviorのPhase 2A/8 evidenceが条件 | No | evidence blocker | Yes |
| DD-RR-008 | `VERIFIED_RESOLVED` | Phase 1Aからnamed object/DACL/machine record/WAL/watchdogを明示除外 | No | N/A（対象作業はP2A） | No（単独では） |
| DD-RR-009 | `VERIFIED_RESOLVED` | 現行設計文書のpublic surfaceは6 commandで統一。5 command表記は過去reviewの問題説明だけ | No | No | No（単独では） |

`RESOLVED_WITH_CONDITIONS`は、design contractの安全側結果は読めるが、指定した設計明確化または後続evidenceなしに該当Phaseを開始できないという意味である。`VERIFIED_RESOLVED`も実装/evidence完了を意味しない。

## 4. 指摘事項

### DD-RRR-001 — deadline-coupled durable CASのWindows publication algorithmが未定義

- **ID**: DD-RRR-001
- **重要度**: High
- **関連するDD-RR**: DD-RR-002
- **対象文書**: `docs/architecture.md`、`docs/requirements.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`
- **対象セクション**: architecture 7.4/10.1/11.1、requirements FR-309/404、testing 7.2、implementation Phase 2A
- **問題**: `DurableDecisionCasV1`に必要な結果は定義されたが、Windows上でその結果を実現するexact API、head file layout、slot selection、atomic replacement、flush対象と順序、成功戻り値後のtick sample、recovery validation ruleが未選択である。特に`commitCompletedTickMs`を、同じatomic durable publicationにどう含めるかがない。
- **具体的な障害/race/crash状況**: watchdogはdeadline直前にdecision lockを取り、candidateを検証してheadのrename/replace/flushを開始する。I/OがAV、filter driver、disk stallでdeadlineをまたいで戻る。戻った後の`GetTickCount64`ではlateと判定できるが、valid Keep headが既にdurableになっている可能性がある。その直後、late headを無効化するRevert headを書く前にprocess/OSがcrashすると、startup actorはvalid head + exact candidateを見てKeepを採用し得る。一方、headに書く`commitCompletedTickMs`をpublication前に採ると、実際のdurable completion時刻ではなくpre-checkになる。
- **一次資料との照合**: Microsoftの[`FlushFileBuffers`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)は指定file bufferを書き出すAPIであり、clock条件付きpublication/CASを提供しない。[`MoveFileExW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw)の`MOVEFILE_WRITE_THROUGH`はreturnまでmoveをdiskへ反映する性質を記載するが、expected stateVersionとのCASやdeadline条件を同じoperationで評価しない。[`ReplaceFileW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew)はfile replacement APIだが`REPLACEFILE_WRITE_THROUGH`はnot supportedで、documented failure形も複数ある。したがって、これらを列挙するだけでは現在のprimitive contractは成立しない、というのが本レビューの推論である。
- **推奨修正**: 次のどちらかを設計文書で選び、再レビューする。
  1. 利用するexact Windows API、two-slot/head layout、同一volume条件、file/directory durability、stateVersion comparison、deadline sample位置、late-return時にvalid headが存在しないことを説明できるalgorithmを定義する。
  2. そのalgorithmがdocumented APIで構成できない場合、「durable publication完了時刻」をlinearization条件にする現在要件を、実装可能な別contractへ変更し、安全性/SLAを再承認する。

  いずれの場合も、Phase 2Aは未定義primitiveを探索して後から意味を決める場ではなく、事前に定義したalgorithmをfault/power/AV条件で検証する場にする。
- **修正しない場合の影響**: deadline後Keep、または同じcrash imageに対するKeep/Revert解釈分岐を排除できず、DD-RR-002の安全性とAC-020/026が成立しない。
- **Phase 1A blockerか**: 技術内容はNo。ただしPhase 0 review completion条件にHigh 0件が必要なため、現在のroadmap上はPhase 1A開始承認へ進めない。
- **Phase 2A blockerか**: Yes。検証対象となるexact primitive/algorithmを先に設計する必要がある。
- **Phase 1B blockerか**: Yes。
- **分類**: 設計修正。修正後にPhase 2A実証も必要。

### DD-RRR-002 — root remount/boot-handshake restartの識別経路が未指定

- **ID**: DD-RRR-002
- **重要度**: Medium
- **関連するDD-RR**: DD-RR-001
- **対象文書**: `docs/architecture.md`、`docs/requirements.md`、`docs/ui-design.md`、`docs/testing-strategy.md`
- **対象セクション**: architecture 5.1/5.4、requirements FR-207/410、UI 4、testing 3.3
- **問題**: rotate/non-rotate表はroot application restart/remountとboot-handshake restartをrotate対象にする一方、6 commandのどのrequest/accepted pointが新boot handshakeで、普通のstatus resyncとどう区別されるかを定義していない。`get_display_change_status`はoptional `sessionId`だけのread-only commandとして記載され、root-instance identityやhandshake modeを持たない。
- **具体的な障害/race/crash状況**: Stage 1 ACK後、同じdocument内でReact rootだけがunmount/remountする。page navigation/reloadがないためRustのpage-load hookだけでは検出できず、新rootが通常statusを取得して旧`viewRevision`とStage 2 bindingを受け取ると、設計が禁止するStage 1 authority transferが成立する。逆に全status callでrotateするとfocus/event-gap resyncのたびにpresentation failureとなる。
- **一次資料との照合**: Tauriの[`on_page_load`](https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html#method.on_page_load)はpage loadのStarted/Finishedを通知できるためreload/navigation検出候補にはなるが、React root remount自体はdocument loadではない。Tauriのevent listenerもpage reload/navigation時に解除されるがSPA内のcomponent lifecycleとは別である（[`Calling Rust from the Frontend`](https://v2.tauri.app/develop/calling-rust/)）。
- **推奨修正**: 6 commandを増やさずに行うなら、`get_display_change_status`へversioned boot-handshake variantとcore-issued challenge/sequenceを定義し、(a) page-load Startedで旧binding即失効、(b) Finished後のfirst handshakeで新token発行、(c) root remount時のexplicit handshake restartで旧binding失効、(d) ordinary status/focus resyncではrotateしない、というaccepted pointを明記する。frontend申告はavailability DoS以上のauthorityを与えず、active presentation中のrestartはStage 1再構築またはRevertへ一意にする。
- **修正しない場合の影響**: 実装者ごとにroot remountをreload扱い/ordinary render扱いに分け、old Stage 1 authorityの移送防止が一意にならない。
- **Phase 1A blockerか**: No。
- **Phase 2A blockerか**: Yes。protocol modelへexact handshake transitionを入れる必要がある。
- **Phase 1B blockerか**: Yes。
- **分類**: 設計修正 + Phase 3〜6 packaged WebView E2E実証。

### DD-RRR-003 — MachineActorRecordV1のnormative field/state名が分散している

- **ID**: DD-RRR-003
- **重要度**: Medium
- **関連するDD-RR**: DD-RR-003、DD-RR-007
- **対象文書**: `docs/architecture.md`、`docs/security.md`、`docs/tauri-review-resolution.md`
- **対象セクション**: architecture 18.1/19.2〜19.6、security 8.3、resolution DD-RR-003
- **問題**: architecture 19.2の必須field listは`controllerProcessIdentity`/`watchdogProcessIdentity`を持つが、architecture 18.1でmachine record projectionとされる`controllerInstanceId`を明示fieldにせず、`watchdogInstanceId`も単独fieldでない。またserialized `recordState`は`ACTIVE_INTENT`/`FAILED_CLOSED`を定義する一方、transaction orderは`ACTIVE` + `journalStateClass=PREPARING`、maintenance tableは`ACTIVE/CRITICAL_UNKNOWN`を使う。文中にalias説明はあるが、encoder/decoderが使用する一つのcanonical enum/schemaがない。
- **具体的な障害/race/crash状況**: runtime writerが`ACTIVE`を保存し、new installer readerが`ACTIVE_INTENT`だけをvalid enumとするとunknown schemaとして永久blockする。逆方向にaliasを表示classとして緩く受理すると、schemaVersionを変えずに異なる状態を同一視する。controller processが生存したまま`controllerInstanceId`だけrotateした場合、machine record projectionの更新要否も実装者判断になる。
- **推奨修正**: `MachineActorRecordV1`を一つのnormative tableにし、wire field名、type、required-by-state、hash/raw policy、enum valueを固定する。少なくとも`recordSchemaVersion`、record `stateVersion`、`recordState`、`machineEpoch`、`bootId`、journal/terminal generations、display/owner/logon/session、`controllerInstanceId`、`watchdogInstanceId`、各full process identity、binary version、WAL reference、timestamps、operation intent/completion/error/checksumを明示する。`ACTIVE/PREPARING`と`CRITICAL_UNKNOWN`はUI表示classにだけ残すか、wire enumから完全に除く。
- **修正しない場合の影響**: update/repair/uninstallは安全側にblockする可能性が高いが、version間readerの解釈差、不要な永久maintenance block、actor fence不一致を生む。
- **Phase 1A blockerか**: No。
- **Phase 2A blockerか**: Yes。schema/fault harness作成前に正本を固定する必要がある。
- **Phase 1B blockerか**: Yes。
- **分類**: 設計修正 + Phase 2A schema/DACL/crash実証。

## 5. DD-RR-001詳細評価

### 5.1 発行と保存

- 発行主体はTauri Rust coreである（requirements FR-208/410、architecture 5.4/18.1）。
- React counter、timestamp、storageからの生成/引継ぎは禁止されている。
- 128-bit以上のCSPRNG opaque tokenという必要安全特性が定義されている。特定crate/API選択は未実装事項であり、現段階ではCSPRNG・entropy・alphabet/length test oracleまである。
- raw tokenをmachine record、per-user WAL、通常log、localStorage/sessionStorageへ残さない。WALにはbinding/consumed digestだけを残す。
- diagnostic用short identifierを残す場合のexact formatは未定義だが、raw非出力floorは明確である。

### 5.2 lifecycle

architecture 5.4とrequirements FR-410は次を一意に区別する。

| Event | 設計上の結果 |
| --- | --- |
| 初回main WebView/boot handshake | `VIEW_UNBOUND`から新token発行 |
| reload | rotate、旧token即失効 |
| main-document navigation | committed navigation/reloadごとrotate。外部navigationはdeny |
| root application restart/remount | rotate |
| renderer crash/WebView recreate | rotate |
| controllerInstanceId変更 | rotate |
| presentation reconstruction | rotate、deadline延長なし |
| focus復帰、minimize/restore、ordinary render、child component remount | rotateせずstatus resync |

normative結果は明確である。ただしroot remountをcoreが認識するmechanismはDD-RRR-002の条件が残る。

### 5.3 失効とStage binding

- reload/crash/controller変更/terminal/new presentation/unrelated lease・generation changeで旧bindingを失効する。
- ACK acceptance tupleは`controllerInstanceId, viewRevision, sessionId, leaseVersion, generation, stage, presentationToken, ackNonce, presentationDeadlineTickMs, observedPayloadDigest`である。
- Stage 1成功時だけ同じviewをexact Stage 2 generation/tokenへ明示rebindする。
- Stage間でviewが変わればStage 2-only ACKを拒否し、旧Stage 1権限を継承せずpresentation failureからRevertへ進む。
- byte-identical duplicateはconsumed digestから同じresultを返す。same nonce/different payload、old stage/token/generation/leaseは`STALE_PRESENTATION_ACK`、old viewは`STALE_VIEW_INSTANCE`で区別する。

### 5.4 実装可能性と残余リスク

- initial/reload/navigationはTauri page-load lifecycle hookが実装候補になる。
- frontend identity取得はstatus/boot handshakeを使う設計意図が読めるが、exact handshake DTO/accepted pointは補足が必要である。
- Phase 2Aにprotocol model、Phase 3〜6にpackaged reload/crash/root-remount/old-renderer/focus E2Eがrouteされている点は妥当である。
- ACKはDOM/focus/bounds/accessibility stateを示すだけで、映像が物理的に見えることを証明しないという残余riskはrequirements FR-308、UI 8.2、resolution DD-RR-001に維持されている。

**DD-RR-001判定: RESOLVED_WITH_CONDITIONS**

## 6. DD-RR-002詳細評価

### 6.1 authoritative decisionとcommit条件

- watchdogがdecisionの唯一のauthoritative ownerで、Tauri coreはConfirm/Revertをforwardするだけである。
- unpublished `KEEP_COMMIT_CANDIDATE`とauthoritative `DecisionHeadV1`は分離されている。
- candidate full-write/checksum/flush/close/reopenはpreparationであり、commit pointではない。
- commit pointはdecision lock下のdeadline-aware `DecisionHeadV1(KEPT_SESSION)` publication成功と定義される。
- CAS比較にはsessionId、generation、leaseVersion、bootId、epoch、watchdogInstanceId、stateVersion、current state、deadline、Revert/terminal未決定、worker quiescence、current=R/persisted=P0、candidate schema/checksum/refが含まれる。

### 6.2 deadlineとcrash table

- Confirm request受理、decision lock取得、candidate reopenをdeadline判定点にしていない。
- timeout/manual Revert/EOF/session/presentation failureは同じdecision lock/stateVersionで排他される。
- architecture 7.4は、Confirm前、lock前、identity/deadline検証後、`KEEP_PENDING`前後、candidate partial/flush/reopen、head前後、post-commit verify、core response前、watchdog exit前を列挙する。
- unpublished candidateは無視してRevert、valid published head + exact candidateはKeep、published terminal Keep後のRevertは拒否、duplicate Confirmは同じKeep resultとなる。
- head/candidateのpartial/torn/schema/checksum/conflict、AV/share/verification failureはnormal apply errorへ落とさずcritical/No-Goとする。

### 6.3 未解消点

crash tableの望ましい結果は一意だが、その結果を生成するpublication primitiveがまだ一意でない。architecture 7.4自身もatomic replace/renameを「Phase 2Aで証明できた場合だけ」の候補に留めている。これは適切なNo-Go姿勢だが、設計resolutionとしては、deadline crossingをatomic durable operationの内部で防ぐalgorithmが未定義である。

- **flush後/DecisionHead前crash**: 現行表どおりunpublished candidateを無視してRevert。一意である。
- **DecisionHead publish後/response前crash**: exact valid headを再証明できるならKeep、duplicate Confirmも同結果。一意である。
- **deadline跨ぎ**: 期待結果はRevertだが、late-return前にvalid headがdurableにならない実装根拠がないため、現時点ではcrash-consistentと検証できない。
- **publication結果不明**: critical/No-Goとする安全分類は妥当。ただしPhase 2A実行前にexact primitiveを定義する必要がある。

**DD-RR-002判定: PARTIALLY_RESOLVED**

## 7. DD-RR-003詳細評価

### 7.1 lockとdurable順序

architecture 19.2/19.4は次を固定している。

1. machine-wide maintenance/mutation gate
2. trusted display由来のper-display mutation lock
3. owner SID/logon由来のper-user/logon recovery lock
4. machine actor record/per-user WAL file handles

終了時はowner WAL handle、user lock、display lock、machine gateの逆順で解放する。

transaction durable orderも次で一意である。

1. machine ACTIVE_INTENT相当をfull write/checksum/`FlushFileBuffers`/close/reopen/publish
2. owner WALをprovisionし`PREPARED`へ進める
3. exact generationをmachine recordへlink
4. watchdog readyを両recordへlink
5. `APPLY_GO_ARMED`をowner WALへdurable化しmachine recordへlink
6. その後にだけone-use GO/mutation
7. owner WAL terminal
8. worker/old watchdog/replacement actor quiescence
9. controller transaction/view/command authority失効
10. sole finalizerがowner terminal durabilityを再確認
11. machine `TERMINAL_CLEAN`をdurable publish
12. reverse release

machine ACTIVE durabilityより先のowner `PREPARED`と、owner terminal/quiescenceより先のmachine cleanは明示禁止される。

### 7.2 MachineActorRecordとmaintenance

recordはschema/stateVersion、machineEpoch、boot、owner/logon/session、display、lease、WAL reference/generation/state/terminal digest、controller/watchdog/worker process identity、binary、tick/wall timestamps、operation intent/completion/error/checksumを持つ。C0/P0/Rを持たず、restore authorityにならない。

maintenance/update/repair/uninstallはmachine gate取得後にrecordとreferenced owner WALをread-only照合する。unreadable、missing、unknown schema、checksum/generation/boot/owner mismatch、active/nonterminal/critical、actor/worker不明、stale/live判定不能、terminal durability不明、別owner pendingでは拒否する。old recovery reader/binaryをcompletion record durable前に削除しない。

architecture 19.5はACTIVE_INTENT前/partial/flush/reopen、PREPARED前後、watchdog ready前、APPLY_GO_ARMED前後、mutation途中/後、terminal前後、quiescence前、TERMINAL_CLEAN前後、lock release途中を列挙し、maintenance可否とowner recovery actionを一意にする。

### 7.3 条件

durable ordering自体は解消された。残る設計条件はDD-RRR-003のwire schema名統一である。exact SDDL、ProgramData/fixed directory、standard-user write、elevated read-only access、FUS/RDP、AV/share/reparse/power faultはPhase 2A/8の実証事項であり、証明不能ならmutation/maintenance No-Goである。

**DD-RR-003判定: RESOLVED_WITH_CONDITIONS**

## 8. 関連修正の評価

### 8.1 Phase順序

`docs/implementation-plan.md:78-99`は次の順で一貫する。

`P0 → P1A → G1A → P2A → G2A → P1B → P2B → P3 → P4 → P5 → P6 → P7 → P8`

P2BはP1B完了後であり、P1Bの前提に戻らない。循環依存はない。P1B前提にはP1A/G1A、P2A/G2A、durable CAS、machine/WAL order、view fence、clock、heartbeat、takeover、maintenance evidenceとexact mutation run approvalが含まれる。Phase 2A recordにはfault/crash/power-loss injection allowlistと専用人間承認が必要である。

### 8.2 heartbeat

- 250ms emission、4 misses、250ms graceful-stop等はcandidateで、runtime default/product constantではない。
- miss/suspect、IPC stall、alive-but-hung、authoritative process exit、resume/starvation、AV/EDR delay、termination deniedを分離する。
- sleep/resume、CPU load、debugger、disk stall、AV/EDR、process handle、termination、replacement launchをPhase 2A測定対象とする。
- product昇格にはfinite cells/sample、false-positive、worst-case latency、0 unproven takeover/double actor、Safety/Windows/Product owner承認を要求する。

**DD-RR-005判定: RESOLVED_WITH_CONDITIONS**

### 8.3 clockとReact countdown

- same-boot live deadlineはwatchdogが直接読む`GetTickCount64`だけを正本とする。
- wall clockはdiagnosticとboot/stale矛盾検出補助だけで、矛盾時はnew mutation拒否/active Revertのavailability gateとなる。
- bootをまたいでtickを直接比較しない。bootId stabilityはPhase 2A/physical evidence事項である。
- ReactはRust statusのbounded `remainingMs`を`performance.now()`相当で短時間だけ補間し、最低1秒ごととfocus/resume/gapでresyncする。
- UIの正値はKeep可否を保証せず、0表示もterminal Revert decisionではない。

**DD-RR-006判定: RESOLVED_WITH_CONDITIONS**

### 8.4 maintenance operation fence

maintenance、update、repair、uninstallのbegin/complete/commit/rollbackが個別行で定義され、machineEpoch、bootId、binaryVersion、terminal generation、actor/owner/nonce、intent/completion/reject code、old reader retentionを要求する。owner unavailable、WAL unreadable、actor不明、AV/file lockではfail closedである。

**DD-RR-007判定: RESOLVED_WITH_CONDITIONS**

### 8.5 command数

現行requirements、architecture、security、testing、implementation、migration、checklistは次の6 commandで一致する。

1. `get_display_snapshot`
2. `begin_display_change`
3. `ack_display_change_presentation`
4. `confirm_display_change`
5. `restore_display_change`
6. `get_display_change_status`

`docs/tauri-design-review.md`と`docs/tauri-review-resolution.md`に残る「5 command」は過去の欠落問題と修正前状態を記録する履歴であり、現行surfaceの主張ではない。

**DD-RR-009判定: VERIFIED_RESOLVED**

## 9. Phase 1A境界

### 9.1 allowlist候補

`docs/implementation-plan.md:165-173`と`docs/windows-display-research.md:329-351`は、次のread-only候補で一致する。

- `EnumDisplayDevicesW`
- `EnumDisplaySettingsExW`によるcurrent/registry/normal/raw enumeration
- `GetDisplayConfigBufferSizes`
- `QueryDisplayConfig`
- `DisplayConfigGetDeviceInfo`
- monitor identity、current process SID、logon identity、active console session ID、OS version/build/KB、boot/clockのdocumented read-only observation
- bounded JSON evidence、read-only error/timeout処理
- Rust/`windows` crate compile/binding確認

これらは候補であり、承認済みrowではない。exact DLL/API/argument/flag/timeout/output/redaction/sibling prohibition/artifact hash/approverを1 callずつfreezeする必要があり、現在approved rowは0件である。

### 9.2 明示除外

Phase 1Aから次が完全に除外されている。

- `CDS_TEST`
- `SDC_VALIDATE`
- `ChangeDisplaySettingsExW`の全call
- `SetDisplayConfig`の全call
- temporary apply、restore、display mutation
- registry/profile write、persistence
- `CreateMutexW`/`OpenMutexW`、named semaphore/event/file lock
- mutex ownership、abandoned mutex test
- `Global\\`/`Local\\` writable object
- security descriptor、DACL、SDDL、cross-session writable object
- machine gate、machine actor record、per-user WAL、lock prototype
- watchdog/worker、process spawn/termination、heartbeat、takeover
- installer/update/repair/uninstall operation
- scale/HDR/orientation/multi-monitor mutation

**DD-RR-008判定: VERIFIED_RESOLVED**

## 10. Phase 1A開始recordと承認状態

`docs/implementation-plan.md:23-60`に未記入templateが存在し、次を含む。

- Target Machine identifier
- Windows edition/version/build/installed KB
- CPU architecture
- GPU/driver
- monitor/firmware
- connection/port/dock/adapter
- current resolution/refresh
- HDR/scale
- RDP or local
- exact call allowlistとallowlist version/evidence ID
- explicit forbidden-call/static audit result
- evidence fields、field-by-field redaction、retention/access policy
- Operator、Evidence Owner、Reviewer
- execution date、evidence location
- Approver、Approval result
- Phase-specific human authorization record

現在、全欄は未決定または未承認で、approved call rowは0件である。今回の設計レビューはこれらを記入せず、Phase 1A専用承認も与えない。

さらに本レビューではHigh 1件が残るため、Phase 1A開始前に必要なものは「実施記録と人間承認だけ」にはまだなっていない。まずDD-RRR-001の設計修正と再レビューが必要である。その後も、上記recordの全欄freeze、forbidden-call audit、Phase 1A専用の人間承認が必要である。

## 11. Phase authorization

- Phase 1A: 未承認、`NOT EXECUTABLE`
- G1A: 未実施
- Phase 2A: 未承認。P1A/G1A closure、exact P2A record、fault injection allowlist、専用人間承認がない
- G2A: 未実施
- Phase 1B: 未承認。P2A/G2A closure、exact lab cell/transition、blind recovery、out-of-band経路、人間のdisplay mutation承認がない
- Phase 2B: 未承認。P1B closureとPhase 2B専用mutation承認がない
- Phase 3以降: 未承認

本レビューの最終判定、resolution文書の自己判定、Phase 1A record完成のいずれも、Phase 2A/1B/2Bの承認へ自動継承されない。

## 12. 最終まとめ

1. **DD-RR-001**: `RESOLVED_WITH_CONDITIONS`。normative lifecycle/bindingは一意だがroot remount/boot-handshake識別経路を明記する必要がある。
2. **DD-RR-002**: `PARTIALLY_RESOLVED`。crash tableは一意だがdeadline-coupled durable CASのWindows algorithmが未定義でHighが残る。
3. **DD-RR-003**: `RESOLVED_WITH_CONDITIONS`。durable順序は一意。MachineActorRecord wire schema/state名の正本化が必要。
4. **DD-RR-004〜009**: 004/008/009は`VERIFIED_RESOLVED`、005/006/007は`RESOLVED_WITH_CONDITIONS`。
5. **Critical件数**: 0件。
6. **High件数**: 1件。
7. **Medium件数**: 2件。
8. **viewRevision lifecycle**: normative結果は一意。root-remount検出/handshake accepted pointが未指定のため、そのままでは完全に一意な実装にはならない。
9. **Confirm linearization point**: 期待意味は一意だが、Windows上のdeadline-coupled durable publication方法が未定義なので、現時点でcrash-consistentと承認できない。
10. **flush後/DecisionHead前crash**: unpublished candidateを無視しRevert。これは一意。
11. **deadline跨ぎ**: 設計上の期待はRevert/Keep不可。ただしlate durable headを残さないalgorithmが未定義でありHigh。
12. **machine record/owner WAL durable順序**: ACTIVE_INTENT→PREPARED→READY→APPLY_GO_ARMED→mutation→owner terminal→quiescence/actor invalidation→TERMINAL_CLEAN→reverse releaseで一意。
13. **maintenance fail-closed**: unreadable/mismatch/unknown/active/actor不明/terminal未証明/owner unavailableで拒否する。設計上はfail closed。
14. **Phase順序**: 循環なし。P2BはP1B後。
15. **Phase 1A allowlist**: read-only candidateだけ。exact承認rowは0件。
16. **Phase 1A除外操作**: CDS/SDC mutation family、apply/restore/write、named object/DACL/machine record/WAL/watchdog/process/installer、scale/HDR/orientation mutation。
17. **Phase 1A開始前記入項目**: machine/OS/KB/CPU/GPU/driver/monitor/firmware/connection/current/HDR/scale/session、roles/date/evidence、exact API row/redaction/retention/forbidden audit/approver/result。
18. **Phase 1A開始前の人間承認**: revised design再レビューclosureと、exact recordに対するPhase 1A専用authorization。現時点はいずれも未完了。
19. **Phase 2A**: まだ進めない。
20. **Phase 1B**: まだ進めない。Phase 2Bも同様。
21. **最大の残余risk**: deadlineをまたぐfile publicationをKeepとして誤認すること。製品全体では、Win32 display callがblockしてworker quiescenceを証明できずrollback callを安全に開始できないriskも引き続き最大級である。
22. **最終判定**: `NOT_APPROVED`。DD-RR-002にHighの設計gapが残り、DD-RR-001/003にもMediumの設計明確化が必要だからである。
