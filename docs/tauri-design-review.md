# DisplayDeck Tauri設計レビュー

レビュー日: 2026-08-03  
レビュー種別: 実装担当者とは独立したシニアエンジニアレビュー  
対象: Tauri 2移行後の設計文書一式  
実施範囲: 設計レビューのみ。実装、既存設計文書の変更、依存関係追加、build/test、技術スパイク、Windows display mutationは実施していない。

## 1. 結論

**最終判定: APPROVED WITH CONDITIONS**

この判定が許可対象として評価するのは、`docs/implementation-plan.md`の **Phase 1A read-only Windows API技術スパイク** だけである。本体実装、Tauri project生成、`CDS_TEST`を含むmutation関連call、temporary apply、watchdog mutation統合、installer作成、署名、配布は承認しない。

Tauri 2 / React / TypeScript / Vite / Rust / WebView2という構成はWindows専用アプリとして現実的である。Reactを未信頼のpresentation/draft clientに限定し、Tauri Rust core、独立watchdog、one-shot workerを別process/failure domainに分ける基本設計も妥当である。旧Electron設計で指摘されたhelper単一障害、crash-consistent journal、C0/P0補償、GDI/CCD mapping、cross-process fencingは、Tauri向け設計契約として概ね引き継がれている。

一方、mutation実装へ進むには、次の6件のHigh指摘を設計へ反映する必要がある。

1. 2段階presentation ACKのfrontend→Rust呼出経路が5 commandのどこにもない。
2. `Confirming`中のRevert可否がcommand表とstate表で矛盾し、watchdog CASより先にTauri coreが勝者を決め得る。
3. watchdogだけがcrashしTauri coreが生存する場合のlive takeover protocolが未定義である。
4. sleep/hibernate、watchdog再起動、boot境界を扱うclock/deadline contractが未定義である。
5. per-user/logon-session lockだけではFast User Switching等の別session actorをfenceできると証明されていない。
6. per-machine upgrade/uninstallと各userのrecovery journal/watchdogを調停するmachine-wide maintenance contractがない。

これらはPhase 1Aのread-only queryを妨げない。しかし、Phase 2B controlled watchdog recovery、Phase 3/4のcommand/UI契約、Phase 6 mutation integration、Phase 8 installerの該当部分を開始する前には解消が必要である。

件数:

- Critical: 0件
- High: 6件
- Medium: 4件
- Low: 2件
- Question: 9件

## 2. 確認した文書

指定された文書はすべて存在し、すべて確認した。欠落文書はない。

| 文書 | 扱い | 結果 |
| --- | --- | --- |
| `AGENTS.md` | 現在のrepository gate/規範 | 確認済み |
| `docs/requirements.md` | 現行Tauri要件 | 確認済み |
| `docs/architecture.md` | 現行Tauri architecture | 確認済み |
| `docs/windows-display-research.md` | 現行Windows API調査 | 確認済み |
| `docs/ui-design.md` | 現行UI/UX設計 | 確認済み |
| `docs/security.md` | 現行security設計 | 確認済み |
| `docs/testing-strategy.md` | 現行test戦略 | 確認済み |
| `docs/implementation-plan.md` | 現行phase/gate計画 | 確認済み |
| `docs/risks-and-open-questions.md` | 現行risk/decision一覧 | 確認済み |
| `docs/design-review.md` | 過去のElectron設計review履歴 | 履歴として確認済み |
| `docs/review-resolution.md` | 過去reviewのresolution履歴 | 履歴として確認済み |
| `docs/tauri-migration.md` | Electron→Tauri移行記録 | 確認済み |
| `docs/tauri-design-review-checklist.md` | 今回のreview checklist | 確認済み |

`docs/design-review.md`と`docs/review-resolution.md`のWindows 11-only、Electron、preload、IPC、custom protocol等は2026-08-01時点の履歴であり、現在有効な設計とは判定していない。

## 3. 一次資料との照合

### 3.1 Windows API

- Microsoftは`ChangeDisplaySettingsExW`について、flag `0`をdynamic change、`CDS_TEST`を非適用test、`CDS_UPDATEREGISTRY`をUSER profile更新として区別している。現設計が初期版でprofile persistenceを使わず、`CDS_TEST`もPhase 1Aから除外する方針は妥当である。[Microsoft: ChangeDisplaySettingsExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-changedisplaysettingsexw)
- `QueryDisplayConfig`はtemporary modeのcurrent dataとpersistence databaseが異なり得ること、buffer size取得とのrace、remote/current desktop accessでの`ERROR_ACCESS_DENIED`、Windows 10/11のvirtual mode/refresh flag差を文書化している。C0/P0分離、bounded retry、remote fail-closedは妥当である。[Microsoft: QueryDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)
- `SetDisplayConfig`はsupplied pathをcurrent sessionで排他的にenableするAPIである。初期single-path設計で無条件に採用せず、GDI strategyとの比較をPhase 1Bへ残す判断は妥当である。[Microsoft: SetDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setdisplayconfig)
- GDIのinteger refreshとCCDのrational refreshの関係、driverが返すcandidate、HDR/DRR/DLDSR-like mode、C0 exact restoreの成功率は一次資料だけでは決められない。現設計がこれらを要検証に分類している点は正しい。

### 3.2 Tauri 2

- Tauri 2では、`invoke_handler`へ登録したapplication commandは既定では全window/webviewから使用可能であり、`AppManifest::commands`を使ってpermission対象にする必要がある。現設計はこの点を認識している。[Tauri: Capabilities](https://v2.tauri.app/security/capabilities/)
- Capabilityは複数に一致するとpermissionがmergeされ、capabilities directory内のfileは明示設定しない場合に自動enableされ得る。Phase 3ではcapability IDの明示allowlistが必要である。[Tauri: Capabilities](https://v2.tauri.app/security/capabilities/)
- custom application permissionはcommand allow/denyへ結び付けられる。5 commandを個別permissionにする方向は妥当である。[Tauri: Permissions](https://v2.tauri.app/security/permissions/)
- sidecar documentationは外部binaryの同梱・起動方法を示すが、parent death後の生存、Windows Job separation、Task Manager終了耐性まで保証しない。現設計がそれらをPhase 2/8のevidence gateにしている点は妥当である。[Tauri: Embedding External Binaries](https://v2.tauri.app/develop/sidecar/)

### 3.3 時刻

「monotonic」という語だけではsleep policyは決まらない。Microsoftの説明では`GetTickCount64`はsleep/hibernate時間を含む一方、`QueryUnbiasedInterruptTime`は含まない。したがって、15秒をsleep中も経過させるというtest oracleには、exact clock API、boot identity、watchdog再起動時の比較規則が必要である。[Microsoft: Windows Time](https://learn.microsoft.com/en-us/windows/win32/sysinfo/windows-time)

## 4. 設計の良い点

1. ReactがOS設定、deadline、journal、session finalityを所有しない。
2. Tauri commandがoperation-specificで、raw width/Hz、device path、Win32 flag、executable pathを受け取らない。
3. public `prepare`/`apply`を分けず、frontendからは`begin_display_change`を1 transactionとして見せる。
4. queryを1つのconsistent snapshotにまとめ、generation driftを避ける。
5. watchdogとWin32 workerを分離し、blocking callをdeadline ownerへ置かない。
6. worker identityをPIDだけでなくcreation time、image、role、nonce、operation sequenceへbindする。
7. dual-slot WAL、write-ahead GO、flush/reopen、startup decision tableが具体化されている。
8. C0、P0、Rを分離し、通常restoreをC0 exact、P0 fallbackをdegradedと扱う。
9. candidate identity、UI label、GDI apply tuple、CCD expected observationを分離する。
10. eventをhintに限定し、status commandでgeneration gapを修復する。
11. rollback failure/blocked/degradedをordinary apply errorより強く扱う。
12. scale mutation、multi-monitor mutation、DLDSR-specific handling、profile persistenceを初期版から除外する。
13. CIとphysical Windows labを明確に分け、physical evidenceなしにsupportを宣言しない。
14. all-process/watchdog lossを15秒保証と偽らず、blind recoveryとproduct decisionへ分離する。

## 5. 文書間の整合性

| 項目 | 評価 | コメント |
| --- | --- | --- |
| 採用技術 | PASS-DESIGN | Tauri 2 / React / TypeScript / Vite / Rust / WebView2で一貫 |
| 対象OS | PASS-DESIGN / Question | Windows 10/11で一貫。exact edition/build/archは未決定 |
| 初期release範囲 | PASS-DESIGN | single active path、resolution/refresh、non-persistent apply |
| scale | PASS-DESIGN | read-only observationで一貫。mutationはPhase 9 |
| multi-monitor/DLDSR/HDR | PASS-DESIGN | 初期mutation対象外で一貫 |
| Windows API | PASS-DESIGN / SPIKE | GDI hybridを第一候補、CCD applyは比較対象 |
| React/Rust責務 | PASS-DESIGN | Reactはpresentation/draft、Rust/watchdogが正本 |
| Tauri command | OPEN | 5 commandにはpresentation ACKの送信経路がない。TDR-001 |
| state transition | OPEN | `Confirming`中のRevert可否が矛盾。TDR-002 |
| watchdog | OPEN | standalone watchdog crashのlive takeoverが未定義。TDR-003 |
| deadline/sleep | OPEN | exact clock、boot identity、UI projectionが未定義。TDR-004 |
| session/fencing | OPEN | cross-logon-session coordinationが未定義。TDR-005 |
| recovery WAL | PASS-DESIGN / OPEN | crash orderingは強いがterminal cleanup/rolloverが不足。TDR-008 |
| Tauri security | PASS-DESIGN / OPEN | command restrictionは正しいがcapability auto-enableを明示denyすべき。TDR-009 |
| test | PASS-DESIGN | CIとphysical labの分離、fault injection、zero toleranceは妥当 |
| installer | OPEN | per-machine maintenanceとper-user recoveryの調停が不足。TDR-006 |
| Electron残存 | PASS-DESIGN | current文書にactive Electron architectureはない |

旧runtime用語検索では、current文書中の`Electron`、`preload`、`ipcMain`、`ipcRenderer`、`contextBridge`、`nodeIntegration`、`contextIsolation`、`macOS`等は`docs/tauri-migration.md`の移行説明または`AGENTS.md`の禁止規則に限定されている。`PowerShell`も非採用理由としてのみ残る。誤ってcurrent architectureへ残ったElectron実装契約は見つからなかった。

## 6. Tauri commandレビュー

| Command | 評価 | 引数/戻り値 | state/concurrency | 冪等性/timeout | Security |
| --- | --- | --- | --- | --- | --- |
| `get_display_snapshot` | 妥当 | 引数なしでatomic snapshotを返す | mutation中はbusy付きstatus。query分割より安全 | read-only。worker timeout上限はPhase 1で固定 | native outputも未信頼として検証 |
| `begin_display_change` | 基本妥当、補足必要 | opaque revision/monitor/modeだけ | Ready/dirty/single-flight。public prepare/applyを分けない判断は妥当 | 自動retry禁止は妥当だが、response loss後のstatus recoveryとcommand completion deadlineが不足。TDR-007 | fresh re-enumeration、membership、mapping、watchdog readinessを再検証 |
| `confirm_display_change` | 基本妥当 | sessionId/generationだけ | AwaitingConfirmationのみ | duplicateは同じterminal result。deadline延長なし | old session/generationを拒否 |
| `restore_display_change` | state矛盾あり | sessionId/generationだけ | command表はApplying以降、state表はConfirmingでstatusのみ。TDR-002 | duplicate revertは同じresultであるべき | 別session journalを指定不可 |
| `get_display_change_status` | 妥当 | optional sessionId | 常時。startup recovery/gap repairの正本 | read-only/short deadlineが必要 | raw journal/path/native dataを返さない |
| presentation ACK | **欠落** | stage/session/generation/viewRevision等が必要 | Stage 1/2だけ | duplicate ACKのidempotency、late ACK拒否が必要 | fixed operation-specific commandまたは同等のtyped private pathが必要。TDR-001 |

`prepare`と`apply`をfrontendへ分離しない判断は正しい。内部phaseはwatchdog WALで分離されており、stale prepared planをfrontendが再利用できない。

## 7. 状態遷移レビュー

Application stateとdurable watchdog stateを分け、後者をより細粒度にした構成は妥当である。Reactはstate projectionを表示するだけで、watchdog journalがmutation後のtransaction truthとなる。

| 不正操作/競合 | 現設計 | 評価 |
| --- | --- | --- |
| Applying中の再Apply | controller single-flight + OS-wide lock | PASS-DESIGN |
| Restoring中のConfirm | handler/state machineで拒否 | PASS-DESIGN |
| watchdog ready前のmutation | WAL/handshake/lock失敗でGOなし | PASS-DESIGN |
| recovery data durable前のmutation | write-ahead GO | PASS-DESIGN |
| restored/kept sessionの再operation | terminal state/generationで拒否 | PASS-DESIGN |
| stale session confirm | session/generation/epoch照合 | PASS-DESIGN |
| multiple Tauri instance | controller + mutex | 同一user/logon内はPASS-DESIGN |
| cross-logon instance | per-user/logon lockしかない | OPEN。TDR-005 |
| Keep/Revert同時要求 | watchdog CASを意図 | Tauri coreの`Confirming` validationが先に勝者を決め得る。TDR-002 |
| memory stateとjournal不一致 | generation順projection/status再同期 | PASS-DESIGN。ただしwatchdog crash takeoverはOPEN |

## 8. Watchdog failure scenarioレビュー

| # | Scenario | 設計上の結果 | 評価 |
| --- | --- | --- | --- |
| 1 | recovery data保存後、watchdog ready前にcore crash | journalはwatchdogが所有する。mutation前ならEOFを受けABORTED_NO_MUTATION。coreが独自にjournalを書かない | PASS-DESIGN |
| 2 | watchdog起動後、設定変更前にcore crash | EOFでsafe abort。preflightまでならWindows未変更 | PASS-DESIGN |
| 3 | applyの途中で一部だけ成功 | worker exit確認後fresh readbackし、expected不一致ならrollback。exit未確認ならparallel callせずblocked | PASS-DESIGN / SPIKE |
| 4 | apply後、確認画面前にcore crash | deadlineはpost-readbackから進行し、EOF/presentation failureでrollback | 設計意図は妥当。ただしACK経路はTDR-001 |
| 5 | Keep押下直後にcore crash | `KEEP_DECIDED` durableならkeep完了。durable前ならEOFがrevertを勝たせる | PASS-DESIGN。response文言はTDR-007 |
| 6 | CONFIRM受信後、KEEP_DECIDED保存前にwatchdog crash | durable stateがawaitingなら保守的にrevertすべき | live takeover actor/期限が未定義。TDR-003/TDR-004 |
| 7 | restore中にwatchdog crash | journal intentとworker identityから、旧worker quiescence後にresume | startup tableはあるがlive takeoverが未定義。TDR-003 |
| 8 | watchdog restore済み、coreはpendingのまま | status commandでlatest generation/terminalを再同期 | PASS-DESIGN |
| 9 | Windows sleep/hibernate | resume時に期限超過なら即revertが期待される | clock API次第で期限が延び得る。TDR-004 |
| 10 | Windows reboot | 15秒保証外。next launch recoveryとapproved blind P0 procedure | PASS-DESIGN / Question |
| 11 | userがcoreとwatchdogをTask Managerで終了 | all-process lossとして15秒保証外。blind procedure必須 | 境界は明記済み。product decision TDR-Q02 |
| 12 | 新versionが旧journalを読む | known compatible schemaだけ。unknown majorはmutation禁止/evidence保持 | decoder方針は妥当。ただしinstallerのall-user gateはTDR-006 |

## 9. Recovery dataレビュー

| 項目 | 評価 |
| --- | --- |
| 保存場所 | current user fixed application local-data配下の専用Recovery directory。exact API/pathはPhase 2 |
| filename | fixed dual-slot名とし、sessionId/user inputを使わない。妥当 |
| user/device scope | user/logon scoped。cross-session/machine maintenanceはTDR-005/TDR-006 |
| format/schema | canonical bounded JSON、magic/schema/minReader/length/generation/state mask/digest |
| identity | sessionId、epoch、owner nonce、operation sequence、watchdog/worker process identity |
| baseline | C0/P0/R、target/topology、support fingerprint、expected/observed tuple |
| time | createdAt、deadline evidence、wall-clock diagnostic。clock semanticsはTDR-004 |
| state/decision | durable state、Keep/Revert decision、attempt/resume state。単純なbooleanより安全 |
| atomicity | inactive slot full write、flush、close/reopen、max valid generation。rename依存を避ける案は妥当 |
| corruption | bad digest/unknown schema/invalid transitionでmutation禁止 |
| locking | mutex/epoch/identityで同一scopeをfence。cross-sessionは未解決 |
| tamper | DACL、path/reparse/hardlink検証、fresh Windows observation。same-user authenticityはQuestion |
| cleanup | terminal evidenceを保持する原則は正しいが、new session rollover/retention/secure cleanupが不足。TDR-008 |
| privacy | raw device path、EDID serial、username/profile path、nonce、journal本文を通常logへ出さない。妥当 |

Recovery dataをblindにWin32 structへcastせず、fresh target/topology/current/persisted/support fingerprintへ再検証する方針は妥当である。

## 10. Windows API/Rust評価

### Windows API

- 初期候補を`EnumDisplayDevicesW` + `EnumDisplaySettingsExW` + `ChangeDisplaySettingsExW`とし、identity/current observationを`QueryDisplayConfig` + `DisplayConfigGetDeviceInfo`で補うhybridは、single active pathのPhase 1A/1B仮説として妥当である。
- `SetDisplayConfig`を初期apply APIとして確定せず、`SDC_VALIDATE`/`SDC_APPLY`のdriver補正とfull path semanticsを比較する方針は妥当である。
- `CDS_UPDATEREGISTRY`、`SDC_SAVE_TO_DATABASE`、unsafe mode、registry直接変更を初期版で禁止する点は妥当である。
- monitor IDを永続IDにせず、snapshot-scoped evidenceに限定する点は妥当である。
- candidate tupleをwidth×height×Hzの直積で合成せず、fresh complete recordだけを使う点は妥当である。
- 59.94/60、DRR、HDR、DLDSR-like、current-not-listed、virtual/RDP、cable/power/driver resetはdocumented factではなくphysical evidenceへ残されている。

### Rust

- Microsoft `windows` crateを第一候補とし、exact version/featuresをPhase 1Aでpinする方針は妥当である。
- UTF-16、structure size、union member、buffer count/index、checked arithmetic、return code、handle lifetimeをFFI境界で検査する方針は妥当である。
- `unsafe`をWindows FFI/process boundaryへ限定し、上位へowned safe domain typeを返す方針は妥当である。
- Tauri async commandにdeadline所有権を置かず、blocking Win32 callをone-shot workerへ出す方針は妥当である。
- main/watchdog/workerが同じRust package/library codeを共有できるdirectory案であり、restore logicを別実装で複製する必要はない。Phase 3でCargo package/workspace形態を決める際も、FFI/domain/storage/protocolの共有libraryを単一正本にすべきである。
- panic/`unwrap`/`expect`方針は明示が薄い。process boundaryではpanicをprotocol EOFとして扱い、mutation後はrollback triggerとする必要があるが、現protocol fault設計で吸収可能なMedium未満の実装規約事項と評価する。

## 11. Tauri security評価

- frontend compromiseを前提に、command引数を未信頼とする構造は妥当である。
- custom commandを`AppManifest::commands`とCapability/Permissionの両方で制限する方針は妥当である。
- remote origin、external navigation、popup/new WebView、remote script/CDN、`unsafe-eval`を禁止する方針は妥当である。
- shell/process/fs/http/opener/updater permissionをfrontendへ付けず、sidecar pathをRust固定にする方針は妥当である。
- CSPはsemantic baselineとして妥当だが、selected buildのexact directivesとTauri internal IPC sourceはpackaged evidenceが必要である。
- production DevTools無効、dev URL不在、capability overlap、permission inventoryはrelease artifactで確認すべきである。
- application commandはTauri上で既定公開され得るため、`AppManifest::commands`を実際に生成することが必須である。
- capability directoryの自動enableを避ける明示設定が不足している。TDR-009。

## 12. UI/UX、初期scope、test、配布

### UI/UX

- candidate 2〜8件のdiscrete sliderとselect併用、9件以上でselect主操作という仮説は妥当で、usability test対象として明記されている。
- resolution変更時のrefresh exact保持/最小差/低い方という規則は決定的である。
- current/planned、exact/degraded/failed/blockedをtext/semanticsで区別する。
- initial Revert focus、Enterはfocused buttonだけ、Escape/closeはRevertというsafe baselineは妥当だが、人間承認が必要である。
- UIをblack-screen safety mechanismとして過大評価していない。
- recovery案内はsupport-cell固有procedureへbindする必要がある。TDR-010。

### 初期scope

初期mutation scopeは十分に削減されている。さらに削るべき候補は、機能ではなくrelease laneである。

- 最初のqualified mutation laneは1つのWindows 11 x64 exact cellに限定し、Windows 10は別cellのevidenceが揃うまでread-onlyまたはunsupported表示とする。ただしproduct target自体をWindows 10/11から変更する判断ではない。
- scale read-only observationはsafeだが、Windows Settings表示との信頼できるmappingが得られなければ`unknown/unsupported`表示だけにする。
- sliderは必須機能にせず、selectだけでも初期releaseを成立させられる。
- mock adapterはrelease機能ではなくdevelopment artifactとして扱う。TDR-012。

### Test

- React、Rust、watchdog、storage、protocol、physical Windows、packaged installerの階層分離は妥当である。
- CI runnerをphysical display evidenceと扱っていない。
- durable write/GO/call/exit/readback全境界のfault injectionが計画されている。
- black-screen test、Task Manager、sleep、cable、monitor power、RDP、virtual display、NVIDIA/AMD/Intel、DLDSR-like、WebView2、NSISをphysical gateへ分けている。
- zero-tolerance項目と反復数が定義されている。ただし50回成功は未知hardwareへの一般化ではなく、frozen cell qualificationだけを意味する。

### Distribution

- NSISをTauri first-party bundler pathとprotected per-machine installの第一候補にし、MSIをenterprise/repair/upgrade比較にする方向は妥当である。
- WebView2 bootstrapper/embedded/offline/fixed runtimeは未決定として正しく残されている。
- main/watchdog/worker/DLL/installer署名、publisher/timestamp、SmartScreen/AV、x64/arm64、runtime asInvokerはrelease gateである。
- per-machine installerが全userのactive recovery状態をどう検出・fenceするかが未定義である。TDR-006。

## 13. 指摘事項

### TDR-001 — 2段階presentation ACKを送る公開経路がない

- **ID**: TDR-001
- **重要度**: High
- **対象文書**: `docs/requirements.md`、`docs/architecture.md`、`docs/ui-design.md`、`docs/security.md`
- **対象セクション**: requirements FR-203/FR-308、architecture 5.1/5.2/10、ui-design 8.2、security 3.2/3.3
- **問題点**: 公開commandは5つ、eventはRust→Reactのhintだけと定義されているが、sequence diagramではReactからTauri coreへStage 1/Stage 2のpresentation ACKを返す。5 commandのどれにもこの副作用はなく、eventをrequestに使うことも禁止されている。
- **問題になる具体的な状況**: apply/readbackは成功し、watchdogは`AWAITING_PRESENTATION`へ入る。Reactはoverlayを表示できてもACKを送れず、watchdogは2秒で毎回rollbackする。実装者がgeneric event emitや`confirm_display_change`の意味流用で穴埋めすると、5 command allowlistとsecurity modelが崩れる。
- **推奨修正**: operation-specificな`ack_display_change_presentation`を追加するか、同等に型付けされたone-way channelを正式な6番目のsurfaceとして定義する。payloadはsessionId、generation、stage、viewRevision、bounded presentation factsに限定し、duplicate idempotency、late/stale ACK拒否、Capability/Permission、timeoutを定義する。物理可視性を証明するものではないことは維持する。
- **修正しなかった場合の影響**: confirmationへ到達できないか、実装者が未reviewのgeneric IPCを追加する。製品機能またはTauri security boundaryが成立しない。
- **技術スパイクでの確認が必要か**: Tauri mock/packaged E2Eで必要。ただし先にdesign contractを修正する。
- **実装開始を妨げる指摘か**: Phase 3/4/6を妨げる。Phase 1A read-onlyは妨げない。

### TDR-002 — Confirming中のRevert可否が矛盾し、watchdog CASを迂回し得る

- **ID**: TDR-002
- **重要度**: High
- **対象文書**: `docs/architecture.md`、`docs/requirements.md`、`docs/ui-design.md`
- **対象セクション**: architecture 5.1、6.1、11.1、requirements FR-404/FR-405、ui-design 4
- **問題点**: `restore_display_change`のcommand表はApplying以降のactive sessionで許可する一方、application state表の`Confirming`はstatusのみとする。設計原則ではKeep/Revertの勝者をwatchdog CASが決めるが、Tauri coreが先に`Confirming`へ遷移してRevertを拒否すると、coreが事実上Keepを先勝ちさせる。
- **問題になる具体的な状況**: userがKeepを押した直後、画面異常に気づいてRevertを押す。confirm requestはpipe送信前またはwatchdog durable decision前だが、core local stateだけが`Confirming`になっている。Revert handlerが拒否し、watchdogは競合requestを見ずKeepを確定する。
- **推奨修正**: durable decisionが未確定の間はRevertをwatchdogへforwardし、watchdog CASだけが勝者を決めるcontractへ統一する。あるいは安全優先でRevertが`KEEP_DECIDED` durable前なら勝つ規則を明示する。application state、command allow-state、protocol state、test oracleを同じ表へまとめる。
- **修正しなかった場合の影響**: 画面異常時の最後のRevert操作がcore state raceで失われ、危険なmodeをKeepする可能性がある。
- **技術スパイクでの確認が必要か**: Phase 2A fake-clock/CAS raceとPhase 4 mock UIで必要。
- **実装開始を妨げる指摘か**: confirm/restoreの実装とPhase 2B/6を妨げる。Phase 1Aは妨げない。

### TDR-003 — watchdog単独crash時のlive takeover protocolが未定義

- **ID**: TDR-003
- **重要度**: High
- **対象文書**: `docs/architecture.md`、`docs/testing-strategy.md`、`docs/implementation-plan.md`、`docs/risks-and-open-questions.md`
- **対象セクション**: architecture 4.5/4.6/11.3、testing-strategy 7.3/10、implementation-plan Phase 2B、risks Q02/Q07
- **問題点**: testing/planは「watchdog crash時のTauri recovery takeover」を要求するが、architectureはstartup recoveryしか定義していない。Tauri coreがwatchdog process exitを検知した後、誰がいつreplacement watchdogを起動し、mutex abandonment、epoch、private pipe、deadline、old worker quiescenceをどう引き継ぐかがない。
- **問題になる具体的な状況**: R適用後、watchdogがpanicするがTauri core/WebViewは生存する。画面はblack screenで、旧workerは既にexitしている。coreはprocess exitを観測してもlive recovery actorを起動する規則がなく、15秒後もrestore callが発行されない。
- **推奨修正**: standalone watchdog lossを(a) live takeover保証対象、(b) full watchdog lossとして保証外のどちらにするか明示する。保証対象なら、core側monitor、replacement launch deadline、mutex/epoch takeover、persisted deadline継承、old worker identity確認、second-watchdog失敗、UI statusをsequence/decision tableに追加する。保証外ならtesting-strategyのTauri takeover期待を削り、product warning/blind recovery decisionへ統一する。
- **修正しなかった場合の影響**: watchdog自身の単一障害で自動復元が消え、test oracleとproduct guaranteeも矛盾する。
- **技術スパイクでの確認が必要か**: Phase 2A/2Bで必須。先に期待動作を設計する。
- **実装開始を妨げる指摘か**: Phase 2B/6 mutationを妨げる。Phase 1A/2A no-mutation prototypeは妨げない。

### TDR-004 — clock、sleep、boot、deadline projectionの契約が不足

- **ID**: TDR-004
- **重要度**: High
- **対象文書**: `docs/requirements.md`、`docs/architecture.md`、`docs/ui-design.md`、`docs/testing-strategy.md`
- **対象セクション**: requirements FR-205/FR-307、architecture 4.6/7.2/10.1、ui-design 8.1、testing-strategy 7.2
- **問題点**: 「monotonic deadline」とだけ定義され、exact Windows clock API、sleep/hibernateを経過へ含めるか、boot identity、watchdog再起動後の比較、Reactへ渡すprojection形式がない。Rustのmonotonic値をそのままWebViewの`performance.now()`と比較することもできない。
- **問題になる具体的な状況**: trial中にPCが30秒sleepする。sleepを除外するclockならresume後も15秒Keep可能となり、testの「deadlineを延長しない」に反する。別ケースではwatchdog crash/restart後に旧boot由来またはprocess-local deadlineを誤解し、Keepを期限後に受理する。Reactがwall-clock deadlineを使えば時刻変更でcountdownが逆行する。
- **推奨修正**: Windows 10/11で採るexact clock、sleep inclusion、boot/session identity、serialized deadline evidence、takeover comparisonを定義する。UIにはwatchdog計算のbounded `remainingMs`とstatus取得generation/receipt sequenceを返し、local monotonic countdownは表示補助に限定して定期resyncする。resume通知後はauthoritative statusを取得し、期限超過ならKeepを表示上も即disabledにする。
- **修正しなかった場合の影響**: 15秒acceptance testが実装ごとに異なり、sleepやwatchdog restartで危険modeの維持時間が延びる。
- **技術スパイクでの確認が必要か**: Phase 2Aでclock API/suspend/resume/watchdog restartを実測する。
- **実装開始を妨げる指摘か**: Phase 2B/6のdeadline実装を妨げる。Phase 1Aは妨げない。

### TDR-005 — per-user/logon-session lockだけではcross-session actorをfenceできない

- **ID**: TDR-005
- **重要度**: High
- **対象文書**: `docs/requirements.md`、`docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`
- **対象セクション**: requirements FR-204/FR-501/FR-502、architecture 3/4.6、security 8.2、testing-strategy 8
- **問題点**: lockはper-user/logon-session scopeである。Fast User Switchingや複数local sessionで別scopeのwatchdogが同じphysical display stackに対して安全に独立できるかは未確認で、machine-wide conflictを防ぐ上位lock、active-console transition trigger、session change fencingがない。
- **問題になる具体的な状況**: User Aがtrial中にUser BへFast User Switchする。User B側のDisplayDeckは別mutexを取得できる。Aのdeadline rollbackとBのquery/applyが競合し、AのC0をBのactive desktopへ適用する、またはAのwatchdogがconsole accessを失ってblockedになる可能性がある。
- **推奨修正**: Phase 1Aでsession/desktopごとのAPI visibilityをread-only確認し、mutation contractとして「active local console sessionが変化したら即Revert decision」「別logon sessionのactive transactionを検知したらmutation禁止」「必要ならmachine-wide mutation mutex + per-session journal owner」の三層を設計する。session notification、namespace/DACL、service不使用時の検知可能性を明記する。
- **修正しなかった場合の影響**: cross-sessionで二重mutationまたは誤session restoreが起き、別userの画面を変更する可能性がある。
- **技術スパイクでの確認が必要か**: Phase 1A read-only classificationとPhase 2A/2B fencing testで必須。
- **実装開始を妨げる指摘か**: mutation session/fencing実装を妨げる。Phase 1Aはこの問題を観測するため進められる。

### TDR-006 — per-machine maintenanceとper-user recoveryの調停がない

- **ID**: TDR-006
- **重要度**: High
- **対象文書**: `docs/architecture.md`、`docs/security.md`、`docs/implementation-plan.md`、`docs/testing-strategy.md`、`docs/risks-and-open-questions.md`
- **対象セクション**: architecture 15、security 7/12、implementation-plan Phase 8、testing-strategy 14、risks R-D03/Q04
- **問題点**: first candidateはprotected per-machine installだが、recovery journalは各userのLocalAppData相当、transaction lockはper-user/logon-sessionである。elevated installerが全logged-on userのactive/pending/blocked journalとwatchdogをどう検出し、binary replacement/removalをfenceするかがない。
- **問題になる具体的な状況**: User AのsessionでwatchdogがRを監視中、User B/adminがupgradeを開始する。installerはBのjournalだけを見て安全と判断し、shared watchdog/worker binaryを置換または削除する。Aのtimeout時にworker起動/署名照合が失敗し、black screenからrestoreできない。
- **推奨修正**: machine-wide maintenance mutex、全session actor discovery、versioned maintenance handshake、active/critical journal中のfail-closed installer exit、in-use binary replacement rules、rollback可能なversion coexistenceを設計する。全user profileを無差別にscanする案に依存せず、running watchdog/process/sessionとmachine lockを正本にする。repair/uninstallも同じgateを使う。
- **修正しなかった場合の影響**: upgrade/uninstallがlive recovery能力を除去し、R-D03のCritical riskが実際に成立する。
- **技術スパイクでの確認が必要か**: Phase 2Aでmaintenance lock protocol、Phase 8でNSIS/MSI/Fast User Switching実機testが必要。
- **実装開始を妨げる指摘か**: Phase 8とpublic releaseを妨げる。Phase 1A〜5のread-only workは妨げない。

### TDR-007 — mutation commandのcompletion/lost-response contractが不足

- **ID**: TDR-007
- **重要度**: Medium
- **対象文書**: `docs/architecture.md`、`docs/ui-design.md`、`docs/testing-strategy.md`
- **対象セクション**: architecture 5.1/10、ui-design 4/7、testing-strategy 4
- **問題点**: `begin`がどのdurable stateでreturnするか、worker hang時にinvoke自体をいつ切り上げるか、response loss時にReactが何を表示し何回statusを取るかが完全には定義されていない。`confirm`/`restore`もACK待ちとaccepted responseを分けていない。
- **問題になる具体的な状況**: begin requestはwatchdogに受理されRが適用されたが、WebView IPC responseだけが失われる。Reactがordinary apply errorと表示してretryするとbusyになり、confirmation overlayを出さないままdeadlineまで危険modeを維持する。
- **推奨修正**: commandごとにaccepted point、return point、frontend timeout、timeout後のmandatory status recovery、terminal waitの有無を定義する。response不明は「失敗」ではなく`STATUS_REQUIRED`として扱い、beginを自動retryしない。
- **修正しなかった場合の影響**: authoritative watchdog stateとUI表示が長時間ずれ、confirmation提示またはcritical status表示が遅れる。
- **技術スパイクでの確認が必要か**: Tauri packaged IPC/fault E2Eで必要。
- **実装開始を妨げる指摘か**: Phase 3/4/6の最終command contract前に修正。Phase 1Aは妨げない。

### TDR-008 — terminal journalのrollover、retention、cleanup規則が未定義

- **ID**: TDR-008
- **重要度**: Medium
- **対象文書**: `docs/architecture.md`、`docs/security.md`、`docs/testing-strategy.md`
- **対象セクション**: architecture 7.3/11.3、security 7.2/7.3/11、testing-strategy 6
- **問題点**: terminal observationを保持しcleanupを後の安全な起動時に行う原則はあるが、fixed dual slotを次sessionへどうrolloverするか、critical terminalを何件/何日保持するか、generation exhaustion/wrap、clean terminal deletion、diagnostic exportとの順序がない。
- **問題になる具体的な状況**: `RESTORED_EXACT`後のcleanup中にcrashし、片slotが旧session terminal、片slotが新session PREPAREDになる。decoderがcross-session generationを誤って比較する、またはcritical evidenceを新transactionが上書きする。
- **推奨修正**: session/epochを含むgeneration comparison、terminal→EMPTY/new-session transition、critical stateのmutation block、retention quota、cleanup crash tableを追加する。critical/degraded/blockedは明示解決/diagnostic exportまで上書き不可とし、normal terminalだけを安全なhandshake後にreclaimする。
- **修正しなかった場合の影響**: startup recoveryが別session recordを混同するか、重要な復旧証跡を失う。
- **技術スパイクでの確認が必要か**: Phase 2A storage fault injectionで必要。
- **実装開始を妨げる指摘か**: Phase 2 storage implementationのclosure条件。Phase 1Aは妨げない。

### TDR-009 — Tauri capability fileの自動enableを明示的に封じていない

- **ID**: TDR-009
- **重要度**: Medium
- **対象文書**: `docs/security.md`、`docs/architecture.md`、`docs/testing-strategy.md`
- **対象セクション**: security 3、architecture 5.3、testing-strategy 14/15
- **問題点**: command/permission最小化は明確だが、`tauri.conf`で有効capability IDを明示列挙し、capabilities directoryの追加fileを自動enableさせない規則がない。`core:default`を使わないこともexact denyとしては書かれていない。
- **問題になる具体的な状況**: 将来plugin検証用capability fileがdirectoryに残り、production buildで自動enableされる。main windowが複数capabilityに一致してpermissionがmergeされ、不要なwindow/webview/process surfaceが増える。
- **推奨修正**: production configでcapability IDをexplicit allowlistし、single file/inventory以外をbuild failureにする。`core:default`を禁止し、必要なevent listen/unlisten等だけを列挙する。generated schemaとpackaged artifactからeffective permission graphを検査する。
- **修正しなかった場合の影響**: 文書上は最小権限でも、production artifactのeffective permissionが増える可能性がある。
- **技術スパイクでの確認が必要か**: Phase 3 selected Tauri versionでpackaged inspectionが必要。
- **実装開始を妨げる指摘か**: Phase 3 security baseline closure前に修正。Phase 1Aは妨げない。

### TDR-010 — blind recovery案内がsupport cellへbindされていない

- **ID**: TDR-010
- **重要度**: Medium
- **対象文書**: `docs/ui-design.md`、`docs/testing-strategy.md`、`docs/requirements.md`
- **対象セクション**: ui-design 2.3/3/9/10、testing-strategy 10/12、requirements FR-407
- **問題点**: UI wireframeは「再起動」「ケーブル再接続」「Windows表示設定」を一般手順として示すが、要件は各support cellで事前承認されたblind procedureだけを保証根拠にする。表示する手順のversion/support fingerprint/適用条件がない。
- **問題になる具体的な状況**: あるdriverではrebootでP0へ戻らず、sign-outまたは特定port再接続だけがqualified procedureである。generic UIがrebootを第一手順として案内し、復旧しないうえ未保存作業を失わせる。
- **推奨修正**: support manifestにprocedure ID/versionを持たせ、sanitized support fingerprintに合うapproved procedureだけを表示/同梱する。mapping不能なら「一般手順で戻る」と断定せず、support/Windows recoveryへ誘導する。画面が見えない場合に参照できるinstaller同梱/外部support手順も設計する。
- **修正しなかった場合の影響**: guarantee外の操作を公式復旧手順と誤認させ、critical failure時のsupport対応を悪化させる。
- **技術スパイクでの確認が必要か**: Phase 1B/2B/7の各cellでprocedure qualificationが必要。
- **実装開始を妨げる指摘か**: UI prototypeは妨げないが、mutation release/support copyを妨げる。

### TDR-011 — roadmap図とPhase 3前提が一致しない

- **ID**: TDR-011
- **重要度**: Low
- **対象文書**: `docs/implementation-plan.md`
- **対象セクション**: 2 roadmap、Phase 3前提Phase
- **問題点**: roadmapはPhase 2B→Phase 3の直列に見える一方、Phase 3の前提はPhase 0だけで、Phase 2Bはproduct mutation前提とする。どちらも安全に解釈できるが、phase authorizationの読み方が一致しない。
- **問題になる具体的な状況**: ownerが図だけを見てnon-mutating Tauri foundationを不必要に待つ、または本文だけを見て後続phaseも許可されたと誤解する。
- **推奨修正**: Phase 3/4はPhase 0承認後にnon-mutating workとしてparallel可能、Phase 6だけがPhase 2B必須、という意図ならroadmapへ明記する。
- **修正しなかった場合の影響**: gate運用とschedule解釈が担当者により変わる。
- **技術スパイクでの確認が必要か**: 不要。文書修正のみ。
- **実装開始を妨げる指摘か**: 直接は妨げない。human phase authorizationを常に優先する。

### TDR-012 — mock adapterはinitial release機能ではない

- **ID**: TDR-012
- **重要度**: Low
- **対象文書**: `docs/requirements.md`、`docs/architecture.md`、`docs/testing-strategy.md`
- **対象セクション**: requirements 7、architecture 14、testing-strategy 4
- **問題点**: deterministic mock adapterが「初期リリース範囲」に含まれるが、同時にproduction artifactへmock switchを残さないとする。release deliverableとdevelopment/test artifactが混在している。
- **問題になる具体的な状況**: release checklistがmock scenarioを製品featureとして数えるか、production bundleにmock選択経路を残すかで解釈が分かれる。
- **推奨修正**: mockをPhase 4 development/test artifactへ移し、initial product scopeから外す。production artifactにmock code/switchがないことだけをacceptanceに残す。
- **修正しなかった場合の影響**: 機能安全への影響は小さいが、release scopeとartifact inspectionが曖昧になる。
- **技術スパイクでの確認が必要か**: 不要。
- **実装開始を妨げる指摘か**: 妨げない。

## 14. 人間判断が必要な事項

### TDR-Q01 — session-only Keepを製品仕様として採用するか

- **ID**: TDR-Q01
- **重要度**: Question
- **対象文書/セクション**: `docs/requirements.md` 7/10、`docs/risks-and-open-questions.md` Q01
- **問題点**: Keepがrebootを越えない仕様のproduct approvalがない。
- **具体的状況**: userが永続保存と誤認する。
- **推奨修正**: 初期版はP0不変のsession-only Keepを明示承認し、UI/copy/supportへ反映する。
- **未決定の影響**: mutation release不可。
- **技術スパイク**: Phase 1B/2BでP0不変を実証。
- **実装開始を妨げるか**: mutation product integration/releaseを妨げる。Phase 1Aは妨げない。

### TDR-Q02 — 15秒保証とwatchdog/all-process lossの境界

- **ID**: TDR-Q02
- **重要度**: Question
- **対象文書/セクション**: `docs/requirements.md` FR-307/406、`docs/risks-and-open-questions.md` Q02
- **問題点**: Keep受付、Revert decision、call-start、baseline完了のどこをexternal guaranteeにするか、watchdog lossを含めるか未決定。
- **具体的状況**: 15秒後にcall開始してもdriver hangで画面は長く戻らない。
- **推奨修正**: safe proposalの「t0から15秒受付、deadline後250ms以内call-start、completionは別SLO、all-process lossは外」をownerが承認または変更する。
- **未決定の影響**: acceptance/copy/supportが確定しない。
- **技術スパイク**: Phase 2B/7で必須。
- **実装開始を妨げるか**: mutation releaseを妨げる。Phase 1Aは妨げない。

### TDR-Q03 — exact Windows 10/11 support matrix

- **ID**: TDR-Q03
- **重要度**: Question
- **対象文書/セクション**: `docs/requirements.md` FR-503/504、`docs/risks-and-open-questions.md` Q03
- **問題点**: edition/build/KB、x64/arm64、Windows 10 ESU/LTSC/consumer、GPU/display/connectionが未決定。
- **具体的状況**: EOL consumer Windows 10またはuntested arm64へmutationを配布する。
- **推奨修正**: Phase 1Aは最初のexact Windows 11 x64 physical cellをfreezeし、Windows 10は別laneとしてowner/equipmentを明示する。
- **未決定の影響**: Phase 1A機材とrelease statementが確定しない。
- **技術スパイク**: read-only Phase 1Aから必要。
- **実装開始を妨げるか**: Phase 1A開始前に対象機材決定が必要。

### TDR-Q04 — installer、署名、WebView2 distribution

- **ID**: TDR-Q04
- **重要度**: Question
- **対象文書/セクション**: `docs/security.md` 12、`docs/risks-and-open-questions.md` Q04/Q08
- **問題点**: NSIS per-machine/per-user、MSI、certificate/timestamp、WebView2 online/offline/fixed runtimeが未決定。
- **具体的状況**: unsigned sidecarがAV隔離され、rollback時だけ起動不能になる。
- **推奨修正**: protected per-machine signed baselineとoffline/revocation policyをRelease/Security/Budget ownerが決定する。
- **未決定の影響**: Phase 8/public release不可。
- **技術スパイク**: Phase 8 packaged test。
- **実装開始を妨げるか**: Phase 1Aは妨げない。

### TDR-Q05 — Enter/focus/15秒accessibility

- **ID**: TDR-Q05
- **重要度**: Question
- **対象文書/セクション**: `docs/ui-design.md` 8.3/11、`docs/risks-and-open-questions.md` Q05
- **問題点**: Revert初期focus、global Enterなし、固定15秒のacceptabilityが未承認。
- **具体的状況**: userが反射的にEnterを押し危険modeをKeepする、またはNarrator読上げ中にtimeoutする。
- **推奨修正**: safe baselineをProduct/Accessibility/Safety ownerが承認し、Narrator実機testで不成立ならdeadline/copyを再設計する。
- **未決定の影響**: UI acceptance/mutation release不可。
- **技術スパイク**: Phase 4/7。
- **実装開始を妨げるか**: mock UI探索は可能、final mutation UIは不可。

### TDR-Q06 — initial apply API strategy

- **ID**: TDR-Q06
- **重要度**: Question
- **対象文書/セクション**: `docs/windows-display-research.md` 2/4/5、`docs/risks-and-open-questions.md` Q09
- **問題点**: GDI dynamic applyかCCD supplied configかはevidence未取得。
- **具体的状況**: CCD path arrayが意図せず別pathをdisableする、またはGDI modeとCCD readbackを一意mappingできない。
- **推奨修正**: Phase 1A mapping後、Phase 1Bで1つのqualified transitionだけを比較し、副作用が少ないstrategyを選ぶ。
- **未決定の影響**: Phase 2B/6 mutation不可。
- **技術スパイク**: Phase 1A/1B必須。
- **実装開始を妨げるか**: Phase 1Aはこの判断のために進める。

### TDR-Q07 — journal authenticityのthreat boundary

- **ID**: TDR-Q07
- **重要度**: Question
- **対象文書/セクション**: `docs/security.md` 7.3、`docs/risks-and-open-questions.md` Q10
- **問題点**: ACL+digest+fresh validationにDPAPI-protected HMACを加えるか、same-user attackerをどこまで扱うか未決定。
- **具体的状況**: same-user malwareがslotを置換し、recoveryをDoSまたは別targetへ誘導する。
- **推奨修正**: arbitrary journal valueをblind applyしないfloorを維持し、HMACの価値/鍵lifecycleをPhase 2 security reviewで決定する。
- **未決定の影響**: Phase 2 closure/security claimが確定しない。
- **技術スパイク**: Phase 2A。
- **実装開始を妨げるか**: Phase 1Aは妨げない。

### TDR-Q08 — cross-session mutation policy

- **ID**: TDR-Q08
- **重要度**: Question
- **対象文書/セクション**: `docs/requirements.md` FR-204/501/502、`docs/testing-strategy.md` 8
- **問題点**: Fast User Switchingを全面unsupportedにするか、machine-wide mutex/active-console monitoringでsupportするか未決定。
- **具体的状況**: User A/Bのwatchdogが別lockを取り、同じphysical displayへ競合operationする。
- **推奨修正**: initial safe baselineはsession switchを即Revert/blocked triggerとし、同時session mutationをunsupportedにする。support拡張は別evidence後とする。
- **未決定の影響**: transaction fencing scopeが確定しない。
- **技術スパイク**: Phase 1A read-only + Phase 2A。
- **実装開始を妨げるか**: mutation implementationを妨げる。Phase 1Aは妨げない。

### TDR-Q09 — DLDSR-like diagnosticsとscale表示

- **ID**: TDR-Q09
- **重要度**: Question
- **対象文書/セクション**: `docs/ui-design.md` 5/12、`docs/risks-and-open-questions.md` Q06/Q11
- **問題点**: apply不可candidateをdiagnostic表示するか、scaleをknownとして出せるmappingがあるか未決定。
- **具体的状況**: preferred超candidateをDLDSRと誤表示する、またはDPIから125%を誤推測する。
- **推奨修正**: Phase 1A evidence前はDLDSRと命名せず、scaleはmapping不能ならunknown/unsupportedとする。
- **未決定の影響**: diagnostics UXのみ。safe mutation floorは確定済み。
- **技術スパイク**: Phase 1A/9。
- **実装開始を妨げるか**: Phase 1A/initial safe mutationを妨げない。

## 15. 技術スパイク前に必須の条件

### Phase 1A read-only開始前

1. 人間ownerが改訂設計とPhase 1Aだけを明示承認する。
2. TDR-Q03として最初のexact Windows edition/build/KB、x64、GPU/driver/display/connection、operator、evidence ownerをfreezeする。
3. call allowlistを`EnumDisplayDevicesW`、`EnumDisplaySettingsExW`、`GetDisplayConfigBufferSizes`、`QueryDisplayConfig`、`DisplayConfigGetDeviceInfo`等のread-only queryに限定する。
4. `CDS_TEST`、`ChangeDisplaySettingsExW`、`SetDisplayConfig`、registry write、display setting writeをPhase 1Aから明示除外する。
5. device path、EDID、username等のredactionとevidence保存場所を承認する。
6. TDR-005のcross-session観測項目をPhase 1A test planへ追加する。

TDR-001〜004、006はPhase 1A開始前のblockerではないが、各後続phase開始前にdesign closureが必要である。

## 16. 推奨する最初のWindows API技術スパイク

最初のspikeは、Tauri UIを持たないisolated Rust read-only processとする。

- 1つのexact Windows 11 x64、standard user、local active console、single physical active pathから開始
- `windows` crateのexact version/features/binding fidelity
- GDI adapter/monitorとCCD LUID/source/target/device pathの一意cross-map
- `ENUM_CURRENT_SETTINGS`相当C0と`ENUM_REGISTRY_SETTINGS`相当P0
- normal/raw candidateの観測。rawはdiagnosticのみ
- 59.94/60、119.88/120、source/target rational、DRR/virtual refresh
- preferred/current-not-listed、HDR/advanced color、bitsPerPel/orientationのread-only observation
- hotplug buffer retry、multiple active path、RDP、virtual display、Fast User Switchingのfail-closed classification
- standard user権限、query latency、hang傾向、process timeout
- sanitized evidence schema

Phase 1Aの中止条件:

- read-only queryがunbounded hang/system instabilityを再現する。
- target cross-mapが一意でない。
- GDI candidateからexact expected observationを推測なしに作る見込みがない。
- current baselineをallowlisted fieldで再構築できない。

## 17. 推奨するwatchdog技術スパイク

### Phase 2A: display mutationなし

- fake workerを使うwatchdog/worker別process prototype
- parent Tauri相当process kill、watchdog単独kill、worker hang/late exit
- live takeoverを採る場合のreplacement watchdog protocol
- per-user/per-session lockとmachine-wide mutation/maintenance lock比較
- dual-slot WALの全write/flush/reopen/GO/exit境界fault injection
- exact clock API、sleep/hibernate、wall-clock change、watchdog restart、boot boundary
- Keep/Revert/EOF/timeoutのsingle CASと`Confirming` race
- presentation Stage 1/2 ACK protocolのlate/duplicate/stale test
- PID reuse、process creation time/image/role/nonce、OpenProcess denied
- inherited handle allowlist、parent Job、stdout/stderr backpressure
- terminal rollover/cleanup/unknown schema

### Phase 2B: 別承認後のcontrolled recovery

- Phase 1Bでqualifiedした1 transitionだけ
- watchdog ready/WAL durable前のmutation 0件
- timeout/manual Revert/parent killのC0 exact restore
- watchdog単独crash/live takeoverまたは明示した保証外挙動
- sleep/resume直後のdeadline判定
- C0!=P0、P0 fallback degraded、persisted drift
- worker hang時にparallel restoreを出さずblocked
- out-of-band captureとapproved blind recovery

## 18. 最終まとめ

1. **Criticalの件数**: 0件。
2. **Highの件数**: 6件。
3. **Mediumの件数**: 4件。
4. **技術スパイク前に必須の修正/決定**: Phase 1Aのexact machine/call allowlist/redaction/owner承認、cross-session read-only観測追加。High 6件は該当後続phase前に設計解消。
5. **初期releaseから追加で削るべき機能**: 新たな大機能削減は不要。最初のmutation support laneを1 exact Windows 11 x64 cellへ限定し、sliderを必須にせず、mockをrelease scopeから外す。
6. **Windows実機で最初に確認すべき事項**: GDI↔CCD identity、C0/P0、fractional refresh、current-not-listed、standard user、hotplug/RDP/virtual/multiple/Fast User Switching classification。
7. **最初のWindows API spike範囲**: Phase 1A read-only queryだけ。`CDS_TEST`を含むmutation関連callは禁止。
8. **watchdog spike範囲**: まずfake worker/no-mutationでWAL、clock、crash/takeover、fencing、protocol、presentation ACKを証明し、その後別承認で1 qualified transitionだけ。
9. **Tauri設計の良い点**: React未信頼、5 operation-specific command、atomic snapshot、AppManifest/Capability/Permission、event hint/status resync、fixed signed sidecar path、Rust FFI boundary。
10. **最大の技術的risk**: driver/kernel内Win32 callが返らずold worker quiescenceを証明できないため、競合restoreを安全に開始できないこと。
11. **人間判断が必要な事項**: session-only Keep、15秒保証、Windows support matrix、installer/signing/WebView2、focus/accessibility、GDI/CCD strategy、journal authenticity、cross-session policy、diagnostic表示。
12. **Windows API技術スパイクへ進めるか**: 条件付きで進める。許可対象はPhase 1A read-onlyだけ。
13. **最終判定と理由**: **APPROVED WITH CONDITIONS**。基本architectureとfail-closed原則は実現可能で、read-only evidenceを集める準備は整っている。一方、presentation/control/state、watchdog loss、clock、cross-session、installer maintenanceにHigh gapがあり、mutationまたは本体実装の該当phaseへは進めない。

