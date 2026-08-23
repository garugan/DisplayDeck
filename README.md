# DisplayDeck

Windows 10 / Windows 11向けのディスプレイ管理アプリケーションです。

## Step 1: `EnumDisplayDevicesW` の確認（Windows実機検証完了）

Step 1では、読み取り専用のRust CLIを使って、Windowsのディスプレイアダプターと各adapter配下のmonitorを列挙します。ディスプレイ設定の変更は行いません。

実装は [`native/display-probe`](native/display-probe) にあります。

### 取得する情報

- ディスプレイアダプター一覧
- 各adapter配下のmonitor一覧
- `DeviceName`
- `DeviceString`
- `DeviceID`
- `DeviceKey`
- primary displayかどうか
- desktopに接続されているか
- raw `StateFlags`とmirroring / remote / RDPUDD SDK marker（positive diagnosticのみ）

### 前提条件

- Windows 10またはWindows 11のネイティブ環境（WSLではなくWindows側で実行）
- Rust stableの`rustc`と`cargo`が利用可能
- Windowsのローカル対話セッションから実行

ツールチェーンを確認します。

```text
rustc --version
cargo --version
```

### 実行手順

リポジトリのルートディレクトリで、次を実行します。

```text
cargo run --manifest-path native/display-probe/Cargo.toml
```

### 期待する出力

環境によって値や件数は異なりますが、次の形式で表示されます。

```text
Adapter 0
  DeviceName: \\.\DISPLAY1
  DeviceString: NVIDIA GeForce RTX 4070
  DeviceID: ...
  DeviceKey: ...
  Primary: true
  AttachedToDesktop: true
  StateFlagsRaw: 0x00000005
  MirroringDriverMarker: false
  RemoteSdkMarker: false
  RdpuddSdkMarker: false
  CurrentResolution: 3440x1440
  CurrentRefreshRateHz: 144

  Monitor 0
    DeviceName: ...
    DeviceString: MSI MAG342CQ
    DeviceID: ...
    DeviceKey: ...
    Primary: false
    AttachedToDesktop: true
```

### 確認項目

1. `windows = 0.62.2`を使用してCLIが正常にbuildできる。
2. 接続環境に対応するadapterが列挙される。
3. adapter配下にmonitorが列挙される。
4. 各deviceの`DeviceName`、`DeviceString`、`DeviceID`、`DeviceKey`が表示される。
5. primaryとdesktop接続状態が表示される。
6. 実行前後でWindowsの解像度、refresh rate、monitor配置などが変化しない。

`DeviceID`、`DeviceKey`、monitor名の内容は、GPU、driver、monitor、接続方式によって異なる場合があります。確認時は省略せず、実際の標準出力を保存してください。

### Step 1の完了条件

上記のbuild、列挙結果、読み取り専用であることはWindows実機で確認済みです。

## Step 2: 現在の解像度・refresh rateの確認（Windows実機検証完了）

Step 2では、各adapterの`DeviceName`を使って`EnumDisplaySettingsExW`を呼び、現在の解像度とrefresh rateを取得します。

- mode: `ENUM_CURRENT_SETTINGS`
- flags: `0`
- `DEVMODEW.dmSize`: 呼び出しごとに`size_of::<DEVMODEW>()`を設定
- 有効性: `dmFields`の`DM_PELSWIDTH`、`DM_PELSHEIGHT`、`DM_DISPLAYFREQUENCY`を確認

実行コマンドはStep 1と同じです。

```text
cargo run --manifest-path native/display-probe/Cargo.toml
```

adapter情報の末尾に次の2行が追加されます。

```text
  CurrentResolution: 3440x1440
  CurrentRefreshRateHz: 144
```

APIが現在値を返さない場合や必要な`dmFields`がない場合は`unavailable`と表示します。`dmDisplayFrequency`が`0`または`1`の場合は、具体的なHzとして扱わず`driver default`と表示します。

GDIの`dmDisplayFrequency`は整数値です。59.94Hzなどの厳密な分数refresh rateとの照合は、Step 4のCCD `QueryDisplayConfig`で行います。

### Step 2の確認項目

1. CLIが正常にbuild・実行できる。
2. desktopに接続された各adapterに`CurrentResolution`が表示される。
3. `CurrentRefreshRateHz`が表示される。
4. Windows Settingsに表示される現在の解像度・refresh rateと比較する。
5. 実行前後で解像度、refresh rate、monitor配置などが変化しない。

### Step 2の完了条件

現在の解像度と整数refresh rateの取得結果はWindows実機で確認済みです。

## Step 3: 利用可能な解像度・refresh rate一覧の確認（Windows実機検証完了）

Step 3では、各adapterの`DeviceName`を使って`EnumDisplaySettingsExW`を繰り返し呼び、利用可能なmodeを列挙します。

- mode index: `0`から開始し、APIがfalseを返すか承認済み上限`4095`へ達するまで1ずつ増加。`4096`は呼び出さない
- flags: `0`（`EDS_RAWMODE`は使用しない）
- `DEVMODEW.dmSize`: 呼び出しごとに`size_of::<DEVMODEW>()`を設定
- 有効性: `dmFields`の`DM_PELSWIDTH`、`DM_PELSHEIGHT`、`DM_DISPLAYFREQUENCY`を確認
- 順序: APIが返したindexと列挙順を保持

実行コマンドはこれまでと同じです。

```text
cargo run --manifest-path native/display-probe/Cargo.toml
```

adapter情報にmode件数と一覧が追加されます。

```text
  AvailableModes: 3
  AvailableModesEnumeration: Complete
    Mode 0: 1920x1080 @ 60 Hz
    Mode 1: 2560x1440 @ 60 Hz
    Mode 2: 2560x1440 @ 144 Hz
```

同じ解像度・refresh rateが複数回表示される場合でも、列挙recordを勝手に統合しません。表示していないbit depthやorientationなどが異なる可能性があるためです。

APIが1件もmodeを返さないadapterは`AvailableModes: 0`と表示します。fieldが取得できないrecordや`dmDisplayFrequency`が`0/1`のrecordは、Step 2と同じ規則で`unavailable`または`driver default`と表示します。標準出力へ展開するlegacy mode行は全adapter合計8192件を上限とし、省略時も`AvailableModes`総数と`ModeRecordsOmitted`を表示します。

### Step 3の確認項目

1. CLIが正常にbuild・実行できる。
2. desktopに接続された各adapterに`AvailableModes`の件数が表示される。
3. mode indexが`0`から連続して表示される。
4. Windows Settingsで選択可能な主要な解像度・refresh rateが一覧に含まれる。
5. 重複recordや`unavailable`がある場合は、標準出力を省略せず保存する。
6. 実行前後で解像度、refresh rate、monitor配置などが変化しない。

### Step 3の完了条件

利用可能な解像度・整数refresh rate一覧はWindows実機で確認済みです。重複recordは調査用に保持し、driverが列挙するネイティブ解像度を超えるmodeも削除していません。

## Step 4: CCD active configurationの確認（Windows実機検証完了）

Step 4では、CCD APIを使って現在のactive display pathとmode情報を取得します。

- `GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS)`で必要な配列長を取得
- `QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS)`でactive pathとmode情報を取得
- topology変更による`ERROR_INSUFFICIENT_BUFFER`では、size取得から最大3回retry
- path上限256件、mode上限1024件
- source/targetのmode index、type、adapter LUID、IDを照合してからunionを読み取る
- `QDC_VIRTUAL_MODE_AWARE`、`QDC_VIRTUAL_REFRESH_RATE_AWARE`、`QDC_DATABASE_CURRENT`はまだ使用しない

実行コマンドはこれまでと同じです。

```text
cargo run --manifest-path native/display-probe/Cargo.toml
```

GDI情報より前に、次のようなCCD情報が表示されます。

```text
CCD Active Configuration
  ActivePaths: 3
  Path 0
    Source: adapter=0x0000000000000000 id=0 modeInfoIndex=0
    SourceMode: 3440x1440 at (0, 0) pixelFormat=4
    Target: adapter=0x0000000000000000 id=1 modeInfoIndex=1
    TargetAvailable: true
    TargetPathRefreshRate: 144000/1000 (144.000000 Hz)
    TargetModeActiveSize: 3440x1440
    TargetModeVSync: 144000/1000 (144.000000 Hz)
```

実際のadapter LUID、ID、mode index、rationalの分子・分母は環境によって異なります。decimal値だけでなく、分子と分母を含む標準出力全体を保存してください。

Step 4の完了時点では`DisplayConfigGetDeviceInfo`を呼ばず、CCD source/targetのfriendly nameやGDI `DeviceName`とのcross-mapは対象外でした。これらは次のStep 5で追加します。Step 3のmode候補とCCD active pathを適用可能候補として関連付ける処理は、引き続き未実装です。

### Step 4の確認項目

1. CLIが正常にbuild・実行できる。
2. `ActivePaths`がWindowsでactiveなdisplay path数と整合する。
3. 各pathにsource/targetのadapter LUID、ID、mode indexが表示される。
4. source modeの解像度・位置が現在のdesktop配置と整合する。
5. target modeのactive sizeとrational VSyncが表示される。
6. GDIの整数refresh rateとCCDのrational refreshを比較する。
7. desktop未接続のadapterがactive pathとして誤って表示されない。
8. 実行前後で解像度、refresh rate、monitor配置などが変化しない。

### Step 4の完了条件

active path、source/target mode、rational refreshはWindows実機で確認済みです。friendly name取得とGDI↔CCD cross-mapは、次のread-only Step 5として扱います。

## Step 5: GDI ↔ CCD exact cross-mapの確認（Windows実機検証完了）

Step 5では、GDIとCCDが返すidentityを使って、各active CCD pathをGDI adapter/monitorへ対応付けます。friendly name、解像度、位置、refresh rateによる推測は行いません。

### 使用するread-only API

- `DisplayConfigGetDeviceInfo(DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME)`
  - CCDの`(adapterId, sourceId)`から`viewGdiDeviceName`を取得
- `DisplayConfigGetDeviceInfo(DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME)`
  - CCDの`(adapterId, targetId)`からfriendly name、monitor device path、output technology、connector instance、flags、EDID IDsを取得
- `EnumDisplayDevicesW(..., EDD_GET_DEVICE_INTERFACE_NAME)`
  - 同じadapter `DeviceName`とmonitor indexを使い、GDI monitorのdevice interface pathを別recordとして取得

通常のmonitor列挙で得た`DeviceID`は保持します。`EDD_GET_DEVICE_INTERFACE_NAME`で得た`DeviceID`は`DeviceInterfacePath`として別に表示し、通常recordを上書きしません。

Cargo dependencyは引き続き`windows = "=0.62.2"`です。featureは次の2つだけで、Step 5による追加はありません。

- `Win32_Devices_Display`
- `Win32_Graphics_Gdi`

`Win32_Foundation`はこのWin32 feature階層からtransitiveに有効になるため、重複して明示していません。`EDD_GET_DEVICE_INTERFACE_NAME`は`windows 0.62.2`で`Win32_UI_WindowsAndMessaging`配下に公開されていますが、UI API自体は使わないため、確認済みの定数値`0x00000001`をCLI内に限定して定義しています。

### exact mapping規則

source側は、CCD `viewGdiDeviceName`と全GDI adapterの`DISPLAY_DEVICEW.DeviceName`を照合します。target側は、CCD `monitorDevicePath`と全GDI monitorの`DeviceInterfacePath`を照合します。

両target pathはSetupAPIでmonitorへ到達するためのpathとして文書化されていますが、Microsoftの説明だけから、すべてのdriver/OSで両文字列が常に同一とまでは断定しません。ここでの`Exact`は、同じrunで観測した2つの文字列が完全一致したというStep 5の実機evidenceです。

- 最初のNULより前のraw UTF-16 code unit列を完全一致で比較
- `String::from_utf16_lossy`をidentity比較に使用しない
- 大文字・小文字変換、trim、Unicode normalization、slashやprefixの置換、device pathのparseを行わない
- 0件一致は`Unmapped`、複数一致は`Ambiguous`
- sourceとtargetがそれぞれ一意に一致した後、同じ親adapterか、source adapterとtarget monitorがdesktop接続中か、2つのCCD recordのoutput technologyが一致するかを別に検査
- clone topologyで複数pathが同じsource endpointを共有することは許容するが、共有されたsource nameは全recordで一致する必要がある
- 同じtarget endpointが複数pathに現れた場合は統合せず`Inconsistent`
- path/source/targetがactive/in-useでない場合、targetが現在のsessionでavailableでない場合、targetにforced availability flagまたは`friendlyNameForced`がある場合、あるいは未知のtarget-name flagがある場合は`Inconsistent`

friendly nameは人間向けlabelとしてだけ表示します。空のfriendly nameはmonitor device pathのmapping失敗とはみなしません。device interface symbolic linkはopaqueかつsnapshot/session scopedとして扱い、永続IDとして保存しません。device由来の表示文字列に含まれる改行、terminal control、方向制御文字などは、1行の診断出力を偽装できないようescapeします。

GDI列挙の前後をCCD queryで挟み、2回のactive endpoint identity、path/source/target status、target availability、output technologyの集合が一致した場合だけmappingを確定します。一致した場合の表示は、atomicな安定性を意味する`Stable`ではなく`SampledStable`です。2回の観測間で変化して同じ値へ戻るABAや、sampling後の変化までは証明できません。観測値が異なる場合は`StaleSnapshot`と表示し、そのrunでは`Exact`を確定しません。

### 実行手順

リポジトリのルートディレクトリで、これまでと同じコマンドを実行します。

```text
cargo run --manifest-path native/display-probe/Cargo.toml
```

GDI monitor情報には`DeviceInterfacePath`が追加され、最後に次のようなcross-mapが表示されます。

```text
GDI <-> CCD Exact Cross-map
  SnapshotStatus: SampledStable
  Path 0
    SourceMatch: Exact (Adapter 0)
    SourceAttachedToDesktop: true
    SourceEndpointMultiplicity: 1
    SourceEndpointIdentityConsistent: true
    SourceInUse: true
    TargetMatch: Exact (Adapter 0 / Monitor 0)
    ParentAdapterConsistent: true
    TargetAttachedToDesktop: true
    OutputTechnologyConsistent: true
    TargetEndpointMultiplicity: 1
    TargetAvailableForSession: true
    TargetInUse: true
    TargetForcedAvailability: false
    TargetFriendlyNameForced: false
    TargetNameHasUnknownFlags: false
    PathActive: true
    Result: Exact
  Summary: ExactPaths=3 UnmappedPaths=0 AmbiguousPaths=0 InconsistentPaths=0 Stale=false
```

### Step 5の確認項目

1. CLIがWindowsで正常にbuild・実行できる。
2. 各active pathの`SourceGdiDeviceName`が表示され、対応するGDI adapterの`DeviceName`へ一意に`Exact`となる。
3. 各GDI monitorの`DeviceInterfacePath`と各CCD targetの`TargetDevicePath`が表示され、active targetが一意に`Exact`となる。
4. `SourceAttachedToDesktop`、`SourceEndpointIdentityConsistent`、`SourceInUse`、`ParentAdapterConsistent`、`TargetAttachedToDesktop`、`OutputTechnologyConsistent`、`TargetAvailableForSession`、`TargetInUse`、`PathActive`が`true`となる。
5. `TargetForcedAvailability`、`TargetFriendlyNameForced`、`TargetNameHasUnknownFlags`が`false`となる。
6. 通常列挙のmonitor `DeviceID`がStep 1までと同じ意味の値として残り、`DeviceInterfacePath`で上書きされていない。
7. desktop未接続のadapter/monitorがactive CCD pathへ誤って対応付けられない。
8. 実ディスプレイが3台なら、安定したtopologyで例のように`ExactPaths=3`となり、`UnmappedPaths=0`、`AmbiguousPaths=0`、`InconsistentPaths=0`となる。
9. 実行中にdisplayを抜き差しして前後の観測値が異なった場合は、誤った`Exact`ではなく`StaleSnapshot`またはAPI errorになる。短時間で元の値へ戻るABAはこのCLIだけでは検出できないため、hotplug中のrunを完了証拠にしない。
10. 実行前後で解像度、refresh rate、monitor配置などが変化しない。

### Step 5の完了条件

安定したWindows実機環境で、3本のactive pathがすべて一意に`Exact`となることを確認済みです。実機結果は`ExactPaths=3`、`UnmappedPaths=0`、`AmbiguousPaths=0`、`InconsistentPaths=0`、`Stale=false`で、desktop未接続の`DISPLAY4`はcross-mapに含まれませんでした。

通常・安定topologyでのStep 5は完了です。hotplug中の`StaleSnapshot`は追加検証項目として未実施ですが、通常状態の完了条件には含めません。完全一致または整合性検査が成立しない別環境では推測で補完せず、そのsupport cellを`Unmapped`、`Ambiguous`、または`Inconsistent`として記録します。

## Step 6: CurrentObservation統合（Windows実機検証完了）

Step 5で一意にcross-mapできたactive pathごとに、GDIの現在値とCCDのsource/target値を1つのread-only `CurrentObservation`へ統合します。新しいWindows API、crate dependency、`windows` feature、`unsafe`は追加していません。解像度・refresh rate・配置を変更するAPIも使用しません。

Step 6の`Exact`は、今回比較する現在のdesktop解像度とraw refresh rationalの数学的関係が完全一致したことだけを表します。完全な`DEVMODE` tuple、bit depth、HDR/color、preferred mode、物理presentation rate、candidateの適用可能性を証明するものではなく、`canApply`判定にも使用しません。

### 解像度とrotationの規則

GDI側は、Step 2で`DM_PELSWIDTH`と`DM_PELSHEIGHT`が有効な場合だけ保持した`dmPelsWidth` / `dmPelsHeight`を使います。CCD側は`DISPLAYCONFIG_SOURCE_MODE`のwidth / heightを使い、target pathの`DISPLAYCONFIG_ROTATION`を次のように適用してからGDIのdesktop surfaceと比較します。

| CCD rotation | raw値 | source寸法への処理 |
| --- | ---: | --- |
| `Identity` | 1 | width / heightを維持 |
| `Rotate90` | 2 | width / heightを交換 |
| `Rotate180` | 3 | width / heightを維持 |
| `Rotate270` | 4 | width / heightを交換 |

未知のrotation、0を含む寸法、GDI current modeまたはCCD source modeの欠落は`Unavailable`です。known rotation適用後のCCD source寸法とGDI寸法が違う場合だけ`Mismatch`とし、幅と高さを推測で補正しません。この照合規則は、[`DEVMODEW`](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-devmodew)、[`DISPLAYCONFIG_SOURCE_MODE`](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-displayconfig_source_mode)、[`DISPLAYCONFIG_ROTATION`](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ne-wingdi-displayconfig_rotation)のdocumented field semanticsから導くStep 6の判定規則です。

CCD `TargetModeActiveSize`は物理signalのactive領域であり、scaling時にはsource surfaceと異なり得ます。そのためGDI desktop解像度の一致根拠にはせず、raw CCD sourceとの関係を`Exact`、`Distinct`、`Unavailable`として別に表示します。

### integer / rational refreshの規則

次の3関係を独立に比較します。

- GDI integer Hz ↔ CCD path `refreshRate`
- GDI integer Hz ↔ CCD target mode `vSyncFreq`
- CCD path `refreshRate` ↔ CCD target mode `vSyncFreq`

比較はdecimal表示や許容誤差を使わず、正の分子・分母を`u128`へ拡張してcross multiplicationします。したがって`60`と`60/1`、`60`と`120/2`は`Exact`ですが、`60`と`60000/1001`は`Distinct`です。GDIのdriver default値`0` / `1`、未報告field、CCDの`0/0`、分母0、0Hzは`Unavailable`です。

CCD path refreshとtarget VSyncは同じ値へ丸めません。[`DISPLAYCONFIG_PATH_INFO`](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-displayconfig_path_info)で説明されるWindows 11のDynamic Refresh Rateではvirtual / physical refreshが意図的に異なり得るため、有効だが異なる値は破損を意味する`Mismatch`ではなく`Distinct`として保持します。各fieldの意味は[`DISPLAYCONFIG_PATH_TARGET_INFO`](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-displayconfig_path_target_info)と[`DISPLAYCONFIG_VIDEO_SIGNAL_INFO`](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-displayconfig_video_signal_info)に従います。

### snapshotと分類規則

Step 5のmapping evidenceに加え、GDI列挙の前後で取得した2つのCCD snapshotについて、CLIが保持するsource modeの全fieldとtarget modeの全field（pixel rate、H/V sync、active/total size、scanline ordering）、rotation、scaling、path rational refreshをorder-independentに比較します。mode-info indexやpath配列順だけの変化はidentity変化とみなしません。Step 6で保持・判定に使わない`DISPLAYCONFIG_VIDEO_SIGNAL_INFO`のanonymous unionは、ここでいうtarget mode fieldには含みません。特にMiracastの`vSyncFreqDivider`は解釈していないため、Step 6の`Exact`をMiracast supportまたはphysical presentation rateの保証として扱いません。

- mapping evidenceが変化した場合、Step 5とStep 6を`StaleSnapshot`にする
- mappingは同じでも前後CCD current-mode evidenceが変化した場合、Step 6だけを`StaleSnapshot`にする。Step 7追加後はGDI current full tupleもmode列挙の前後で比較し、こちらが変化した場合はGDI値を確定せずStep 6関係を`Unavailable`にする
- API error、判定に必要な値の欠落、未知のrotation、非`Exact` mappingは`Unavailable`にする。個別fieldに`Mismatch`があっても必要値が欠ける場合、全体の`Result`は`Unavailable`を優先する
- clone topologyは1つのsourceを複数target pathが共有し、targetごとにrotationが異なり得るため、Step 6では`CloneSourceNotQualified`として`Unavailable`にする
- 必要値がすべて揃い、knownなrotation適用後のdesktop解像度が違う場合は`Mismatch`にする
- 解像度は一致し、有効なrefreshまたはsource/target signal寸法が数学的に異なる場合は`Distinct`にする
- 必要な全関係が完全一致した場合だけ`Exact`にする

`SampledStable`はatomic snapshotを意味しません。2回の観測間で変化して元の値へ戻るABAと、2回目の観測後の変化は検出できないため、hotplug中のrunを完了証拠には使用しません。

### 実行手順と出力例

Windows上のリポジトリルートで実行します。

```text
cargo test --manifest-path native/display-probe/Cargo.toml
cargo run --manifest-path native/display-probe/Cargo.toml
```

縦置きdisplayでは、次のようにCCD sourceへrotationを適用した寸法とGDI desktop寸法が一致することを確認します。

```text
GDI / CCD Current Observations
  SnapshotStatus: SampledStable
  Scope: current resolution/refresh relations only
  Path 1
    Mapping: Adapter 1 / Monitor 0
    DeviceName: \\.\DISPLAY2
    FriendlyName: TW215FHDNS
    Rotation: Rotate270 (4)
    ScalingRaw: 1
    GdiDesktopResolution: 1080x1920
    CcdSourceResolution: 1920x1080
    RotationAppliedSourceResolution: 1080x1920
    DesktopResolutionRelation: Exact
    CcdTargetActiveResolution: 1920x1080
    CcdSourceVsTargetActive: Exact
    GdiRefresh: 60 Hz (integer)
    CcdPathRefresh: 60/1 (60.000000 Hz)
    CcdTargetVSync: 60/1 (60.000000 Hz)
    GdiVsCcdPathRefresh: Exact
    GdiVsCcdTargetVSync: Exact
    CcdPathVsTargetVSync: Exact
    Result: Exact
  Summary: ExactPaths=3 DistinctPaths=0 MismatchPaths=0 UnavailablePaths=0 Stale=false
```

`ScalingRaw`などのraw値は環境によって異なります。例の値そのものではなく、Windows Settingsと物理配置に対する関係を確認してください。

### Step 6の確認項目

1. unit testが成功し、CLIがWindowsで正常にbuild・実行できる。
2. Step 5とStep 6の`SnapshotStatus`が安定したtopologyで`SampledStable`となる。
3. 横置きの`DISPLAY1` / `DISPLAY3`でGDI寸法、CCD source寸法、rotation適用後寸法が期待どおりとなる。
4. 縦置きの`DISPLAY2`で、CCD source `1920x1080`と`Rotate270 (4)`から`1080x1920`が得られ、GDI `1080x1920`と`Exact`になる。
5. GDI Hz、CCD path rational、CCD target VSyncのraw分子・分母が3関係として別々に表示される。
6. `60/1`や`144/1`は対応するGDI整数Hzと`Exact`になり、近似やdecimal丸めが使われていない。
7. 通常の3台・extend topologyで`ExactPaths=3`、`DistinctPaths=0`、`MismatchPaths=0`、`UnavailablePaths=0`、`Stale=false`となる。
8. mode変更またはhotplugと重なったrunは`StaleSnapshot`またはAPI errorとなり、そのrunの値を成功証拠にしない。
9. 実行前後で解像度、refresh rate、monitor配置などが変化しない。

### Step 6の完了条件

通常の3台・extend環境でWindows実機検証済みです。Step 6実装時点のunit testは`10 passed; 0 failed`で、Step 5 / Step 6はいずれも`SnapshotStatus: SampledStable`となりました。横置きの`DISPLAY1` / `DISPLAY3`、縦置きの`DISPLAY2`でrotation適用後の寸法が一致し、GDI `144` / `60`とCCD `144/1` / `60/1`の関係もすべて`Exact`でした。

実機summaryは`ExactPaths=3`、`DistinctPaths=0`、`MismatchPaths=0`、`UnavailablePaths=0`、`Stale=false`です。実行前後で解像度、refresh rate、配置に変化がないことも目視確認済みとして、通常環境のStep 6 baselineは完了です。Step 7でGDI currentをfull tupleのbefore / after samplingへ強化した現HEADでも同じsummaryとなり、回帰がないことを確認済みです。59.94/60、DRR、clone、hotplugなどの追加support cellは、通常の3台・extend環境とは分けてStep 8でfail-closed分類します。

## Step 7: `DEVMODEW` candidate model（Windows実機検証完了）

Step 7では、Step 3のnormal mode列挙結果をwidth / height / Hzだけの表示値として扱わず、`dmFields`のpresenceを含むallowlisted [`DEVMODEW`](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-devmodew) tupleとして保持します。duplicate、同じ表示labelを持つ別tuple、current-not-listed、不完全field、currentとのorientation / color policy差を、推測せず別々に分類します。

このStepは引き続き読み取り専用です。新しいWindows API、crate dependency、`windows` featureは追加していません。`windows = "=0.62.2"`と`Win32_Devices_Display` / `Win32_Graphics_Gdi`の2 featureを継続し、使用するmode APIは既存の[`EnumDisplaySettingsExW`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsexw)だけです。current取得とindex列挙はいずれもflags `0`です。

次は使用しません。

- `ENUM_REGISTRY_SETTINGS`
- `EDS_RAWMODE`
- `EDS_ROTATEDMODE`
- `ChangeDisplaySettings*` / `CDS_*`
- `SetDisplayConfig` / `SDC_*`
- registry write、PowerShell、cmd、process spawn

### 保持するallowlisted tuple

各current / indexed recordから次をRustの値へコピーします。inactiveなunion field、raw structure bytes、pointer、`dmDriverExtra`のprivate bytesは保持しません。

- raw `dmFields`
- `dmPosition`
- `dmDisplayOrientation`
- `dmDisplayFixedOutput`
- `dmBitsPerPel`
- `dmPelsWidth` / `dmPelsHeight`
- raw `dmDisplayFlags`
- raw `dmDisplayFrequency`
- 返却された`dmSize` / `dmDriverExtra`（envelope検査用の整数値のみ）

field valueは対応する`dmFields` bitがある場合だけ採用します。optional fieldのabsenceを`0`、default orientation、default fixed outputなどへ変換しません。frequency `0`と`1`も同じ値へ潰さず、それぞれraw driver-default markerとして保持します。

`dmBitsPerPel`、width、height、display flags、frequencyのpresenceを、DisplayDeckがnormal candidateを完全と認定する保守的な必須条件にします。これはMicrosoftがすべてのdriverへ保証する返却maskではありません。欠落recordも観測結果として残し、API失敗ではなく`Incomplete` / `HardExcluded`とします。

次の場合も`Incomplete`です。

- `dmSize`が期待するpublic `DEVMODEW` sizeと異なる
- `dmDriverExtra != 0`
- allowlist外の`dmFields` bitがある
- 必須fieldの欠落、またはpresence bitに対応する値をcaptureできない
- bits per pixel、width、heightが0
- orientationが`0..=3`以外、fixed outputが`0..=2`以外
- `dmDisplayFlags`にMicrosoft SDK / `windows 0.62.2` bindingでknownなvalue mask外のbitがある。legacy grayscaleまたはtext-mode bitはknownでも初期対象外としてhard excludeする

`dmDisplayFrequency`が`0`または`1`でもtuple自体のpresenceは完全に保持できますが、具体的なHzやexpected readbackを確定できないため`HardExcluded`です。`dmColor`はprinter fieldなのでdisplay color evidenceには使用しません。bits per pixelがcurrentと同じでもHDR、色空間、bits per channel、advanced colorの保持は証明できず、`AdvancedColorEvidence: NotObserved`と表示します。

### identity、duplicate、current membership

同じrun内の`EnumerationProvenance`、人間向け`DisplayLabel`、`ApplyTuple`、`ExpectedObservation`を別modelにしています。adapter / enumeration indexだけではsnapshotやtupleへ再解決できないため、これを`CandidateIdentity`とは呼びません。Step 7の`CandidateIdentity`とselection tokenは明示的に`NotIssued`です。

- full tuple equalityはraw field mask、field presence、allowlisted valueをすべて完全一致で比較する
- 同じ完全tupleが複数indexにある場合は、recordを削除せず`ExactTupleDuplicate`とする
- width / height / raw frequencyのlabelが同じでも、bits per pixel、flags、position、orientation、fixed output、presenceが違えば`ProjectionCollision`とし、統合しない
- currentはnormal listとfull tupleで比較し、1件だけなら`ListedUnique`、複数なら`AmbiguousExactRecords`、0件なら`NotListedExact`とする
- visible labelだけ一致するrecordは`projection-only`診断として残すが、current exact matchへ昇格させない
- 4096 recordの上限に達した場合は列挙不完全とし、current-not-listedを断定しない

各adapterではcurrent-before → normal list → current-afterの順で読み取ります。前後current tupleが違う場合は`ChangedDuringCapture`として候補をfail closedにします。normal list自体は1回だけ列挙するため、出力はcandidate list全体のatomic stabilityを主張しません。ABAや列挙後の変更もStep 5 / 6と同様に残るsampling上の限界です。

adapterはindex `0..=31`、monitorは各adapterで`0..=31`、normal modeは各adapterで`0..=4095`だけを呼び出します。許可範囲をすべて使った場合は次のindexをprobeせず`LimitReached`とし、該当inventory / candidateをfail closedにします。これによりadapter最大32、monitor最大32/adapter・合計最大1024、mode最大4096/adapterのread-side allocation boundを維持します。

### policyとeligibility

currentとcandidateのposition、orientation、fixed output、bits per pixel、display flagsをfieldごとに`Exact`、`Different`、`NotReported`、`PresenceMismatch`で比較します。absenceや違いを補完せず、`Exact`以外はこのStepの候補を`HardExcluded`にします。desktop未接続adapter、32 bpp未満、列挙empty/unavailable、列挙上限到達もhard exclusionです。

tupleが完全でcurrentがnormal listへ一意に含まれ、policy fieldがすべて`Exact`でも、candidate reportはStep 5のexact target mappingやsupport fingerprintへまだbindされず、非current candidateのCCD rational / source / target readbackも取得できません。そのためStep 7が生成できる最も強い分類は`LabUnqualified`で、これらをqualification gapとして明示します。`ProductAllowed`は常に0です。selection token、`canApply=true`、`DEVMODEW`再構築、preflight、display変更は実装しません。

### `unsafe`

追加した`unsafe`は、返却された`DEVMODEW`のdocumented display unionを読む2箇所だけです。

- `Anonymous1.Anonymous2`からposition / orientation / fixed outputをコピー
- `Anonymous2.dmDisplayFlags`からraw display flagsをコピー

どちらも対応する`dmFields` bitを先に確認し、全bit patternが有効な整数/wrapperを返却後のborrow中にコピーします。raw unionやpointerをsafe domain modelへ公開しません。既存FFI callのflagsは引き続き読み取り専用の`0`です。

### 実行手順と出力

Windows上のリポジトリルートで実行します。

```text
cargo test --manifest-path native/display-probe/Cargo.toml
cargo run --manifest-path native/display-probe/Cargo.toml
```

Step 7追加後のunit testは、既存Step 6の10件とcandidate modelの15件、合計25件です。標準出力末尾に次のsectionが追加されます。tuple groupは全recordを1回だけ保持し、recordごとに全peer indexを複製しません。詳細candidateは全adapter合計1024件、legacy mode行とgroup indexはそれぞれ合計8192件まで標準出力へ展開します。省略分もmodel、summary、group countには含めるため、境界入力でも重複分類のmemory / stdoutが二次増加しません。

```text
GDI Mode Candidate Classification
  CaptureScope: one bounded normal-mode enumeration (flags=0), bracketed by current-mode samples
  CandidateListStability: not claimed (single enumeration)
  Mutation: disabled; ProductAllowed=0; SelectionTokens=0
  DetailedRecordOutputLimit: 1024 total
  GroupIndexOutputLimit: 8192 total
  AdapterEnumerationStatus: Complete
  Adapter 0
    DeviceName: \\.\DISPLAY1
    MonitorEnumerationStatus: Complete
    EnumerationStatus: Complete
    CurrentTupleStatus: Complete
    CurrentMembership: ListedUnique (Mode ...)
    CandidateRecords: ...
    ExactDuplicateGroup ...: Modes ...
    ProjectionCollisionGroup ...: Modes ...
    Mode 0
      EnumerationProvenance: adapter=0 enumerationIndex=0
      CandidateIdentity: NotIssued (read-only Step 7)
      DisplayLabel: 640x480 @ 60 Hz (raw integer)
      ApplyTuple: dmSize=... dmDriverExtra=0 dmFields=0x... position=... orientation=... fixedOutput=... bitsPerPixel=... size=640x480 displayFlags=0x... frequency=60 Hz (raw integer)
      TupleStatus: Complete
      ExactDuplicate: ...
      ProjectionCollision: ...
      CurrentRelation: ...
      PolicyRelations: ...
      AdvancedColorEvidence: NotObserved
      ExpectedObservation: Missing (...)
      Eligibility: LabUnqualified (...) または HardExcluded (...)
      SelectionToken: NotIssued (read-only Step 7)
    Summary: ... ProductAllowed=0 SelectionTokens=0
```

### Step 7の確認項目

1. `cargo test`で25件すべて成功し、CLIがWindowsでbuild・実行できる。
2. adapter / monitorのenumeration statusを保存する。active adapterではmode enumerationと`CurrentTupleStatus`が`Complete`になることを確認し、desktop未接続adapterの`EmptyOrUnavailable`はそのまま保持する。active / inactiveを問わずmode上限到達はcompleteへ昇格しない。
3. currentがfull tupleで1件、複数、0件のどれに分類されたかを、projection-only一致と分けて確認する。
4. 既存の見た目が同じmodeについて、`ExactTupleDuplicate`か`ProjectionCollision`かをfull tupleで説明できる。
5. `dmFields`、position、orientation、fixed output、bits per pixel、display flags、raw frequencyがpresence込みで表示される。
6. incomplete / unknown recordが推測で`Complete`へ昇格しない。raw driver-default `0` / `1`はpresence込みtupleとして`Complete`になり得るが、具体Hzまたはeligibleへ昇格せず`HardExcluded`になる。
7. candidateごとのpolicy relationと`ExpectedObservation: Missing`の理由が表示される。
8. summaryが常に`ProductAllowed=0`、`SelectionTokens=0`である。
9. `CandidateIdentity`とselection tokenが発行されず、mapping / support fingerprint / expected observationの不足がqualification gapとして残る。
10. 現在値の取得経路もbefore / after full tuple samplingへ変わったため、現HEADでStep 5 / 6も再確認し、通常の3台環境でStep 5 `ExactPaths=3`、Step 6 `ExactPaths=3` / `UnavailablePaths=0`の回帰がない。
11. 実行前後で解像度、refresh rate、monitor配置などが変化しない。

### Step 7の完了条件

Windows実機で`cargo test`の25件がすべて成功し、`cargo build`とCLI実行も完了しました。完全ログでStep 5は`ExactPaths=3`、Step 6は`ExactPaths=3` / `UnavailablePaths=0`のままで、Step 7追加後の回帰がないことを確認済みです。縦置き`DISPLAY2`もCCD source `1920x1080`へ`Rotate270`を適用した`1080x1920`がGDI currentと`Exact`になりました。

Step 7の実機結果は次のとおりです。

```text
AdapterEnumerationStatus: Complete
Mutation: disabled; ProductAllowed=0; SelectionTokens=0

DISPLAY1 CurrentMembership: NotListedExact (projection-only Mode 315)
DISPLAY2 CurrentMembership: NotListedExact (projection-only Mode 18)
DISPLAY3 CurrentMembership: NotListedExact (projection-only Mode 18)

Summary: Records=619 Complete=619 Incomplete=0
         ExactDuplicateGroups=0 ExactDuplicateRecords=0
         ProjectionCollisionRecords=591
         LabUnqualified=0 HardExcluded=619
         ProductAllowed=0 SelectionTokens=0
```

このWindows実機観測環境では、currentと同じwidth / height / integer Hzを持つrecordは存在しても、presenceを含むfull tupleが一致するnormal candidateはありませんでした。`ExactTupleDuplicate`は発生せず、同じ表示labelを持つ異なるtupleである`ProjectionCollision`を591件観測しました。CLIはprojection-only一致をcurrent candidateへ昇格せず、619件すべてを`HardExcluded`にしており、Step 7のfail-closed規則が実機でも機能しています。

通常の3台・extend環境に対するStep 7は完了です。この結果はcandidate policyを緩和する根拠ではなく、このWindows実機観測環境ではmutation候補を発行できないことを示します。3本のdistinct-source active pathによる複数display構成はStep 8でfail-closed evidenceとして扱い、初期MVPのmutation対象外とします。59.94/60、hotplug、RDP、virtual、clone / shared-sourceの追加探索は、具体的なsupport要求が出るまで行いません。

## Step 8: read-only support assessment（CLI milestone完了・履歴）

Step 8では、Step 5のexact cross-map、Step 6のcurrent observation、Step 7のcandidate catalogを、文字列出力ではなく同じtyped reportから集約します。目的は、現在の観測環境で確認できたnegative evidenceと未観測gapを区別し、初期mutation対象へ誤って昇格させないことです。

これは診断用のfail-closed precheckです。`Supported`、`Qualified`、`canApply`に相当する型や分岐は持ちません。すべての実行で次を固定します。

```text
MutationReadiness: Blocked
MutationAllowed: false
ProductAllowed: 0
SelectionTokens: 0
G1AGate: NotReadyEvidenceGaps
Phase1AClosure: NotClaimed
```

新しいWindows API、crate dependency、`windows` feature、`unsafe`は追加していません。既存のread-only APIだけを使用し、display変更、registry write、PowerShell、cmd、process spawnは実装しません。

### typed captureとfail-closed規則

Step 5 / 6のprint関数が成功・失敗理由を失わないよう、次のtyped outcomeへ変更しました。Step 8は標準出力をparseせず、この値を直接使用します。

- `MappingCapture::SampledStable(CrossMap)`または`Unavailable(reason)`
- `ObservationCapture::SampledStable(CurrentObservationReport)`または`Unavailable(reason)`

adapter / monitor / mode列挙上限、initial / verification CCD API error、stale snapshot、cross-map未確定はそれぞれ別reasonのまま伝播します。CCD errorは`ConsoleOrDesktopAccessDenied`、`BoundExceeded`、`TopologyRace`、`UnsupportedNativeEvidence`、`InvalidNativeEvidence`、`ApiError`へ正規化します。`ERROR_ACCESS_DENIED`はconsoleまたはcurrent desktopへaccessできない、あるいはremote sessionの可能性があるという広いnegative evidenceであり、RDP確定とは表示しません。invariant不一致、API unavailable、stale、active path 0本または2本以上、clone / shared source、non-exact mapping / observation、特殊・未知output technology、positive GDI remote marker、current-not-listed、候補0件またはactive adapterにlab候補がない状態を成功へ補正しません。

`DISPLAY_DEVICEW.StateFlags`はraw値と、mirroring driver / remote / RDPUDDのSDK markerを保持します。adapterとmonitorで意味が重なるbitをcontext別に判定し、初期cellで許可していない既知bitとSDK mask外の未知bitもmarkerにします。さらに、GDIでdesktop接続と報告されたadapter / monitorのbounded bitsetと、CCD exact source / target mappingの逆向きcoverageを比較します。片側にしかないattached deviceや重複indexを整合済みと扱いません。markerが存在する場合はfail closedですが、markerがないことをsingle local console sessionの証明にはしません。`TS_COMPATIBLE`などのSDK marker名からremote sessionを断定しません。CCD output technologyのSDTV dongle、Miracast、indirect wired、indirect virtual、DisplayPort USB tunnel、`OTHER`、reserved / 未知値もunqualified markerです。通常のHDMI / DisplayPort等を観測してもphysical support fingerprintが証明されたとは扱いません。

CCD path / source / targetのstatus flag、rotation、scaling、pathとtarget modeのscan-line ordering、source pixel formatもallowlistで検査します。legacy precheckではpath flagは`ACTIVE`のみ、source statusは`IN_USE`のみ、target statusは`IN_USE`のみ、rotationは1〜4、scalingは通常値1〜4、scan-line orderingはprogressive、pixel formatは32bppだけをmissing-evidence-only経路へ通します。DRR boost、preferred-unscaled、forced target、HMD、custom / preferred scaling、unspecified / interlaced scan-line、8/16/24bpp、`NONGDI`、未知bit / enumは観測済みunsupported evidenceとしてfail closedにします。これは値からvirtual deviceやremote sessionを推測するものではありません。

現行CCD queryはStep 4から継続してlegacyな`QDC_ONLY_ACTIVE_PATHS`です。返却pathに`DISPLAYCONFIG_PATH_SUPPORT_VIRTUAL_MODE`がある場合、virtual-aware unionをlegacy `modeInfoIdx`として読む前にrunを拒否します。未知path flagもunion read前に拒否します。

assessmentの総合状態は次の3種類です。どの状態でも`MutationReadiness: Blocked`、`MutationAllowed: false`、`ProductAllowed: 0`、`SelectionTokens: 0`、`Phase1AClosure: NotClaimed`です。

| Disposition | 意味 |
| --- | --- |
| `NotAssessable` | API結果または内部構造の完全性を信頼できず、判定不能 |
| `RejectedByObservedEvidence` | 安全にdecodeできたnegative / unsupported / unknown evidence、race、access denial、上限超過などを観測 |
| `BlockedByMissingEvidence` | 限定したread-only観測は整合したが、正式qualificationに必要な固定gapが残る |

内部invariant不一致やnative構造不整合とnegative blockerが同時にある場合は`NotAssessable`を優先し、個別blockerのdetailは失わず保持します。

設計資料の旧Phase 1A first-cell候補surfaceは`QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_MODE_AWARE`と別のallocation contractを要求していました。Step 8では、このflag変更とvirtual-aware index decodeを追加せず、`ApprovedCcdSurfaceNotImplemented`として明示しました。初期MVPはこの未実装surfaceを必要とするcellをsupport外にし、追加captureだけを目的とした別Stepは作りません。将来そのcellをsupportするときだけ[`QDC_VIRTUAL_MODE_AWARE`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)を実装・検証します。

### scenario分類

現在のAPI surfaceから、次を互いに独立して集計します。

- active path 0 / 1 / 複数と、同一source endpointを共有するclone path
- portrait rotationを観測したpath数と、そのうちStep 6 `Exact`となったpath数
- positive non-integral CCD rationalと、整数GDI Hzに対して`Distinct`を維持した比較数
- candidate総数と9件以上のadapter
- `CurrentMembership::NotListedExact`となったadapter
- candidateのfixed-kind `HardExclusion` reason histogram
- Miracast / indirect / unknown output technologyとGDI remote関連marker
- DRR / HMD / status flag / rotation / scaling / scan-line / pixel-formatの既知unsupportedまたは未知native marker
- mapping / observation / candidate summary、path index / location、source / target endpoint multiplicity、GDI attached deviceのreverse coverage、provenance、read-only counterの内部invariant
- exact source mappingに現れるactive adapter自身が持つ`LabUnqualified`候補数。active外adapterの候補ではこの条件を満たさない

`60`と`60000/1001`は引き続き近似一致にしません。stableな2回のsnapshotはhotplug不在の証明ではないため、hotplugは未qualification gapのままです。API成功もRDP、Fast User Switching、second interactive logonがないことを証明しません。

preferred mode、persisted baseline、advanced color / HDR、DRR / virtual refresh、exact expected observation、candidate-target binding、support fingerprintは、現在の承認済み実装surfaceでは取得または確定していません。値を推測せずevidence gapとして表示します。

### 実行手順と期待する通常環境の判定

Windows上のリポジトリルートで実行します。

```text
cargo fmt --manifest-path native/display-probe/Cargo.toml
cargo fmt --manifest-path native/display-probe/Cargo.toml -- --check
cargo test --manifest-path native/display-probe/Cargo.toml
cargo build --manifest-path native/display-probe/Cargo.toml
cargo run --manifest-path native/display-probe/Cargo.toml
```

完全な検証ログを残す場合は、Windows PowerShellで次を実行します。各`cargo`実行直後の`$LASTEXITCODE`を確認し、非0ならそこで検証を停止します。

```powershell
cargo fmt --manifest-path native/display-probe/Cargo.toml -- --check
if ($LASTEXITCODE -ne 0) { throw "cargo fmt --check failed: $LASTEXITCODE" }

cargo test --manifest-path native/display-probe/Cargo.toml 2>&1 |
    Tee-Object -FilePath step8-windows-tests.txt
if ($LASTEXITCODE -ne 0) { throw "cargo test failed: $LASTEXITCODE" }

cargo build --manifest-path native/display-probe/Cargo.toml 2>&1 |
    Tee-Object -FilePath step8-windows-build.txt
if ($LASTEXITCODE -ne 0) { throw "cargo build failed: $LASTEXITCODE" }

cargo run --manifest-path native/display-probe/Cargo.toml 2>&1 |
    Tee-Object -FilePath step8-windows-validation.txt
if ($LASTEXITCODE -ne 0) { throw "cargo run failed: $LASTEXITCODE" }
```

保存したログから主要な完了条件を抽出します。

```powershell
Select-String -Path step8-windows-tests.txt -Pattern 'test result: ok. 55 passed'

$step8Patterns = @(
    'SnapshotStatus: SampledStable'
    'Summary: ExactPaths=3'
    'GdiActiveCoverage: Assessed'
    'consistent: true'
    'Disposition: RejectedByObservedEvidence'
    'MutationAllowed: false'
    'ProductAllowed: 0'
    'SelectionTokens: 0'
    'Phase1AClosure: NotClaimed'
)
Select-String -Path step8-windows-validation.txt -Pattern $step8Patterns
```

`Select-String`は存在確認用です。件数や前後関係を省略せず確認するため、最後に`step8-windows-validation.txt`全体も保存・確認してください。Windows PowerShellでは、Cargoがstderrへ出す正常なstatus行を`2>&1`で取り込んだ際に`NativeCommandError`形式で表示される場合があります。その表示だけで失敗とは判定せず、`$LASTEXITCODE`、`test result`、各summaryを併せて確認します。

上記3個の`step8-windows-*.txt`は実機検証用のローカルartifactであり、Gitへ追加しません。実行前後にWindows Settingsで解像度、refresh rate、monitor配置が変化していないことも目視確認します。

Step 8追加後のunit testは合計55件です。通常の3台・extend観測環境では、末尾に概ね次の判定が追加されます。

```text
Read-only Support Assessment
  Scope: diagnostic fail-closed precheck only
  CellIdentity: NotIssued (read-only Step 8)
  SupportFingerprint: NotIssued
  CcdQuerySurface: legacy QDC_ONLY_ACTIVE_PATHS
  ApprovedCcdSurfaceImplemented: false
  MappingCapture: SampledStable
  ObservationCapture: SampledStable
  ActivePaths: MultipleActivePaths { count: 3 }
  InventoryComplete: true
  CurrentTupleCaptureComplete: true
  PortraitRotation: observed=1 exact=1
  PositiveNonIntegralRefresh: comparisons=0 distinct=0
  CcdNativeEvidenceMarkers: ...all zero...
  GdiEnvironmentMarkers: attachedAdapters=3 attachedMonitors=3 mirroring=0 remoteSdk=0 rdpuddSdk=0 knownUnqualifiedStateFlags=0 unknownStateFlags=0
  GdiActiveCoverage: Assessed { attached_adapters: 3, exact_source_adapters: 3, attached_monitors: 3, exact_target_monitors: 3, consistent: true }
  CandidateVolume: NineOrMore { count: 619 }
  CurrentNotListedAdapters: 0,1,2
  Candidates: records=619 labUnqualified=0 activeAdapterLabUnqualified=0 hardExcluded=619
  Blockers:
    MultipleActivePaths { count: 3 }
    CurrentNotListedAdapters { count: 3 }
    NoActiveAdapterLabUnqualifiedCandidates { active_adapters: 3, hard_excluded: 619 }
  Disposition: RejectedByObservedEvidence
  MutationAllowed: false
  ProductAllowed: 0
  SelectionTokens: 0
  G1AGate: NotReadyEvidenceGaps
  Phase1AClosure: NotClaimed
```

ここでの`RejectedByObservedEvidence`はCLI実行失敗ではありません。3 active paths、3 adapterの`NotListedExact`、619件all-hard-excludedという観測済みnegative evidenceにより、この環境へmutation候補を発行しないという期待結果です。desktop未接続`DISPLAY4`のmodeが`EmptyOrUnavailable`でも、active source mappingに含まれなければ`CurrentTupleCaptureComplete`をfalseにはしません。ただしactive / inactiveを問わずmode列挙が上限へ達した場合はinventory incompleteとして拒否します。

### Step 8の確認項目

1. Windowsで55件のunit test、build、CLI実行が成功する。
2. Step 5 / 6 / 7のsummaryに回帰がない。
3. 通常の3台環境で`MultipleActivePaths { count: 3 }`、portrait `observed=1 exact=1`、candidate 619件、current-not-listed adapter 3件が表示される。
4. `CurrentNotListed`のhard exclusion reasonが候補ごとに失われず、fixed histogramへ有界に集計される。
5. `Disposition: RejectedByObservedEvidence`、`MutationAllowed: false`、`ProductAllowed: 0`、`SelectionTokens: 0`となる。
6. remote / virtual markerがない通常runでも、local physical / single-sessionを証明したと表示しない。
7. stable runをhotplug検証成功と表示せず、ABAとsampling後の変化を未検証gapとして残す。
8. access denied、上限、race、未対応layout、invalid native evidence、ordinary API errorが同じgeneric errorへ潰れずtyped statusへ残る。
9. GDI attached adapter / monitorとCCD exact source / targetのreverse coverageが一致し、余分・欠落・重複があれば拒否される。
10. DRR boost、HMD、unknown status / enum / GDI state bitなどが`BlockedByMissingEvidence`へ昇格しない。
11. `ApprovedCcdSurfaceNotImplemented`、session / RDP、candidate binding、expected observation、support fingerprint、preferred / persisted / HDR / DRR、formal evidence bundle等のgapが表示される。
12. 実行前後で解像度、refresh rate、monitor配置などが変化しない。

### Step 8の実機検証結果

Windows実機で`cargo fmt --check`、55件のunit test、build、CLI実行が成功しました。Step 5は3 pathすべて`Exact`、Step 6は`ExactPaths=3` / `UnavailablePaths=0`、Step 7は619 candidatesのままで回帰はありません。Step 8では次を確認しました。

- `MultipleActivePaths { count: 3 }`、portrait `observed=1 exact=1`
- GDI attached adapter / monitorとCCD exact source / targetがそれぞれ3 / 3で、reverse coverageは`consistent: true`
- CCD native markerおよびGDI mirroring / remote / RDPUDD / unknown state markerは0。ただし、その不在をlocal physical / single-sessionの証明には使用しない
- 619 candidatesすべて`HardExcluded`で、`CurrentNotListed=619`、`PolicyDifferent=394`、`PolicyEvidenceUnavailable=844`、reason合計1857
- 全internal invariantがtrueで、`Disposition: RejectedByObservedEvidence`、`MutationAllowed: false`、`ProductAllowed: 0`、`SelectionTokens: 0`
- 実行前後で解像度、refresh rate、portrait配置を含む3台のmonitor設定に変化がない

以上によりStep 8のWindows実機検証と実装は完了です。実行当時のCLI出力は`Phase1AClosure: NotClaimed`でしたが、2026-08-24の最短roadmapでは別のPhase 1A/G1A証跡作成を廃止し、既存Step 1〜8をGate Aの一括判断入力として扱います。追加log整形、machine manifest、call trace、evidence bundleだけを目的とした再実行はしません。

## 初期リリースまでの最短roadmap

Step 1〜8のread-only CLI実装とWindows実機観測は完了しました。これ以上read-only探索Stepを増やしません。Gate Aは2026-08-24に承認済みで、Stage 1のTauri app、UI、独立watchdog、fake one-shot worker、dual-slot decision journal、operational WAL、deadline、fencingを実装しました。display mutation、actual machine-dataへの書込み、installerと配布は未実装です。

Step 9で作成したCandidate 04の590 vector、hash/index、再現生成、独立static review、D07 No-Go、D08の25件観測は履歴として保持します。schemaを変更しない限り、fixtureの再生成、手動hash再計算、D08追加batch、Fast Startup/hibernate再検証、別G1A bundle作成は行いません。安全性は実装後の実コードと一つのmutation gateで確認します。

| Stage | 作るもの | 完了条件 |
| --- | --- | --- |
| 0 | MVP範囲と実装baselineの一括承認 | Candidate 04、D08 lab candidate、D07 mutation前Go/No-Go、Stage 1のnon-mutating実装を一度に承認 |
| 1（完了） | read-only Tauri製品 + fake backendの安全core | current/candidate UI、watchdog、one-shot worker、WAL、deadline/fencingをdisplay変更なしで自動test |
| 2 | 製品構成で一つのcontrolled transition | Keep、manual Revert、timeout、parent loss、worker/watchdog failure、startup recoveryを一つのexact cellで確認 |
| 3 | MVP仕上げ + NSIS | UI基本品質、support statement、clean install/launch/uninstall、RC safety smoke |

承認はStage 0、exact mutation run、MVP releaseの3回だけです。UI、read-only統合、watchdog prototype、個別fixture、G1A/G2Aを別々の承認にしません。

初期MVPはlocal consoleの単一user・単一active physical display path・解像度/refreshの非永続変更だけです。multi-display、RDP/FUS、scale、HDR/color、DLDSR、virtual display、arm64、MSI、update/repair、広いhardware matrixは完成後のbacklogへ移しました。D07またはactual mutationがNo-Goなら、安全性を下げずread-only appとして完成させます。

Stage 1はWindows実機smokeまで完了しました。次の実装はGate Bの明示承認後にだけ開始します。詳細と安全上削らない契約は[`docs/implementation-plan.md`](docs/implementation-plan.md)を正本とします。

## Stage 1 Windows実機確認

この確認はread-only表示とfake transactionだけを実行します。Windowsの解像度、refresh rate、配置、registry、実machine-dataは変更しません。D08の追加測定、Candidate 04 fixtureの再生成、G1A資料作成も不要です。

### 1. buildと自動test

Windows PowerShellでDisplayDeckのルートへ移動し、次を上から一度だけ実行します。

```powershell
git pull --ff-only
npm.cmd ci
cargo fmt --all -- --check
cargo test --workspace --all-targets
npm.cmd run build
cargo build -p displaydeck-safety --bin displaydeck-actor --release
npm.cmd run tauri build -- --no-bundle
```

途中で終了コードが非0になった場合は、そこで停止して出力を共有してください。すべて成功したら、次の2ファイルが同じdirectoryにできます。

```text
target\release\displaydeck-app.exe
target\release\displaydeck-actor.exe
```

### 2. read-only appの確認

同じPowerShellでappを起動します。

```powershell
.\target\release\displaydeck-app.exe
```

次の4点を一度確認します。

1. 現在のdisplay、mode、候補または変更不能理由が表示される。
2. 画面上部に「読み取り専用」と表示され、`Apply（未許可）`が押せない。
3. `15秒の安全動作をシミュレート`を一度押し、`戻す`または`この状態を維持`でfake transactionが終了する。
4. Windowsの解像度、refresh rate、配置が変わっていない。

frontend終了時のfake rollback、timeout、WAL/journal破損、stale worker、deadline/fence不一致は上の自動testで確認するため、手動で繰り返しません。fake transactionの一時fileは`$env:TEMP\DisplayDeck-Stage1`だけに作成されます。

### 3. 実機確認結果（完了）

2026-08-24にWindows実機で次を確認しました。

- commit `0aee9e8`をFast-forwardで取得
- `npm.cmd ci`成功（27 packages、報告された脆弱性0件）
- format check、Rust 64 tests、TypeScript / Vite buildがすべて成功
- release build成功。`displaydeck-app.exe`（8,981,504 bytes）と`displaydeck-actor.exe`（669,184 bytes）を生成
- read-only表示、disabled Apply、fake transactionの終了を確認
- 実行前後でWindowsの解像度、refresh rate、配置に変化なし

以上によりStage 1は完了です。`com.displaydeck.app`の末尾に関するTauri warningはWindows buildを失敗させておらず、Gate B前の修正条件にはしません。

## Gate B準備（未承認・read-only）

Stage 2とdisplay mutationはまだ未承認です。Gate Bではexact Windows / GPU driver / monitor / connection / session cellと、一つのtemporary transitionだけを一括承認します。現環境のrecommended candidateは、primaryの`MSI MAG342CQ`を唯一のactive physical display pathにした状態での`3440x1440 / 144 Hz → 60 Hz`です。これは承認候補であり、この時点ではmonitor構成や設定を変更しません。

不足しているexact cell情報を一度だけ取得します。Windows PowerShellでDisplayDeck rootから実行し、出力をそのまま共有します。

```powershell
$ErrorActionPreference = 'Stop'

Get-CimInstance Win32_OperatingSystem |
    Format-List Caption, Version, BuildNumber, OSArchitecture

Get-CimInstance Win32_VideoController |
    Format-List Name, DriverVersion, DriverDate, PNPDeviceID

quser.exe
if ($LASTEXITCODE -ne 0) { throw "session query failed: $LASTEXITCODE" }

$probe = cargo run --quiet -p display-probe 2>&1
if ($LASTEXITCODE -ne 0) {
    $probe
    throw "display-probe failed: $LASTEXITCODE"
}
$probe | Select-String 'ActivePaths:|SourceGdiDeviceName:|TargetFriendlyName:|TargetDevicePath:|TargetConnectorInstance:|OutputTechnology:|CurrentResolution:|CurrentRefreshRateHz:'

$hdr = (Read-Host 'MSI MAG342CQのHDR状態を OFF / ON / UNKNOWN で入力').Trim().ToUpperInvariant()
if ($hdr -notin @('OFF', 'ON', 'UNKNOWN')) { throw 'HDR状態の入力が不正です' }
[PSCustomObject]@{
    Session = $env:SESSIONNAME
    MsiHdr  = $hdr
} | Format-List
```

この取得ではWindows設定を変更しません。出力から一つのGate B承認文を作り、人間ownerの明示承認後にだけStage 2実装を開始します。actual mutation直前には、承認されたmonitor以外を人間operatorが無効化または切断し、`ActivePaths: 1`をread-onlyで再確認します。追加D08 batch、fixture再生成、別G1A資料は作りません。

## Windows以外で実行した場合

Windows以外ではWindows APIをcompile対象にせず、次のメッセージを表示して終了します。

```text
display-probe is Windows-only. Build and run it on Windows 10 or Windows 11.
```
