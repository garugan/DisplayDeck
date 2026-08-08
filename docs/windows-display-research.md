# Windowsディスプレイ制御 技術調査

最終更新: 2026-08-04  
状態: 文書調査。Rust実装、Windows API実行、実機検証は未承認・未実施。

## 1. 読み方

本書では主張を次の4分類で示す。

- **確認事実**: Microsoft、Tauri、Microsoft `windows-rs`等の一次資料で確認できる事項
- **設計判断**: DisplayDeckが安全側に採る方針
- **推測**: 一次資料だけでは保証できず、設計仮説として扱う事項
- **要検証**: exact Windows build/GPU/driver/displayで技術スパイクが必要な事項

APIの存在と一般的なcontractが文書化されていても、特定driverでの候補列挙、Hz対応、HDR/DLDSR、rollback成功、権限は保証されない。

## 2. 結論

### 2.1 推奨方式

推奨は、RustからMicrosoft `windows` crateを使ってdocumented Win32 display APIを直接呼び出す方式である。ただし、Tauri Rust coreからmutation APIを直接呼ばず、次のprocess分離を行う。

1. Tauri Rust core: command validation、UI/state projection、watchdog起動
2. independent Rust watchdog sidecar: lock、WAL、deadline、parent loss、recovery decision
3. one-shot Rust worker: GDI/CCDの1 operationだけを実行

Microsoftの`windows` crateはWindows metadataから生成したbindingでWin32/COM/WinRT APIをRustから呼べる。`windows`はC-style APIを含む比較的safeなprojection、`windows-sys`はraw C-style bindingという位置付けである。ただしDisplayDeckに必要なsignature/constant/featureが選定versionに全て含まれるかはcompile spikeで確認する。[microsoft/windows-rs](https://github.com/microsoft/windows-rs)

### 2.2 初期single-monitor API候補

設計候補は次のhybridである。

- identity/topology/current observation: CCD `QueryDisplayConfig` + `DisplayConfigGetDeviceInfo`
- GDI device関連付け: `EnumDisplayDevicesW`
- current/persisted/mode candidates: `EnumDisplaySettingsExW`
- preflight: `ChangeDisplaySettingsExW` + `CDS_TEST`
- temporary apply/exact C0 restore: `ChangeDisplaySettingsExW`のdynamic change。profile更新flagなし
- post-apply/restore verification: GDI current + CCD active path/source/targetの両方

`SetDisplayConfig` + `SDC_VALIDATE`/`SDC_APPLY`も比較対象だが、GDI列挙candidateからCCD source/target modeへ完全に変換できることを先に証明する必要がある。将来multi-monitorではCCD batchが有力だが、初期V1を単純置換できるとは断定しない。

### 2.3 初期版でscaleを書かない

documented APIでcurrent DPIを取得できる場合は表示できる。一方、Windows Settingsのper-monitor scaleを安全に列挙・変更・rollbackする公開setter contractは本調査で確定していない。非公開packetやregistry書換えを採用しない。scale mutationは初期版から除外し、別spikeと再レビューを要求する。

## 3. Rustと`windows` crate

### 確認事実

- `windows-rs` repositoryは`windows`と`windows-sys` crateを提供し、Windows metadataからbindingを生成する。
- Win32 display APIはUser32/Shcore等のWindows SDK surfaceであり、Rust側は対応featureを明示してlinkする必要がある。

### 設計判断

- 第一候補は`windows` crate。FFI moduleだけがWindows type/pointerを扱い、上位へsafe domain typeを返す。
- exact crate version/feature listはPhase 1でcompile結果とAPI fidelityを確認後にpinする。設計段階でversionを推測しない。
- `unsafe`はstructure初期化、buffer pointer、Win32 call等の小さい境界へ限定し、各blockのsize/lifetime/aliasing/initialization invariantを文書化する。
- watchdog/worker protocolへWin32 pointer、raw struct memory、`dmDriverExtra`を送らない。

### 要検証

- 対象API/constant/structure unionの選定versionでのbinding名とfeature path
- UTF-16文字列、fixed array、union、LUID、rational refreshのsafe変換
- Windows SDK/Visual C++ toolchainとx64/arm64 artifactの要件
- driverが返すunexpected field/sizeに対するbindingとwrapperの挙動

## 4. GDI API候補

### 4.1 `EnumDisplayDevicesW`

**確認事実**

- current sessionのdisplay device情報をindexで列挙する。
- `DISPLAY_DEVICE_ATTACHED_TO_DESKTOP`でdesktop参加deviceを選べる。
- adapterの`DeviceName`を次の呼出しへ渡してmonitor情報を列挙できる。
- `EDD_GET_DEVICE_INTERFACE_NAME`でmonitor device interface nameを`DeviceID`へ取得でき、SetupAPIとのlinkに使える。

出典: [EnumDisplayDevicesW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaydevicesw)

**設計判断**

- `DISPLAY_DEVICE.DeviceName`はGDI call用のsession-scoped keyであり、永続monitor IDにしない。
- friendly string、DeviceID、flagsをsize/termination/allowed value検証後に使う。
- primary flagは表示用と初期single-path検査に使うが、target identityの唯一の根拠にはしない。

**要検証**

- GDI adapter/monitorとCCD adapter LUID/sourceId/targetId/device pathの一意なcross-map
- inactive attached device、USB dock、GPU switching、virtual adapterの列挙形

### 4.2 `EnumDisplaySettingsExW`

**確認事実**

- 指定display deviceのgraphics modeをindex 0から連続呼出しで列挙する。
- `DEVMODE`の`dmSize`を初期化し、返された`dmFields`を見て有効fieldを判断する。
- 通常返却fieldにはbits per pixel、width、height、display flags、frequency、position、orientationがある。
- `EDS_RAWMODE`はmonitor capabilityにかかわらずadapter driverが報告したmodeを返す。通常呼出しはcurrent monitorとcompatibleなmodeを返す。
- outputはDPI virtualizationの影響を受けずphysical pixelである。

出典: [EnumDisplaySettingsExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsexw)

**設計判断**

- 初期product候補は`EDS_RAWMODE`なしのcompatible listを起点とする。raw modeはdiagnostic/spike限定。
- `ENUM_CURRENT_SETTINGS`相当でC0、`ENUM_REGISTRY_SETTINGS`相当でP0を別々にcaptureする候補とする。
- width/height/Hzを別々に合成せず、返されたallowlisted `DEVMODE` field tupleをcandidate recordにする。
- current bitsPerPel、orientation、display flags、fixed-output等が変わるcandidateは初期版でhard-excludedとする案を検証する。

**要検証**

- 59.94Hz等が`dmDisplayFrequency`へどう表現されるか
- duplicate record、driver-private field、current-not-listed、0/1Hz markerの意味
- DLDSR/DSR/custom/virtual modeが通常/RAW listのどちらに含まれるか

### 4.3 `ChangeDisplaySettingsExW`

**確認事実**

- `EnumDisplayDevices`が返すdevice nameをtargetにgraphics modeを変更できる。
- `DEVMODE.dmFields`で変更対象fieldを指定する。
- flag 0はcurrent screenをdynamicに変更する。
- `CDS_TEST`は実際に変更せずrequested modeをtestする。
- `CDS_UPDATEREGISTRY`はdynamic changeに加えmodeをUSER profileへ保存する。
- device `NULL`、`DEVMODE NULL`、flag 0はdynamic change後にregistry valueへ戻す簡便な方法として文書化されている。
- returnはSUCCESSFUL、BADMODE、BADPARAM、FAILED、NOTUPDATED、RESTART等を区別する。
- Microsoftは`EnumDisplaySettings`から返されたDEVMODEを使ってvalid/supported valueだけを渡すよう記載している。

出典: [ChangeDisplaySettingsExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-changedisplaysettingsexw)

**設計判断**

- 初期preflightはfresh列挙recordに対する`CDS_TEST`候補とする。
- initial temporary applyはprofile保存flagを付けないdynamic call候補とし、`CDS_UPDATEREGISTRY`とunsafe mode flagを使用しない。
- rollbackは`NULL`でP0へ戻す一般fallbackではなく、captured C0のallowlisted complete recordを同一targetへexact applyする。
- success code後も別workerでGDI/CCD readbackを行う。

**推測/要検証**

- flag 0と`CDS_FULLSCREEN`のdesktop switch、sign-out、reboot、driver reset時の差
- dynamic applyがHDR、color depth、VRR/DRR、ICC/color pipelineへ与える副作用
- standard user/local consoleでの権限とUAC不要性。公式ページだけから全対象環境で不要とは断定しない
- C0 exact restoreとP0 fallbackの成功率、operation timeout、driver hang

## 5. CCD API候補

### 5.1 `QueryDisplayConfig`

**確認事実**

- active/all/database display pathとsource/target mode情報を返す。
- `GetDisplayConfigBufferSizes`とqueryの間にtopologyが変わると`ERROR_INSUFFICIENT_BUFFER`になり得るため、再度size取得からretryする必要がある。
- active pathではfull source/target mode情報が得られ、CCDはGDI `DEVMODE`に混在するsource/target成分を明示的に分離する。
- `DisplayConfigGetDeviceInfo`でsource/target名、preferred mode等の追加情報を取得できる。
- APIはDPI virtualizationの影響を受けずphysical pixelを返す。
- virtual-mode aware flagはWindows 10、virtual refresh aware flagはWindows 11に関係する。

出典: [QueryDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)

**設計判断**

- active path、adapter LUID/sourceId/targetId、source resolution、target signal、rational refresh、rotation/scanlineをsnapshot/readbackの正本候補にする。
- buffer retry回数と総上限をboundedにし、変動が収束しなければstale/topology errorにする。
- Windows 11専用virtual refresh flagをWindows 10へ送らない。runtime OS/versionとbinding availabilityを明示分岐する。

### 5.2 `DisplayConfigGetDeviceInfo`

**確認事実**

- source/targetのfriendly name、preferred display mode、source device name等を取得できる。
- WDDMでない場合`ERROR_NOT_SUPPORTED`、console desktop accessがないremote session等では`ERROR_ACCESS_DENIED`を返し得る。

出典: [DisplayConfigGetDeviceInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-displayconfiggetdeviceinfo)

**設計判断**

- friendly nameはUI labelだけに使う。
- adapter LUID/sourceId/targetId/device pathはsnapshot内identity evidenceであり、再起動を越える永続IDにはしない。
- preferred source/target modeをcandidate safety分類に使うが、「preferred超=DLDSR」と断定しない。

### 5.3 `SetDisplayConfig`

**確認事実**

- supplied path/mode arrayまたはdatabase topologyを使い、complete display pathを設定する。
- `SDC_VALIDATE`と`SDC_APPLY`はどちらか一方を指定する。
- `SDC_USE_SUPPLIED_DISPLAY_CONFIG`はcaller supplied path/source/targetを使う。
- `SDC_SAVE_TO_DATABASE`は結果をdatabaseへ保存し、supplied configと組み合わせる。
- `SDC_ALLOW_CHANGES`を使うとWindowsがsupplied mode/pathをfunctional solutionへ変更できる。
- `SDC_VIRTUAL_MODE_AWARE`はWindows 10、`SDC_VIRTUAL_REFRESH_RATE_AWARE`はWindows 11からsupportされる。
- remote/no console accessで`ERROR_ACCESS_DENIED`、unsupported driverで`ERROR_NOT_SUPPORTED`になり得る。
- Microsoftは、多くのcallerが`QueryDisplayConfig`でcurrent configを得てから`SetDisplayConfig`でtest/setすることを想定していると説明する。

出典: [SetDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setdisplayconfig)

**設計判断**

- `SDC_VALIDATE`は表示を変更しないvalidation flagでもmutation API familyである`SetDisplayConfig` callを実行するため、Phase 1A read-onlyには含めない。別承認されたPhase 1BでCCD preflight比較に使用できるか評価する。
- 初期版で`SDC_SAVE_TO_DATABASE`を使わない。
- `SDC_ALLOW_CHANGES`はrequested tupleをOSが変更し得るため、初期exact mode contractでは原則使わない。必要性が判明したcandidateはlab-unqualifiedとする。
- multi-monitor将来版ではfull active pathを1 transactionとして扱うCCD strategyを新設する。V1のmonitorごとのloopにしない。

**要検証**

- GDI candidateをsupplied CCD source/target modeへlosslessに変換できるか
- SDC validate/applyのdriver補正、path priority、virtual mode/refresh差
- GDI dynamic applyとCCD applyのrollback/persistence副作用差

## 6. Candidate mappingとmonitor identity

### 確認事実

- GDIは整数frequencyを含むDEVMODE、CCDはsource/targetとrational refreshを別に表す。
- GDI device name、CCD source name、monitor device path等、異なるnamespaceのidentifierが存在する。

### 設計判断

1 candidateを次の4概念へ分ける。

- `CandidateIdentity`: fresh GDI recordのdigestとsnapshot token
- `DisplayLabel`: `2560 x 1440 @ 59.94 Hz`等の表示
- `ApplyTuple`: allowlisted DEVMODE fields
- `ExpectedObservation`: apply後に期待する完全なGDI/CCD observation

`canApply=true`はexpected observationがexactly oneである場合だけとする。integer Hzからrational Hzへの一般丸め、近似、driver名による推測は使わない。current candidateで公式read-only関係から一意導出できない非current modeは、別承認mutation qualificationでexact hardware fingerprintにboundしたevidenceを得るまで`lab-unqualified`とする。

monitor tokenはsnapshot-scopedとし、`adapter LUID + sourceId + targetId + source device name + monitor device path`等のevidenceをtrusted Rust側に保持する。永続preset keyへ流用しない。

## 7. Resolution/refreshの検証

適用候補は最低限次を満たす。

- fresh normal mode enumerationに同一complete tupleが存在する
- current active pathが1本でtarget cross-mapがexactly one
- width/height/frequency/field maskがboundedで、current color/orientation policyを破らない
- `CDS_TEST`または承認されたvalidate APIが成功する
- expected GDI/CCD observationがexactly one
- support fingerprintがqualification evidenceと一致する
- RDP/virtual/special display、hotplug、topology driftではない

preflight成功はapply成功やvisible outputを保証しない。watchdog、post-readback、15秒rollbackが引き続き必要である。

## 8. Windows 10とWindows 11

### 確認事実

- 本書の主要GDI/CCD APIはWindows 10/11より前からdocumented supportがあるものを含む。
- virtual refresh rate aware flag等、Windows 11で追加された差がある。
- Windows 10 Home/Pro 22H2の通常supportは2025-10-14に終了している。ESUやLTSCは別条件である。[Microsoft lifecycle](https://learn.microsoft.com/en-us/lifecycle/announcements/windows-10-end-of-support)
- TauriはWindowsでWebView2を使用する。Tauriのinstaller文書はWindows 10 April 2018以降とWindows 11にWebView2 runtimeがOSの一部として配布されると説明するが、runtime version/修復/企業imageは実機確認が必要である。[Tauri Windows Installer](https://v2.tauri.app/distribute/windows-installer/)

### 設計判断

- product targetはWindows 10/11だが、`Windows 10`/`Windows 11`という大括りだけでmutation supportを宣言しない。
- exact edition/build/KB/architecture/WebView2/GPU driver/display/connectionごとにRC cellを作る。
- Windows 10 EOL consumer editionをpublic supportするか、ESU/LTSCだけにするかはProduct/Security ownerが決める。
- OS-specific flagはruntime availabilityを検査し、unknown buildでbest effort applyしない。

## 9. 管理者権限とsession

### 確認事実

- CCD APIはconsole desktop accessがないremote sessionで`ERROR_ACCESS_DENIED`を返し得る。
- Tauri NSIS installerはdefault per-userなら通常admin不要、per-machineならadminを必要とする。[Tauri Windows Installer](https://v2.tauri.app/distribute/windows-installer/)

### 設計判断

- runtimeはstandard user、local interactive console、`asInvoker`を第一候補にする。
- runtimeで自動UAC elevationしない。通常権限でdisplay operationが成立しないcellはunsupportedとするか、別security reviewを行う。
- installer elevationとruntime display API privilegeを混同しない。

### 要検証

- Windows 10/11各editionでquery/test/apply/restoreに必要なtoken/desktop/session
- protected installからstandard user watchdogを起動した場合のjournal DACL、process query、mutex namespace

## 10. Scale変更の実現性

### 10.1 取得

**確認事実**: `GetDpiForMonitor`はmonitor DPIをqueryするdocumented APIで、callerのDPI awarenessにより結果が変わる。Microsoftはper-monitor DPI-aware callerではこのAPIを使わず`GetDpiForWindow`を参照するよう注意している。raw/angular DPIはuser scaling overrideを含まない。[GetDpiForMonitor](https://learn.microsoft.com/en-us/windows/win32/api/shellscalingapi/nf-shellscalingapi-getdpiformonitor)

**設計判断**: 初期版はDPI awareness contextを明示し、documented APIでWindows Settingsのuser-facing percentへ信頼できるmappingを作れる場合だけ`known`を表示する。DPIから単純にpercentを推測しない。

### 10.2 変更

本調査で、Windows Settings相当のper-monitor scale候補をdocumentedに列挙し、即時反映、再login/Explorer behavior、rollbackまで保証するpublic setter contractは確定していない。

- undocumented `DisplayConfig` packetを実在・supportすると断定しない。
- registry keyの直接変更、Explorer restart、sign-out automationを採用しない。
- Phase 9でdocumented APIの有無、Windows 10/11差、再login要否、multi-monitor整合性、rollbackを改めて調査する。
- safe setterがなければscale mutationを実装しない。

## 11. DLDSR、virtual display、RDP、HDR、color depth

### DLDSR/DSR

- driverがvirtual/supersampled modeをnormalまたはraw enumerationへ含める可能性はあるが、全vendor/driverでの保証は確認できない。
- preferred mode超、source/target dimension差、分類不能candidateを自動的にDLDSRと命名しない。
- 初期版ではpreferred/native boundary超またはexact mapping不能candidateをhard-excludedまたはlab-unqualifiedにする。

### Virtual/RDP

- RDPやconsole accessなしではCCD callがaccess deniedになり得る。
- virtual display driverの識別はdevice flags/nameだけで推測せず、approved classifierで一意に判定できなければmutationを拒否する。
- head-mounted/specialized displayは通常desktop pathと異なるため初期対象外。

### HDR/color depth

- resolution/refresh changeがHDR、advanced color、bits per pixel、color pipelineへ影響し得るかはdriver依存の要検証事項である。
- 初期候補はC0と同じallowlisted bitsPerPel/orientation/display flagsを要求する案とし、HDR/advanced-color stateをdocumented read-only APIでbefore/after観測できるか調べる。
- HDR stateが変化する、取得できない、またはrollback後一致を証明できないsupport cellはmutation対象外にする。HDR setterは使用しない。

## 12. 実装方式の比較

| 方式 | 難易度 | 安定性 | 配布 | 権限 | Security | Win10/11 | 開発/テスト | 保守性 | 判断 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Tauri Rust coreから直接Win32 | 中 | API自体はdocumentedだがcore hang/crashとrollback ownerが同一 | 単一exe寄り | 実機確認 | frontendから隔離可能 | API差検証要 | Rust unitは容易、failure isolation不足 | 中 | query prototype比較のみ。product mutationには不採用 |
| Rust watchdog + one-shot worker + `windows` crate | 高 | failure domain、deadline、WALを分離できる。driver hangは残る | sidecar同梱・署名が必要 | runtime standard user候補 | 最小command、fixed path、protocol検証が可能 | exact matrix必須 | Windows実機範囲が最大 | 高。Rustへ統一 | **推奨** |
| PowerShell起動 | 低〜中 | cmdlet/script/version/policy/locale/AV差 | script/host依存 | policy次第 | injection、execution policy、host surface増 | 実機差大 | 手軽だがproduct fidelity低 | 低 | 正式方式に採用しない |
| Node.js native add-on | 高 | ABI/toolchain/Node lifecycle追加 | native rebuild/package複雑 | API次第 | Web runtimeとの境界が増える | matrix必須 | Tauri/Rustへ不要なNode native層 | 低 | 採用しない |
| 既存Node package | 低 | packageのAPI coverage/保守次第 | dependency容易だがTauriと二重runtime | 不明 | supply-chain/child process実装を要監査 | 不明 | exact recovery contract不足 | 低 | 採用しない |
| 既存Rust crateの高水準display wrapper | 低〜中 | 必要API/flags/rollback coverage次第 | Cargo dependency | API次第 | unsafe/supply chain監査要 | crate実績次第 | spikeで比較可 | 中 | 存在・適合を推測しない。`windows`直結をbaseline |

PowerShellを使わない理由は「Rustの方が常に安全」だからではない。DisplayDeckが必要とするexact struct/flag/return/readback、one-shot process、bounded protocol、WAL stateを一つのRust codebaseで明示しやすく、shell/script surfaceを増やさないためである。

## 13. 技術スパイクで検証すべき事項

### Phase 1A: read-only必須

- allowlistは`EnumDisplayDevicesW`、`EnumDisplaySettingsExW`（current/registry/normal/raw enumeration）、`GetDisplayConfigBufferSizes`、`QueryDisplayConfig`、`DisplayConfigGetDeviceInfo`、documented read-only OS/session/clock observationに限定する。
- `windows` crate binding fidelityと必要feature
- GDI/CCD identity cross-mapとhotplug retry
- C0/P0/current-not-listed、全normal/raw candidate
- GDI integer refreshとCCD rational refreshのexact mapping
- Windows 10/11 virtual mode/refresh flag差
- preferred boundary、DLDSR-like、HDR、DRR、color depth observation
- RDP、virtual display、multiple active pathのfail-closed classification
- standard user権限とAPI latency/hang傾向
- current process owner SID/logon LUID、active console/Windows session ID、OS version/build/installed KB、RDP/Fast User Switching visibilityをdocumented read-only queryだけで観測する

Phase 1Aでは次を明示禁止する。

- `CDS_TEST`
- `SDC_VALIDATE`
- `ChangeDisplaySettingsExW`の全flag/argumentによるcall
- `SetDisplayConfig`の全flag/argumentによるcall
- temporary apply、restore、profile/registry write、display setting mutation
- `CreateMutexW`/`OpenMutexW`、named semaphore/event、mutex ownership、abandoned test、`Global\\`/`Local\\` writable object、security descriptor/DACL/SDDL変更、machine gate/record/per-user WAL/lock prototype作成（Phase 2Aへ移管）

`TEST`/`VALIDATE`という名称や「実変更しない」結果を理由にread-onlyへ分類しない。

### Phase 1B: 別承認のcontrolled mutation

- `CDS_TEST`/`SDC_VALIDATE`の一致とfalse positive/negative
- flag 0/temporary candidateのprofile persistence副作用
- valid/invalid resolution/refresh、apply/readback、C0 exact restore
- C0!=P0、driver-adjusted result、HDR/advanced color保持
- process kill、API timeout、physical recovery procedure
- snapshot durable保存、watchdog ready、2-stage presentation ACK、Confirm/Revert race、timeout/crash recovery、session/boot/owner/display fencing、watchdog live takeover

このspikeはapproved spike-only watchdog/control harnessを使うがproduct integrationではなく、外部のblind recovery手段と安全責任者の承認下で行う。Phase 2A/2Bのwatchdog evidence前に一般ユーザー向けmutation codeへ流用しない。

### Clock/boot/sessionのread-only evidence

- deadline用のexact APIはWin32 `GetTickCount64`とし、sleep/hibernateを経過へ含める。[Microsoft: GetTickCount64](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-gettickcount64)
- wall timeは`GetSystemTimePreciseAsFileTime`をdiagnosticとboot/stale矛盾検出補助だけに使い、live deadline comparison・再構築・延長へ使わない。[Microsoft: GetSystemTimePreciseAsFileTime](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemtimepreciseasfiletime)
- bootId候補はread-only `Win32_OperatingSystem.LastBootUpTime`、exact OS build、`GetTickCount64`/UTC差をcross-checkして作る。wall clock変更、sleep/hibernate、Fast Startup、reboot、watchdog restartでsame/different boot classificationをPhase 2Aまでに証明する。取得不能または矛盾時はmutation不可であり、別clockへsilent fallbackしない。[Microsoft: CIM_OperatingSystem](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/cim-operatingsystem)
- `Global\\` named mutexはTerminal Services sessionをまたいで見える一方、Fast User Switchingもsessionを分けるため、exact namespace/create/open/DACL/standard-user behaviorはPhase 2A lock prototypeだけで確認する。Phase 1Aではnamed object callを行わない。[Microsoft: Kernel Object Namespaces](https://learn.microsoft.com/en-us/windows/win32/termserv/kernel-object-namespaces)

### Phase 2A/2B/8

- watchdogのparent loss生存、Job/handle inheritance、Task Manager kill
- worker termination/quiescence/PID reuse/permission denied
- operational dual-slot WALとfixed-slot A/B `DecisionJournalV1`のflush/torn/short write/unknown schema/startup recovery、`KEEP_AUTHORIZED` entry、generation chain、readback outcome、AV/filter/share/power-loss behavior
- `MoveFileExW`/`ReplaceFileW`をdeadline CAS/active-slot publicationに使わず、`FlushFileBuffers`はdeadlineを評価しないこと、`FILE_FLAG_WRITE_THROUGH`単独でatomicity/CASを証明しないことをfault modelで確認する
- architecture 19.2 canonical machine wire `ACTIVE_INTENT`→per-user PREPARED/`ACTIVE_PREPARED`、per-user terminal→`TERMINALIZING`→actor quiescence→`TERMINAL_CLEAN`のdurable順、maintenance begin/complete/schema compatibility fence
- heartbeat miss/hung/IPC stall/resume/starvation/security-product delay/exitを分ける`HeartbeatPolicyV1`測定
- sidecar同梱、NSIS/MSI install/repair/update/uninstall、署名publisher検証
- WebView2 runtime/install modeとoffline環境

documented file APIだけで全filesystem/storage/power-loss条件の絶対耐久性が保証されるとは扱わない。fixed slots、generation/checksum、flush、close/reopen readback、recovery selectionの組合せをexact Phase 2A cellで実証できなければPhase 1BはNo-Goである。

## 14. 推奨理由と中止条件

Rust + `windows` crate + independent watchdog/workerを推奨する理由:

- TypeScript/WebViewからOS操作を完全に分離できる。
- Win32 struct、flag、return code、readbackを同一言語のsafe wrapperへ閉じ込められる。
- watchdog、worker、WAL、protocolをRustで統一し、Node ABIやscript hostを不要にできる。
- future CCD multi-pathを別strategyとして追加できる。

ただし次のいずれかならmutation機能を中止またはscope縮小する。

- GDI candidateとCCD observationを一意に対応できない。
- C0 exact restoreまたはblind P0 recoveryがfinite matrixでzero-toleranceを満たさない。
- watchdogがTauri強制終了後に生存できない、または旧worker quiescenceを証明できない。
- standard user/local consoleで必要APIが安定しない。
- sidecar署名・protected install・update/uninstall safetyを運用できない。

## 15. 主要一次資料

- [microsoft/windows-rs](https://github.com/microsoft/windows-rs)
- [EnumDisplayDevicesW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaydevicesw)
- [EnumDisplaySettingsExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsexw)
- [ChangeDisplaySettingsExW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-changedisplaysettingsexw)
- [QueryDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)
- [DisplayConfigGetDeviceInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-displayconfiggetdeviceinfo)
- [SetDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setdisplayconfig)
- [GetDpiForMonitor](https://learn.microsoft.com/en-us/windows/win32/api/shellscalingapi/nf-shellscalingapi-getdpiformonitor)
- [Tauri sidecar](https://v2.tauri.app/develop/sidecar/)
- [Tauri Windows installer](https://v2.tauri.app/distribute/windows-installer/)
