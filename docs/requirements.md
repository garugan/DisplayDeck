# DisplayDeck 要件定義

最終更新: 2026-08-05  
状態: Tauri移行後の設計案。再レビュー・実装・技術スパイク未承認。

## 1. 目的

DisplayDeckは、Windowsの対話ユーザーが、接続中ディスプレイの現在値を確認し、解像度とリフレッシュレートの変更予定を作成し、最後に一括して一時適用できるデスクトップアプリケーションである。対象OSはWindows 10およびWindows 11とする。ただし、実際に変更機能を提供するedition/build/CPU/GPU/driver/display/connectionの組は、有限のsupport matrixで個別に合格したものに限る。

最重要目的は設定変更そのものではなく、映像が消えた場合、ドライバーが要求と異なる結果を返した場合、確認画面を表示できない場合、またはTauri本体が終了した場合にも、変更前の設定へ安全に戻すことである。

初期リリースでは単一アクティブモニターの解像度・リフレッシュレートだけを変更対象とする。画面拡大率は現在値を取得できる範囲で表示するが、変更しない。

## 2. 対象ユーザー

- ゲーム、映像制作、配信、プレゼンテーション等の前後で表示モードを切り替えるWindowsユーザー
- Windowsの詳細設定に詳しくなくても、列挙済み候補から安全に選択したいユーザー
- 管理者権限を常用しないローカルの対話ユーザー

初期版は、IT管理者向けのトポロジー管理、リモート管理、常駐プロファイル切替、複数モニター一括制御を対象としない。

## 3. 確定前提と用語

### 3.1 技術・製品前提

- デスクトップ基盤はTauri 2、フロントエンドはReact/TypeScript/Vite、ネイティブ処理はRustとする。
- Windows APIはRustから直接呼び出し、Microsoft `windows` crateを第一候補とする。crate versionとfeatureは技術スパイク後に固定する。
- UIはWindowsのWebView2上で動作する。
- 対象OSはWindows 10および11。その他のOS向け製品実装と切替抽象化は作らない。
- アプリは原則1ウィンドウ、非常駐とする。独立watchdogは変更トランザクションまたは起動時復旧の間だけ生存する。
- 設定ファイルはJSONを基本とする。通常設定と安全上重要な復元journalは分離する。
- 配布はTauri bundlerによるNSIS `setup.exe`を第一候補とし、MSIを比較対象とする。
- フロントエンド開発には決定的なモックデータ/API adapterを使う。モックは製品の別OS対応を意味しない。

### 3.2 用語

| 用語 | 意味 |
| --- | --- |
| 現在設定 | Rust側がWindowsから直近に取得し、必要に応じて再照合した実設定 |
| 変更予定 | React内だけに保持し、まだOSへ適用していない選択 |
| 表示モード | 解像度、refresh、色深度等、適用に必要な列挙済みtuple |
| snapshot | 1回の整合した列挙結果。revisionと短い寿命を持つ |
| change session | 1回の一時適用、確認、維持または復元をまとめる単位 |
| Tauri Rust core | command境界、UI同期、watchdog起動を担当するアプリ本体。rollback期限の最終所有者ではない |
| watchdog | Tauri本体と別processで、lock、epoch、journal、期限、親喪失、復元判断を所有する短命なRust process |
| one-shot worker | 1回のinspect/preflight/apply/readback/restoreだけを行い終了するRust process |
| C0 | trial適用直前のcurrent baseline |
| P0 | trial適用直前のpersisted baseline |
| R | 適用予定のtrial mode |
| exact rollback | current=C0かつpersisted=P0を再取得で確認できた状態 |
| degraded recovery | C0へ戻せず、同一targetの検証済みP0へ戻した状態。通常成功ではない |
| session-only keep | 現サインインセッションではRを維持するが、USER profile/registryへ保存しないこと |
| generation | operational recovery WALではdurable state更新ごとに単調増加し、command/event/statusの状態照合に使う版。`DecisionJournalV1`では別定義で、current identityに閉じたsession-local chainの`1/ROOT -> 2/Keep`だけを使う |
| leaseVersion | watchdog ownershipがreplacementへ移るたびに単調増加する版。旧watchdogをfenceする |
| bootId | 同一Windows bootであることを示す検証済み識別子。取得・相互検証不能ならmutation不可 |
| owner identity | access tokenから得るcanonical user SID、logon LUID、Windows session IDの組。frontend入力を使わない |
| `KEEP_AUTHORIZED` | watchdog decision lock内で期限内のConfirm意思を受理したone-way in-memory state。durable startup authorityでもReact successでもない |
| Confirm committed | `DecisionJournalV1`のnew valid `KEPT_SESSION` generationをflush後のA/B readbackで確認した状態。Reactへsuccessを返せる最初の点 |
| `DecisionJournalV1` | fixed headerとfixed-size slot A/Bを持ち、active selectorを持たない1 fileのterminal decision正本 |

## 4. ユースケース

### UC-01 現在設定を確認する

1. ユーザーがDisplayDeckを起動する。
2. Rust側が未完了journalを先に検査し、安全なterminal状態または復旧状態を確定する。
3. Rust側が接続中monitor、current mode、利用可能mode、取得可能なら拡大率を一つのsnapshotとして返す。
4. Reactが現在値を表示する。取得不能値は推測せず「不明」または「未対応」とする。

### UC-02 変更予定を作成する

1. ユーザーが対象monitorを確認する。
2. 解像度を候補indexで選ぶ。
3. UIがその解像度に属するrefresh候補だけを再構成する。
4. ユーザーがrefreshを選ぶ。
5. 現在値と変更予定を比較する。
6. Applyを押すまではTauri commandを含め、OS変更操作を一切開始しない。

### UC-03 一時適用して維持する

1. ユーザーが「設定を適用」を押す。
2. Rust coreがsnapshot revisionとopaque mode tokenを検証し、watchdogを固定pathから起動する。
3. watchdogがmachine/display/user lockを順に得てstale recoveryと旧workerを検査し、`MachineActorRecordV1`をwire `ACTIVE_INTENT`としてdurable化する。read-only capture後、architecture 7.4 `ProvisionCurrentDecisionBaselineV1`でcurrent session用`REVERT_REQUIRED` rootをflush/reopen検証し、owner WAL `DECISION_BASELINE_PROVISIONED`とmachine `ACTIVE_INTENT`へexact linkした後だけC0/P0/Rを`PREPARED`へ進め、machine recordを`ACTIVE_PREPARED`へlinkする。
4. watchdogがone-shot workerでpreflightし、成功時だけ別workerでRを一時適用する。
5. 別workerのreadbackがexpected observationと一意に一致した時点を`t0`としてdurable化し、15秒の確認期限を開始する。
6. Reactは専用commandでStage 1（Revert操作可能）とStage 2（確認UI全体が操作可能）のpresentation ACKを返す。両ACKが`t0+2秒`以内かつ残り12秒以上で成立しなければ復元する。
7. Reactの確認画面はwatchdog statusの`remainingMs`を表示するだけで、期限の正本にもConfirm可否のoracleにもならない。watchdogはrequest受理時に`GetTickCount64`を読み直す。
8. current=R/persisted=P0のfresh readback後、watchdogはdecision lock内でidentity、`AWAITING_CONFIRMATION`、Revert未勝利、fresh tickを検証し、`tick <= confirmationDeadlineTickMs`なら`KEEP_AUTHORIZED`へlinearizeする。ここまでがConfirm acceptedで、15秒はこの受理期限でありdisk flush完了期限ではない。
9. `KEEP_AUTHORIZED`後はordinary Revert/timeoutを開始せず、`DecisionJournalV1`のcurrent root baselineと反対側のfixed slotへ`KEPT_SESSION`をfull-write、flush、close/reopenし、current identity chainだけのnew generationを確認した時点だけConfirm committedとする。Reactへsuccessを返すのはcommit後だけである。actor lossやoutcome unknownはjournal readbackでKeep/Revert/FAILED_CLOSEDを決める。

### UC-04 自動または手動で復元する

次の場合、watchdogがC0への復元を開始する。

- 15秒の期限までに維持が受理されない。
- ユーザーが「元に戻す」またはEscapeを押す。
- 確認画面を提示・focus・同期できない。
- Tauri coreまたはWebViewが終了し、private control channelが切断される。
- apply/readback/confirm/finalizeが失敗または不一致になる。
- watchdogがin-flight workerの終了を確認した後、operation timeoutを処理する。
- watchdog単独終了をTauri coreが検出し、replacement watchdogがold actor/workerをfenceしてtakeoverする。
- active console/logon sessionが変化する、別interactive/RDP sessionが現れる、またはowner identityを再検証できない。

旧workerの終了を証明できない場合は並行Win32 callを行わず、`RECOVERY_BLOCKED_BY_INFLIGHT_CALL`とする。復元後はcurrentとpersistedを別々にreadbackする。

### UC-05 変更予定をリセットする

「リセット」はReactのdraftを最新snapshotの現在値へ戻すだけであり、OSを変更しない。transaction中は無効とし、確認待ちでは「元に戻す」を使う。

### UC-06 再起動時に未完了sessionを検出する

アプリ起動時、Rust側は通常列挙より先にrecovery journalを検査する。live actorの消滅を証明し、decision tableで一意な処理だけを行う。確認待ちは再開せず、安全側に復元する。未知schema、破損、target ambiguityでは推測変更しない。

## 5. 機能要件

### 5.1 列挙と候補

| ID | 要件 |
| --- | --- |
| FR-001 | 起動時と明示refresh時に、active display path、monitor識別情報、current modeを一つの整合snapshotとして取得する。 |
| FR-002 | monitor名、primary状態、session-scoped opaque tokenを返す。friendly name、device path、EDID値は未信頼入力として上限・文字列を検査する。 |
| FR-003 | `EnumDisplaySettingsEx`等から得た完全なmode recordだけを候補にし、width/height/Hzの直積を合成しない。 |
| FR-004 | candidate identity、UI label、GDI apply tuple、CCD expected observationを分離する。`canApply=true`にはexactly oneの完全なexpected observationと、承認済みread-only導出規則またはexact support fingerprintにboundしたmutation qualification evidenceを要求する。 |
| FR-005 | 59.94/60等は有理数として区別し、整数丸めや一般epsilonで同一視しない。曖昧なcandidateは表示用診断にできてもselection tokenを発行しない。 |
| FR-006 | snapshotにrevision、capture時刻、active path数、support fingerprintを持たせ、apply直前に再列挙してstaleなら変更しない。 |
| FR-007 | current scaleを公開APIで信頼して取得できる場合だけ表示し、unknown/unsupportedを区別する。scale取得失敗だけで安全なmode applyを禁止しない。 |

### 5.2 選択と表示

| ID | 要件 |
| --- | --- |
| FR-101 | 解像度とrefreshは候補配列indexを選ぶ段階式UIとし、連続値を送らない。候補が多い場合はselectを主操作にできる。 |
| FR-102 | 解像度変更時、直前refreshが残れば保持し、なければ最小差、同差なら低い候補を選び、自動変更を通知する。 |
| FR-103 | 現在値と変更予定を見出し・text・差分markerで区別し、色だけに依存しない。 |
| FR-104 | 初期版のscaleはread-only status rowとし、slider/select/apply payloadへ含めない。 |
| FR-105 | current-not-listed modeは現在値として表示できるが、exact restoreとmappingを証明できなければapplyを無効にする。 |
| FR-106 | transaction中はmonitor/resolution/refresh/refresh/reset/applyを無効化し、二重操作を防ぐ。 |

### 5.3 Tauri commandと変更session

| ID | 要件 |
| --- | --- |
| FR-201 | Reactがmutation/ACKへ渡せるのはopaque monitor/mode token、snapshot revision、sessionId、generation、watchdog発行operation token/nonce、presentation stage、bounded viewRevisionだけとする。`StatusRequestV1`に限りfrontend-generated `frontendBootNonce`と最後に観測したcontroller/view digestをnon-authoritative hintとしてechoできる。device/exe/raw mode/Win32 flag/deadline/recovery path/SID/bootIdを受け取らず、known値でcore bindingを上書きしない。 |
| FR-202 | Rust側はcommandごとにwindow/origin、exact input shape、length/range、state、revision、session、最新列挙membershipを検証する。 |
| FR-203 | 公開commandは`get_display_snapshot`、`begin_display_change`、`ack_display_change_presentation`、`confirm_display_change`、`restore_display_change`、`get_display_change_status`の6つだけとする。presentation ACKはevent/channelやConfirmの意味流用ではなく、専用のtyped invoke commandとする。分離したpublic prepare/applyは採用しない。 |
| FR-204 | 初期版は端末全体でactive change sessionを1件に制限する。machine-wide maintenance/mutation gate、trusted target由来のper-display lock、per-user/logon recovery lock、Rust state machineの順で二重起動・別session mutationを拒否する。 |
| FR-205 | 各change sessionは128-bit以上のrandom sessionId、target displayId、C0/P0/R、createdAt、deadline evidence、state、schemaVersion、bootId、owner SID/logon LUID/Windows session ID、controllerInstanceId、watchdogInstanceId、epoch、leaseVersion、generation、decision `stateVersion`、owner nonce、watchdog/worker process identity、confirmation/restoration resultを持つ。 |
| FR-206 | presentation ACK、confirm、restore、status、takeover、cleanupはoperationごとに必要なsessionId、generation、leaseVersion、bootId、owner identity、displayId、actor instance、one-use nonceを照合する。stale/cross-user/cross-boot/wrong-display operationを必ず拒否する。 |
| FR-207 | Tauri eventはsanitized state hintに限定し、authoritative resultにはしない。6 commandを増やさず`get_display_change_status(StatusRequestV1)`に`BOOT_HANDSHAKE/ORDINARY_RESYNC/PRESENTATION_RESYNC`を設ける。初回root mount、root remount、frontend boot再開始、renderer復旧は`BOOT_HANDSHAKE`を明示送信し、受理時に旧bindingを失効してnew `viewRevision`を発行する。focus/minimize解除/event gap/child remountは`ORDINARY_RESYNC`でrotateしない。`frontendBootNonce`はduplicate識別だけでauthorityにしない。 |
| FR-208 | `ack_display_change_presentation`のpayloadは`{sessionId,generation,leaseVersion,presentationToken,stage,viewRevision,ackNonce}`に限定する。`viewRevision`はcoreがCSPRNGで発行する128-bit以上のopaque view-instance tokenであり、React生成counterではない。watchdogは各stage tokenを1回だけ消費して次stageでrotateする。Rust core/watchdogはconsumed ACKのbounded digest/resultを保持し、同一payload/nonceの再送だけはgeneration前進後も同じ結果を返す。異なるpayload、旧token、旧generation/lease/stageからの新規送信は`STALE_PRESENTATION_ACK`、current core viewと一致しない`viewRevision`は`STALE_VIEW_INSTANCE`とする。 |

### 5.4 適用トランザクション

| ID | 要件 |
| --- | --- |
| FR-301 | watchdog起動、handshake、OS-wide lock、epoch、operational dual-slot WAL provision、architecture 7.4のcurrent-session `DecisionJournalV1(REVERT_REQUIRED)` root durable write/flush/close/reopen readback、WAL/machine linkの全てが成功するまでdisplay mutationを行わない。baseline provisioning中のmutation callは必ず0件である。 |
| FR-302 | C0/P0、allowlisted field mask、target/topology identity、R、expected observation、support fingerprint、active worker identityをwrite-aheadで保存する。pointer、raw native buffer、process handle、driver-private bytesは保存しない。 |
| FR-303 | 各operationは「GO待機worker spawn → identity/intent durable化 → one-use GO → process exit確認 → result/readback durable化」の順にする。watchdog自身はblocking display APIを呼ばない。 |
| FR-304 | preflightは列挙済み完全tupleに対して行う。初期GDI案では`CDS_TEST`を候補とし、実API/flagはPhase 1で確定する。 |
| FR-305 | temporary applyはprofile/registryへ保存しない。`CDS_UPDATEREGISTRY`、`SDC_SAVE_TO_DATABASE`、unsafe mode有効化は初期版で使用しない。 |
| FR-306 | apply後はfresh GDI/CCD readbackをexpected observationとexact比較し、一意に一致しなければ確認画面へ進まず復元する。 |
| FR-307 | live deadlineの正本はwatchdogが直接読むWin32 `GetTickCount64`とする。post-apply exact readbackをdurable化したtickを`t0`、confirmation deadlineを`t0+15,000ms`とし、wall clock変更で延長しない。この15秒はdecision lock内でauthoritative watchdogがidentity/state/Revert未勝利を検証し`KEEP_AUTHORIZED`へ入るための期限で、durable slot write/flush/readback完了期限ではない。deadline後に新たなauthorizationは不可で、未authorizedならworker quiescence後にRevertを開始する。deadline-to-call-start値はPhase 2A/2B evidenceと人間SLA承認で固定する。sleep/hibernate後はfresh tickで同じentry ruleを適用する。 |
| FR-308 | 確認UIはmutation前にpre-render可能にする。watchdogは`PRESENTING_STAGE1`でRevert enabled/focusedのACKを、続く`PRESENTING_STAGE2`でKeep/Revert、bounds、visibility、focus、accessible render完了のACKを受ける。両方が`min(t0+2,000ms, confirmationDeadline-12,000ms)`までにdurable化されなければ即時復元する。ACKは物理可視性の証明でもdeadline開始点でもない。 |
| FR-309 | fresh readbackでcurrent=R/persisted=P0を確認した後、watchdogはdecision lock下でexact identity、`AWAITING_CONFIRMATION`、Revert未勝利、`GetTickCount64 <= confirmationDeadlineTickMs`を検証し、同じlock内でone-way in-memory `KEEP_AUTHORIZED`へ遷移する。これがConfirm acceptedだがReact successではない。`DecisionJournalV1`のcurrent root baselineと反対側のfixed slotへnew `KEPT_SESSION` generationをfull-write/checksum/`FlushFileBuffers`/close/reopenし、current identity chainの両slot readbackで確認した時点がConfirm committedで、ここでのみsuccessを返す。foreign generationは比較せず、baselineはreadback完了まで保持する。authorization後はdeadlineを再検査せずordinary Revert/timeoutを開始しない。actor loss/outcome unknownではjournal再読込を正本とし、valid terminal KeepならKeep、なければRevert、unreadable/conflictならFAILED_CLOSEDとする。 |
| FR-310 | watchdog開始後にpreflight/applyが失敗した場合も、durable stateとfresh readbackから「無変更」「復元済み」「復元未確認」を区別してterminal化する。 |
| FR-311 | session creationからapply GOまでのpre-apply leaseは30秒、`KEEP_AUTHORIZED` entryを含むlive decisionを開始できるmaximum protection deadlineはcreationから60秒とし延長しない。未mutationで超過すれば`ABORTED_NO_MUTATION`、mutation後かつ未authorizedならRevertを選ぶ。authorization後のcommit I/Oはconfirmation/maximum deadlineで逆転させずFR-409のwriter-hang fencingで処理する。in-flight actorがquiescentでなければ時間を偽ってterminal化せずblockedにする。 |

### 5.5 ロールバックと復旧

| ID | 要件 |
| --- | --- |
| FR-401 | 通常rollbackは同一targetへC0をdynamic exact restoreし、current=C0かつpersisted=P0を確認する。 |
| FR-402 | C0 exact restore失敗時、同一targetとcaptured P0を再検証できる場合だけP0への動的fallbackを1回試せる。結果は`RESTORED_DEGRADED`とする。 |
| FR-403 | target/topologyが曖昧、persistedがdrift、journalが未知・破損、旧workerがquiescentでない場合は、別monitorや任意modeを推測適用しない。 |
| FR-404 | parent EOF、manual Revert、timeout、presentation/session failure、apply verification failure、Confirmはwatchdogの単一decision lockへまとめる。`KEEP_AUTHORIZED`より前は先にlinearizeしたRevert classが勝ち、authorization後はordinary Revert classを開始しない。同じsessionでKeep/Revert terminalを二重成功させない。write/flush/readback failure時はpossible KeepをRevertで上書きせず、`DecisionJournalV1` A/B readbackだけでoutcomeを決める。 |
| FR-405 | rollbackは同一sessionに対して冪等とする。Confirm arbitration中のrestoreはwatchdogへforwardし`KEEP_AUTHORIZED`前なら競合可能、authorization後はcommit status待ちとして新しいrestoreを開始しない。復元開始後のconfirm、別sessionのrestore、`KEPT_SESSION`を含むterminal後の新operationは拒否する。 |
| FR-406 | watchdog単独lossはTauri core生存時のlive takeover保証対象とする。Tauri coreとwatchdogの同時loss、all-process loss、OS crash、reboot、power lossは、常駐機構を採用しない初期版では15秒保証外とする。次回起動recoveryは行うが代替保証とは呼ばない。 |
| FR-407 | 各admitted support cellで、全process loss後にも事前承認済みのblind reboot/sign-out/physical procedureからP0へ戻れることを反復実証する。成立しないcellはrelease対象外とする。 |
| FR-408 | recovery失敗、degraded、blocked、unknownはjournalとdiagnostic IDを保持し、通常apply errorより高いseverityで案内する。 |
| FR-409 | Tauri coreはwatchdog process handleとprivate heartbeatを承認済み`HeartbeatPolicyV1`で監視する。250ms等はPhase 2A候補で製品定数ではない。miss、suspect、authoritative exit、alive-but-hung、IPC stall、commit I/O stall、resume、starvation、security-product delay、termination/access failureを別状態として記録する。`KEEP_AUTHORIZED`後のwriterが停止しても、旧process exit、machine/display/user/journal-writer lock取得、worker quiescence、lease/instance fencingまでreplacementはjournalへ書かない。exit後はA/B readbackでvalid Keep/No Keep/unreadableを判定する。未承認policyまたはquiescence不能はcritical/blockedとし並行actorを起動しない。 |
| FR-410 | coreはpage-load Startedで旧bindingを即時失効し、Finished後の`BOOT_HANDSHAKE`を待つ。初回/root remount/frontend boot再開始/renderer復旧のhandshake受理、controller変更、presentation再構築で`viewRevision`をrotateする。child remount、focus、minimize/restore、通常renderと`ORDINARY_RESYNC/PRESENTATION_RESYNC`ではrotateしない。active presentation中のboot handshakeは旧stage authorityを失効しStage 1から再構築、期限上不可能ならRevertとし、ACK authorityを移送しない。 |
| FR-411 | `MachineActorRecordV1`は`docs/architecture.md` 19.2のcanonical field、13 wire state、state-specific required/optional/forbidden/transition表だけをwire正本とする。machine gate保持中に`ACTIVE_INTENT`をdurable write/flush/close/reopen検証してからowner WALを`PREPARED`へ進め`ACTIVE_PREPARED`へlinkする。owner WAL terminal、worker quiescence、transaction actor absence後に`TERMINALIZING -> TERMINAL_CLEAN`としlocksをreverse releaseする。unknown schema/enum、field violation、checksum、record/WAL不一致ではfail closedにする。 |
| FR-412 | wall clockは診断とstale/cross-boot検出の補助に限る。同一bootのpresentation/confirmation/session-lifetime判定はwatchdogが直接読む`GetTickCount64`だけを使い、wall clockまたはReactの`remainingMs`から期限を再構築・延長しない。boot identityが不一致または証明不能ならKeepを拒否してrecoveryを選ぶ。 |
| FR-413 | `ProvisionCurrentDecisionBaselineV1`はcurrent identityの`generation=1,previousGeneration=0(ROOT),stateVersion=1,decision=REVERT_REQUIRED`をsession-local chainのrootとし、file-global generationを使わない。session/boot/display/owner/logon/decision-chain leaseが異なるslotはcurrent chainへ接続しない。current baselineをreadbackで証明できない、または結果不明の再読込みでvalid rootがない場合はsession preparation failure/`FAILED_CLOSED`とし、そのsessionでmutationを一度も行わない。 |
| FR-414 | old session slotをreclaimできるのはmachine/display/user/journal-writer lock、old actor absence、linked operational WAL normal terminal、prior machine `TERMINAL_CLEAN`、exact terminal digest、identity readability、retentionを証明し、durable reclaim intentをpublishした場合だけである。critical/blocked/unknown/corrupt/unsupported evidenceを自動reclaimせず、active/unresolved fileをtruncate/delete/recreateしない。Keepはbaselineと別slotへpublishし、readback完了までprior valid baselineを保持する。terminal後cleanup/reclaimは別のdurable operationである。 |

### 5.6 対象環境

| ID | 要件 |
| --- | --- |
| FR-501 | 初期版は、supportedなlocal interactive active console logonが正確に1件、RDP/remote/別interactive logonが0件、active display pathが正確に1本のときだけmutationを許可する。複数monitor/user sessionは表示できても変更を拒否する。 |
| FR-502 | RDP、remote session、Fast User Switching後の非active console、virtual display、head-mounted/specialized display、classification不能環境はmutation不可とする。`KEEP_AUTHORIZED`前のconsole/logon changeは即Revert trigger、authorization後は旧sessionのfrontend authorityをfenceしてjournal commit/recoveryだけを続ける。 |
| FR-503 | Windows 10/11のexact edition/build/KB、x64/arm64、GPU/driver/display/connectionをrelease manifestへ列挙する。範囲表現や「latest」だけではsupportを公称しない。 |
| FR-504 | Windows 10が2025-10-14に通常support終了済みであることを踏まえ、ESU/LTSC/unsupported consumer editionの配布方針をrelease前に人間が決定する。 |
| FR-505 | installer/update/repair/uninstallはmachine-wide maintenance gateを取得し、machine coordination recordと全signed watchdog/controller processを確認する。active/pending/critical/unknown recordが1件でもあればbinary replacement/removalを禁止する。一般userおよび別admin userはowner SIDのper-user journalを復元せず、owner sessionでのrecoveryまたは別承認support procedureを要求する。 |

## 6. 非機能要件

### 6.1 安全性・信頼性

- 安全性を機能数、速度、利便性より優先する。
- watchdog、WAL、fencing、worker separationの実機evidenceがない環境ではmutation機能を提供しない。
- APIのsuccess codeだけを成功根拠にせず、apply/keep/restoreの後にreadbackする。
- transactionの最終状態はReactやTauri eventではなくwatchdog journalを正本とする。
- rollback不能の可能性が見つかったsupport cellは、再試行で上書きせずreleaseから除外する。

### 6.2 セキュリティ

- Reactへshell/process/filesystem/network/openerの任意権限を与えない。
- Tauri Capabilities/Permissionsをmain windowと最小commandに限定し、外部originを許可しない。
- 外部Webページ、remote script、CDN、任意navigationを読み込まない。
- watchdogはpackaged fixed pathからshellなしで起動し、署名・image identity・引数・private handleを検証する。
- Rust `unsafe`はWindows FFI境界へ限定し、安全なwrapperより上へ漏らさない。
- 詳細は`docs/security.md`を正本とする。

### 6.3 互換性・保守性

- current designはWindows専用とし、別OS adapterを先行実装しない。
- React domain type、Tauri DTO、Rust domain model、Windows FFI structを別層にする。
- Windows 10/11差とcrate version差をcompile/runtimeで明示し、未検証flagを送らない。
- recovery schema、command DTO、watchdog protocolはversionedとし、未知major versionではfail closedにする。

### 6.4 性能・オフライン動作

- 起動後2秒以内にloadingを表示し、通常5秒以内にsnapshotまたは明確なerrorを表示することを目標とする。確定値はPhase 1/3で測る。
- 通常利用はネットワーク不要とする。WebView2 installerの取得方式は配布decisionであり、製品runtimeのnetwork要件にはしない。
- control frame、JSON、候補数、文字列、timeoutには具体上限を持たせる。

### 6.5 アクセシビリティ

- 全操作をkeyboardで完了できる。
- 残り時間、現在/予定、error severityを色だけで表さない。
- slider/selectにaccessible name、value text、候補位置を提供する。
- 200% zoom、high contrast、Narrator、focus orderをWindows実機で検証する。

## 7. 初期リリース範囲

### 含める

- Windows 10/11専用。ただしexact release cellsは承認済みmanifestに限定
- local console、single active path、single window
- monitor/current resolution/current refreshの取得
- 利用可能mode取得とexact mapping済みcandidateだけの選択
- resolutionに応じたrefresh候補更新、現在/予定比較、reset
- apply前のfresh validation
- resolution/refreshのnon-persistent temporary apply
- C0/P0/Rのversioned operational dual-slot WALとfixed-slot A/B `DecisionJournalV1`
- independent Rust watchdog、one-shot worker、machine-wide maintenance/mutation gate、per-display/per-user lock、epoch/leaseVersion/generation/actor fencing
- 15秒の確認、専用commandによる2-stage presentation ACK、manual revert、timeout、Tauri crash、watchdog単独crash時のlive takeoverと自動復元
- mutation時は単一local active-console user/logonだけを許可し、Fast User Switching/RDP/second interactive logonをfail closedにする
- 二重操作防止、startup pending recovery、severity別error
- scaleのread-only observation
- フロントエンド用の決定的mock data/API adapter

### 条件付き

- session-only Keep: DDR-Q01の人間承認が必要
- 各Windows 10/11 release cell: exact matrix、blind recovery、zero-tolerance testの合格が必要
- current-not-listed candidate: exact restore、expected observation、qualificationの全てを満たす場合のみ

## 8. 初期リリース対象外

- 画面拡大率の変更、custom scale、registryによるDPI変更
- display modeのUSER profile/registry persistence、再起動を越えるKeep
- 複数monitor mutation、clone/extend/position/primary変更
- DLDSR/DSR固有識別・vendor API・分類不能modeの適用
- HDR、color depth、color space、orientation、VRR/DRRの意図的変更
- preset保存、OBS/game起動、tray、autostart、auto-update
- remote session、virtual display、service operation
- 同時複数interactive user sessionでのmutation、別user journalのrestore、session switch後のconfirmation継続
- watchdogを失った後も15秒で戻すservice/scheduled task等の常駐回復機構
- telemetry、cloud同期、外部アプリ向け汎用IPC/CLI
- Windows以外のOSへの対応とそのためのplatform switching abstraction

## 9. 受け入れ条件

| ID | 条件 |
| --- | --- |
| AC-001 | 各admitted Windows 10/11実機で、Windows表示と一致するcurrent resolution/refreshと一意なmonitor identityを取得する。 |
| AC-002 | resolution変更に応じrefresh候補が決定的に更新され、slider/select/resetだけではWindows APIのmutation callが0件である。 |
| AC-003 | currentとplannedがtext、visual、支援技術で区別できる。 |
| AC-004 | no change、stale、busy、multi-path、remote、ambiguous candidateではApplyが無効またはRust側で安全に拒否される。 |
| AC-005 | baseline/journal/watchdog/lock/epoch/worker identityがdurableかつverifiedになる前にmutation callが発行されない。 |
| AC-006 | apply後readbackがexactly one expected observationと一致しない場合、成功/確認待ちへ進まない。 |
| AC-007 | `KEEP_AUTHORIZED`前の15秒timeout、manual revert、Escape、presentation failure、WebView crash、Tauri core強制終了でwatchdog主導のC0 rollbackが成立する。authorization後のlossはAC-020/026のjournal outcomeに従う。 |
| AC-008 | in-flight workerのexit未確認時に並行restore callを発行せず、blockedを正しく報告する。 |
| AC-009 | Keepはactive sessionに一度だけ受理され、current=R、persisted=P0を確認し、再送は同じ結果を返す。 |
| AC-010 | C0!=P0でもtimeout後current=C0/persisted=P0となる。P0 fallbackはdegradedとして区別する。 |
| AC-011 | 全durable write/GO/call/exit/readback境界の停止でstartup decision tableが一意に動き、未知modeを適用しない。 |
| AC-012 | stale supervisor、PID reuse、double launch、double restore、old session confirmがlock/epoch/identityで拒否される。 |
| AC-013 | Reactから未許可command、shell、filesystem、network、watchdog path、raw Win32 parameterへ到達できない。 |
| AC-014 | watchdog executable置換、journal破損、sessionId不一致、未知schemaでfail closedになる。 |
| AC-015 | scaleはread-onlyで、変更controlやapply payloadが存在しない。 |
| AC-016 | Windows 10/11、WebView2、NSIS、署名、GPU/driver/displayごとの有限RC manifestと証跡が承認されている。 |
| AC-017 | 各support cellで全process loss後のblind recovery procedureを反復実証する。実証不能cellは含めない。 |
| AC-018 | 未決定のproduct/SLA/support/signing/accessibility decisionが全て人間のdecision recordを持つ。 |
| AC-019 | 2-stage presentation ACKが専用commandでのみ成立し、missing/late/stale/wrong-stage/old-view ACKではKeep不可のままrestoreする。 |
| AC-020 | manual Revert、timeout、EOF、presentation/session failureとのraceがdecision lock内の`KEEP_AUTHORIZED` entryで一意に決まり、entry前は先行Revert class、entry後はConfirm commit pathが勝つ。同一sessionにKeep/Revertの二つのterminal成功が生じない。 |
| AC-021 | watchdog単独exit/heartbeat lossでreplacementがleaseVersionを進めてrestoreし、old watchdog/workerのlate operationをfenceする。takeover不能時は並行Win32 callを出さずcritical/blockedになる。 |
| AC-022 | wall clock前後変更、15秒超sleep/hibernate、resume、bootId mismatch、60秒session lifetime超過でKeep受付が延長されない。 |
| AC-023 | old session/generation/lease/boot/controller/watchdog/presentation token、別owner SID、別displayId、terminal sessionからの各operationが拒否される。 |
| AC-024 | Fast User Switching/RDP/second logon時にmutationを開始せず、未authorized live sessionはRevertへ進む。`KEEP_AUTHORIZED`後はfrontend authorityを失効しjournal outcomeへ従う。maintenanceはnon-`TERMINAL_CLEAN`/unknown machine record中のupdate/uninstallを拒否する。 |
| AC-025 | initial/root remount/frontend restart/renderer recoveryの`BOOT_HANDSHAKE`で旧bindingが失効し、active presentationをStage 1から再構築またはRevertする。旧`viewRevision`/Stage authorityは拒否され、`ORDINARY_RESYNC`/focus/minimize/child remountではcurrent viewが不必要にrotateしない。 |
| AC-026 | `DecisionJournalV1`のA/B、partial/torn/short write、checksum、generation chain/conflict、flush、readback、response前crash、deadline直前authorization、authorization後I/O delay、outcome unknownでKeep/Revert/FAILED_CLOSED判定が一意になる。React successはvalid `KEPT_SESSION` readback後だけである。 |
| AC-027 | canonical `MachineActorRecordV1`の全wire stateとper-user WALの各durable境界でcrashしても、maintenanceがnon-`TERMINAL_CLEAN`/mismatch/unknownをcleanと誤認せず、cross-user restoreやcleanupを開始しない。instance IDとprocess identityの個別rotateも旧actorをfenceする。 |
| AC-028 | heartbeat miss、hung、exit、IPC stall、resume、CPU starvation、security-product delay、termination failureの各注入で、未証明takeover・二重watchdog・二重workerが生じない。候補値はPhase 2A証跡と人間承認なしにproduct値へ昇格しない。 |
| AC-029 | file absent/A-B uninitialized/old normal terminal/mixed identity/baseline write-flush-reopenの全checkpointで、current rootの有無とretry/fail-closedがarchitecture 7.4の一つのoracleに収束し、baseline provisioning中のmutation 0件、foreign slotからcurrent decisionへ0件である。 |
| AC-030 | current baselineがA/BのどちらにあってもKeepは反対slotに書かれ、partial/torn/invalid Keep中もbaselineが残る。normal terminal reclaimは1 slotずつでaudit intentが残り、critical/blocked/unknown evidenceとwhole-file recreateは自動実行されない。 |

## 10. 未決定事項

以下は`docs/risks-and-open-questions.md`で管理し、設計上の仮定で確定しない。

- session-only Keepを初期版の製品仕様として承認するか
- 15秒保証の測定点、rollback call開始SLO、baseline復元完了SLO、全process lossの保証境界
- Windows 10/11のexact edition/build/KB、CPU architecture、ESU/LTSC方針
- NSISのper-machine/per-user、MSI採否、WebView2 bootstrapper/offline方式、code signing予算
- confirmation初期focusと15秒固定のaccessibility受容
- preferred/native超、DLDSR-like、virtual refresh、HDR有効環境の表示・除外方針
- watchdog起動方式が必要な親喪失独立性とhandle/Job制御を満たせるか
- `bootId`取得に用いるdocumented `LastBootUpTime`/`GetTickCount64` cross-checkが各Windows 10/11 support cellで安定し、同一bootを誤って別bootとしないか
- machine-wide named object/DACL、machine coordination record、watchdog termination/takeoverがstandard userとFast User Switchingで成立するか

これらが未決定または未検証である間、mutation機能の実装・releaseは開始できない。
