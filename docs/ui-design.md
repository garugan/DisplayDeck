# DisplayDeck UI設計

最終更新: 2026-08-04  
状態: Tauri/WebView2向け設計案。再レビュー・実装未承認。

## 1. UIの目的

UIは「現在値」「まだOSへ適用していない変更予定」「適用後の確認待ち」「復元結果」を混同させない。Reactはdraftと表示だけを持ち、Windows設定、change session、15秒deadlineの正本にはならない。

初期版はTauriの単一main WebViewWindowを使う。確認は同じwindow内のmodal overlayで行い、独立watchdogの復元はoverlay/WebView/Tauri coreの生存に依存しない。

## 2. 画面構成

### 2.1 Main screen

- アプリ名、controller状態、refresh action
- monitor選択/表示。初期mutationはactive pathが1本の場合だけ
- 現在設定card
  - resolution
  - refresh
  - primary status
  - scale observation (`known`/`unknown`/`unsupported`)
- 変更予定card
  - resolution discrete control
  - refresh discrete control
  - changed marker
- Apply、Reset
- inline status/error summary

scaleは初期版でread-only rowとし、disabled sliderを置かない。変更対象と誤認させないためである。

### 2.2 Confirmation overlay

- 「新しい表示設定を確認してください」
- before/after summary
- 残り時間とprogress
- 「元に戻す」
- 「設定を維持する」
- UIが応答してもwatchdogが最終判定することを示す短い説明

overlayはmutation前にDOM/layoutを準備する。apply後、Rust coreがmain windowをrestore/show/focusし、必要な間だけtopmost要求を行う。Reactはeventを返送に使わず、専用`ack_display_change_presentation` commandでStage 1/2をACKする。presentation ACK、focus、window bounds、generation/lease/view同期が所定時間内に成立しない場合、15秒を待たずwatchdogが復元する。

### 2.3 Startup recovery screen

未完了journalがある場合、通常main screenより先に表示する。

- 「前回の変更状態を確認しています」
- sessionのsanitized status
- restoring/blocked/failedの明確な区別
- blocked/failed時のblind recovery案内

前回の確認overlayを再開せず、安全側にrestoreする。

## 3. 簡易ワイヤーフレーム

### Main

```text
┌──────────────────────────────────────────────────────────┐
│ DisplayDeck                           状態: 準備完了  [更新] │
├──────────────────────────────────────────────────────────┤
│ 対象モニター                                             │
│ [ Display 1 — NVIDIA ...                         ▾ ]      │
│ ※ 初期版はアクティブな画面が1台のときだけ変更できます     │
├───────────────────────┬──────────────────────────────────┤
│ 現在の設定             │ 変更予定                          │
│ 解像度  1920 × 1080    │ 解像度  2560 × 1440  [変更]       │
│ Hz      60.00 Hz       │ Hz      120.00 Hz     [変更]       │
│ 拡大率  125%（表示のみ）│ 差分  解像度・Hz                  │
├───────────────────────┴──────────────────────────────────┤
│ 解像度   1920×1080 ──●──── 2560×1440   [候補一覧 ▾]       │
│ Hz       60 ─────●──────── 120          [候補一覧 ▾]       │
│                                                          │
│ [リセット]                              [設定を適用]       │
└──────────────────────────────────────────────────────────┘
```

### Confirmation overlay

```text
┌──────────────────────────────────────────────┐
│ 新しい表示設定を確認してください              │
│                                              │
│ 1920×1080 / 60.00 Hz                         │
│           ↓                                  │
│ 2560×1440 / 120.00 Hz                        │
│                                              │
│ 残り 12 秒  [██████████████░░░░]             │
│ 操作できない場合は自動的に元へ戻ります。       │
│                                              │
│ [元に戻す]              [設定を維持する]      │
└──────────────────────────────────────────────┘
```

### Critical recovery

```text
┌──────────────────────────────────────────────┐
│ 復元を確認できません                         │
│ 自動処理で元の表示へ戻ったことを確認できません。│
│ 診断ID: DD-XXXX                              │
│                                              │
│ 1. Windowsを再起動する                       │
│ 2. ケーブルを再接続する                      │
│ 3. Windowsの表示設定を開く                   │
│                                              │
│ [状態を再確認]                               │
└──────────────────────────────────────────────┘
```

## 4. UI状態一覧

| Rust authoritative state | UI | Input |
| --- | --- | --- |
| Idle/Loading | skeletonと「確認中」 | refresh/apply不可 |
| Ready clean | current=planned | Apply/Reset不可、candidate操作可 |
| Ready dirty | 差分表示 | Apply/Reset可 |
| Validating | 「候補を再確認中」 | 全変更control無効 |
| PreparingRollback | 「復元準備中。画面はまだ変更していません」 | 全変更control無効 |
| Applying | 「一時適用中」 | Revert request以外無効 |
| PresentingConfirmation stage 1 | overlay、Revert enabled/focused、Keep disabled | Revert、Stage 1 ACK |
| PresentingConfirmation stage 2 | overlay、両button enabled、countdown | Revert、Stage 2 ACK。ACK response前のKeepはRustが拒否 |
| AwaitingConfirmation | 2-stage ACK済みoverlay、両button enabled | Keep/Revert |
| Confirming（authorization前） | 「維持要求を確認中」。Keep無効、Revertは有効 | Revert/status。watchdog decision lockで`KEEP_AUTHORIZED`と競合し得る |
| ConfirmCommitInProgress | 「設定の維持を確定処理中」。Keep/Revert無効 | statusのみ。deadline後もdisk commit待ち得る。成功と推測しない |
| Restoring | 「元の設定へ戻しています」 | 全button無効 |
| Restored exact | 復元完了、fresh snapshot要求 | acknowledge/refresh |
| Restored degraded | 重大警告、P0へ戻した説明 | status/案内のみ |
| Failed no mutation | Windows未変更を明示 | 条件付きretry |
| Failed critical/blocked | 最重要案内、Apply禁止 | status/physical recoveryのみ |

React local stateはRust stateより先へ進めない。event gap、focus/minimize復帰、child remountは`get_display_change_status(StatusRequestV1{mode:ORDINARY_RESYNC,...})`で上書き同期し、viewをrotateしない。初回root mount、root remount、frontend bootの明示的再開始、renderer復旧は`mode:BOOT_HANDSHAKE`を送る。reload/crash/recreate時はold presentationToken/viewRevisionと全ACK済みlocal stateを破棄し、new `viewRevision`、`controllerInstanceId`、authoritative statusが返るまでApply/ACK/Keep/Revertをdisabledにする。同じ確認画面やStage 1 ACK権限をlocal stateだけで再開・移送しない。

`viewRevision`はReactのrender counter、route revision、timestamp、storage値ではない。page-load Startedでcoreが旧bindingを失効し、Finished後の`BOOT_HANDSHAKE`受理時に発行する。root remountはpage-loadを伴わなくても必ず同modeを送る。child component remount、focus、minimize/restore、通常renderでは`ORDINARY_RESYNC`だけを行う。`frontendBootNonce`は同一boot duplicate識別用で、ACK/Keep authorityではない。raw tokenをlocalStorage/sessionStorage/logへ残さない。

## 5. Sliderとselectの挙動

### 5.1 Control選定

sliderは少数のordered candidateを素早く比較する場合に有効だが、候補数が多い、labelが長い、keyboard/screen readerで位置だけでは値が分かりにくい場合に不向きである。

初期design:

- 候補2〜8件程度: discrete sliderを主control、同じ候補をselect/listでも選択可能
- 候補9件以上、横幅不足、zoom 200%、label重複: select/listを主controlに切り替える
- 候補1件: value rowとして表示し、変更controlにしない
- 候補0件: unavailable理由を表示しApply不可

閾値8はUI仮説であり、usability testで決める。どちらのcontrolも同じopaque candidate tokenを選び、数値を生成しない。

### 5.2 共通

- `min=0`、`max=candidates.length-1`、`step=1`相当のindex選択
- visible valueはcandidate label、accessible value textは「3/5、2560×1440」等
- pointer drag中もdraftだけを更新し、Tauri mutation commandを呼ばない
- candidate orderはRust snapshotが返すstable sort keyを使い、React独自にsortし直さない

### 5.3 Resolution

- width、height、必要ならaspect/normality情報をlabelにする。
- current resolutionがselectable list外ならcurrent cardには残すが、選択controlへ偽candidateを合成しない。
- preferred/native超やDLDSR-like分類不能candidateは初期版selectionへ出さない。診断表示の有無は未決定。

### 5.4 Refresh

- 選択resolution groupのcandidateだけを表示する。
- labelは60.00/59.94等を区別できるprecisionをRustから受け取る。Reactでinteger丸めしない。
- duplicate表示値でも内部tupleが異なる場合、一般ユーザーに安全に区別できなければcandidateを適用不可にする。

## 6. 候補更新ルール

### Monitor変更

初期single-pathでは通常1件だけである。複数接続を表示する場合もmutation capabilityはfalseとし、個別monitorを選べば変更可能になるような誤解を与えない。

### Resolution変更

1. 新resolution groupのrefresh listを取得する。
2. 前のexact refresh tokenが残れば保持する。
3. 残らなければrational値の差が最小の候補を選ぶ。
4. 同差は低いrefreshを選ぶ。
5. 自動変更したことをlive regionでは過剰に読み上げず、field descriptionとstatusで通知する。

### Snapshot更新

- topology/current changeで全tokenを失効し、draftを自動適用しない。
- currentとdraftが同じcandidateへ再解決できればcleanへ戻す。
- draft candidateが消えた場合はcurrentへresetし、「候補が変更されたため選択を戻しました」と表示する。
- active transaction中のrefreshは拒否する。

## 7. 適用前後の表示

### 適用前

- Apply enabled条件をUIでも示すが、最終validationはRust側で行う。
- 変更内容をcurrent → plannedの2列またはdiff rowで示す。
- session-only Keepの場合、「再起動後も保存される」と表示しない。

### 適用中

- draft controlをdisabledにし、同じbuttonの連打を防ぐ。
- `PreparingRollback`ではまだ画面未変更、`Applying`では変更結果確認中、と文言を分ける。
- spinnerだけでなくtext stateを出す。

### Keep後

- `DecisionJournalV1`のnew valid `KEPT_SESSION` generationをwatchdogがclose/reopen後のA/B readbackで確認したterminal ACKと、fresh snapshotの両方を得てから成功表示する。`KEEP_AUTHORIZED`や「確定処理中」を成功と扱わない。
- persisted=P0のsession-onlyであることを表示する。

### Rollback後

- exact: 「元の設定へ戻しました」
- degraded: 「保存済み設定へ戻しましたが、直前の設定とは一致しません」
- failed/blocked: 「元へ戻ったことを確認できません」

## 8. Confirmation overlay

### 8.1 Countdown

- watchdogが`GetTickCount64`から計算したbounded `remainingMs`のprojectionとして表示し、native absolute tickやlocal 15→14 counterを正本にしない。これは`KEEP_AUTHORIZED`へ入れる残り時間であり、disk flush完了までの残り時間ではない。正値であってもKeep acceptanceを保証しない。
- status/event sampleの`remainingMs`を受け、browser `performance.now()`相当で短時間の表示だけを補間する。最低1秒ごと、focus/resume/event gap時にstatusへ再同期し、0以下ではKeepを保守的にdisabledにする。ただしfrontendの0表示もterminal Revert decisionではなく、watchdogのauthoritative tick/decisionが正本である。click時は表示値で先に成功扱いせず、watchdogの`KEEP_AUTHORIZED`判定とdurable `KEPT_SESSION` resultを待つ。authorization後に0へなってもUIからRevertを開始しない。
- UIが0を表示しても、最終accept/rejectはwatchdog responseに従う。

### 8.2 Presentation handshake

Stage 1:

- overlay visible、expected session/generation表示
- Revert enabled、initial focus候補
- Keep disabled
- bounds、visibility、focus request、accessible renderのack
- payload `{sessionId,generation,leaseVersion,presentationToken,stage:"REVERT_READY",viewRevision,ackNonce}`を専用commandへ送る

watchdogがStage 1を消費しtoken/generationをrotateした後のStage 2:

- Keep enabled、Revert enabled
- focus policyを維持
- payload `{sessionId,newGeneration,leaseVersion,newPresentationToken,stage:"CONFIRMATION_READY",viewRevision,newAckNonce}`を同じ専用commandへ送る
- watchdogが`AWAITING_CONFIRMATION`をdurable化したresponse後だけConfirmをactiveとみなす

両ACKが`t0+2秒`以内かつ残12秒以上を満たさなければ即時rollbackする。同じtoken/nonce/payloadのduplicateは同じstatusを返すだけで、old view、wrong stage、late/stale tokenは拒否する。Stage 1後にreloadした新viewからStage 2だけを送ることも拒否し、presentation failureとしてrollbackする。Stage 1→Stage 2では同じviewをexact next-generation/tokenへRustが明示的にrebindするため、Reactがtoken関係を推測しない。ACKは物理的に映像が見えることを証明しないため、watchdog deadlineを置き換えない。

active presentation中にroot remountまたはfrontend boot再開始が起きた場合、new rootは`BOOT_HANDSHAKE`を送り、old stage/ACK local stateを全て捨てる。Rust coreは旧bindingを失効し、残りpresentation deadline内でnew `viewRevision`へStage 1を再発行する。new rootが`PRESENTATION_RESYNC`でold Stage 2 authorityだけを得る経路はない。再構築が期限内に完了できなければRevertへ進む。

### 8.3 Keyboard

- Escape: active confirmationをrestore requestへ送る。responseが返らなくてもwatchdog deadlineは継続する。
- Alt+F4/window close: `KEEP_AUTHORIZED`前はrestore requestと同義で、Tauri coreが消えてもcontrol pipe EOFでwatchdogが戻す。`ConfirmCommitInProgress`ではcloseからRevertを推測せず、watchdogがDecisionJournal commitまたはloss readbackを完了する。
- Tab/Shift+Tab: overlay内へfocus trapし、2 buttonと説明要素の順序を安定させる。
- Enter: 要求候補はKeepだが、過去reviewのDDR-Q05が未決定である。safe baselineはRevert初期focus、Enterはfocused buttonだけをactivateし、global Keep shortcutを設けない。Enter=Keepを採るにはProduct/Accessibility/Safety ownerの明示決定と誤操作testを必要とする。

## 9. 画面が映らない場合

UI操作不能を正常なfailure scenarioとして扱う。

- React countdown、keyboard、focus、eventを復元条件にしない。
- watchdogはTauri control pipeが生存していてもdeadlineでrestoreする。
- WebView/Tauri core終了が`KEEP_AUTHORIZED`前ならEOFでrollbackを開始する。authorization後ならUI不在でもwatchdogがterminal slot publicationを続け、writer loss時はjournal readback結果に従う。
- watchdog単独lossでTauri coreが生存する場合、UIは`RECOVERY_TAKEOVER`を表示し、replacement watchdogがconfirmationを再開せずrestoreする。old overlay/tokenを破棄する。
- Tauri coreとwatchdogの同時lossは15秒保証外である。各support cellにblind reboot/sign-out/physical recovery procedureを用意する。
- sleep/hibernateからのresumeでは最初にstatusを取得し、期限超過ならoverlayを操作可能に戻さずRestoringへ同期する。

## 10. Error表示

| Level | 例 | 表示 |
| --- | --- | --- |
| Info | refresh中、自動refresh候補変更 | inline status |
| Warning | multi-path、remote、unsupported candidate、scale unknown | 変更不能理由と対処 |
| Error/no mutation | stale、preflight rejected、watchdog start failure | 「Windowsは変更されていません」 |
| Critical/restored | apply失敗後exact restore | original errorより復元結果を先に表示 |
| Critical/degraded | P0 fallback | C0ではないこと、診断ID |
| Critical/failed/blocked | rollback未確認、worker不停止、journal unknown | persistent modal、Apply禁止、blind recovery案内 |

raw path、device path、stack、Win32 code、journal内容を表示しない。ユーザー向けmessageとdiagnostic IDを分ける。

## 11. Accessibility

- native HTML semanticsを優先し、button/select/rangeへaccessible name/value/descriptionを設定する。
- current/planned/changedをtextでも示す。
- countdownは毎秒live announcementせず、残10/5秒等の節目と状態変化をpoliteに通知する案を検証する。
- critical errorはfocusをheadingへ移し、復旧手順を順序付きtextで提供する。
- Windows high contrast、200% zoom、text scale、Narrator、keyboard-onlyを実機gateにする。
- motion reduction設定時はnonessential animationを抑制する。
- confirmationの固定15秒が読み上げと意思決定に十分かはDDR-Q02/Q05の人間判断と実機evidenceを必要とする。

## 12. UI受け入れ観点

- candidate操作だけではmutation commandが呼ばれない。
- candidate多数時にselectへ切り替えても同じtoken/stateになる。
- Rust stateとReact state不一致がstatus commandで収束する。
- startup pendingはconfirmation再開ではなくrecovery画面になる。
- Applying/Restoring中のdouble click/keyboard shortcutが二重commandを作らない。
- confirmation presentation failureが15秒待ちではなく即時restoreになる。
- black screenでもUIなしでwatchdog restoreが成立する。
- exact/degraded/failed/blockedの文言を誤認しない。
