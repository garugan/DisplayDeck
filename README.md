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

通常の3台・extend環境に対するStep 7は完了です。この結果はcandidate policyを緩和する根拠ではなく、このWindows実機観測環境では現時点でmutation候補を発行できないという、将来のG1A review入力の一部です。正式なsupport cell identity、support fingerprint、machine manifest、evidence IDはまだ発行していません。3本のdistinct-source active pathによる複数display構成はStep 8でfail-closed evidenceとして扱い、59.94/60、hotplug、RDP、virtual、clone / shared-sourceは別cellまたは未観測gapとして分類します。

## Step 8: read-only support assessment（CLI milestone完了・Phase 1A closure未完）

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

設計資料のPhase 1A first-cell候補surfaceは`QDC_ONLY_ACTIVE_PATHS | QDC_VIRTUAL_MODE_AWARE`と別のallocation contractを要求しています。Step 8では、このflag変更とvirtual-aware index decodeを暗黙に追加せず、`ApprovedCcdSurfaceNotImplemented`として明示します。この差が残る限り、正式なPhase 1A closureを主張しません。[`QDC_VIRTUAL_MODE_AWARE`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)を使用する次のcapture surfaceは、別途明示承認とWindows再検証が必要です。

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

以上によりStep 8のWindows実機検証と実装は完了です。ただし、これは正式なPhase 1A / G1A closureではありません。exact support cell identity、OS build / KB、GPU / driver / monitor / connection manifest、session / privilege evidence、approved CCD surface、call trace / timebox、sanitized bounded evidence bundle、human reviewが不足しているため、`Phase1AClosure: NotClaimed`を維持します。

ここに記載したのはユーザーが確認した実機結果の要約であり、immutableなPhase 1A evidence recordではありません。元のvalidation log名、実行日時、Target Machine、Operator / Evidence Owner、sanitized evidence IDと保管場所は、正式なPhase 1A recordを作成するときに別途登録します。

## 初期リリースまでの実装roadmap

Step 1〜8のread-only display列挙、mode取得、CCD取得、GDI ↔ CCD exact cross-map、`CurrentObservation`統合、candidate model、fail-closed support assessmentはWindows実機検証済みです。Step 8後の現HEADでもStep 5 / 6 / 7の回帰がなく、実行前後でmonitor設定が変化しないことを確認しました。通常環境の成功だけでPhase 1A / G1A closureを主張せず、観測済みnegative evidenceと未観測または初期対象外のgapを分けて、将来のG1A result reviewへ渡します。

### Read-only CLIの完成

| Step | 対応phase | 内容 | 完了条件 |
| --- | --- | --- | --- |
| 6 | Phase 1A | `CurrentObservation`統合 | exact cross-mapごとにGDI現在値とCCD source/targetを統合し、rotation適用後の解像度とinteger/rational refreshの関係を`Exact`、`Distinct`、`Mismatch`、`Unavailable`で説明できる。`60`と`60000/1001`を近似一致にしない。 |
| 7 | Phase 1A | candidate model構築 | 利用可能modeをwidth/height/Hzだけでなく、承認された完全な`DEVMODE` field tupleで保持する。duplicate、current-not-listed、不完全field、orientation/color policy差を推測せず分類する。 |
| 8 | Phase 1A / G1A | read-only spike完了と結果review | 59.94/60、縦置き、candidate多数、current-not-listed、hotplug、RDP、virtual/multi-pathをfail-closed分類する。追加APIによるpreferred、persisted、HDR/DRR等の観測は、それぞれ個別承認されたread-only rowだけで行う。 |

### 設定変更前の安全基盤

| Step | 対応phase | 内容 | 完了条件 |
| --- | --- | --- | --- |
| 9 | Phase 2A | coordination/storage/process proof | displayを変更せず、watchdog、one-shot fake worker、dual-slot WAL/decision journal、machine/display/user lock、deadline、process identity、lease/fencing、crash consistencyを検証する。 |
| 10 | Phase 1B | controlled mutation spike | Step 9のevidence reviewと別承認後、専用実機・外部映像確認・blind recovery手順の下で、一時的なmode変更、fresh readback、C0 exact restore、P0非変更を限定検証する。 |
| 11 | Phase 2B | controlled watchdog recovery | real one-shot workerとwatchdogを統合し、timeout、manual Revert、親process終了、worker hang、session変更、watchdog takeoverで安全な復元またはfail-closedを実証する。 |

### DisplayDeck製品実装

| Step | 対応phase | 内容 | 完了条件 |
| --- | --- | --- | --- |
| 12 | Phase 3 | Tauri 2基盤 | React、TypeScript、Vite、Rust、単一local window、typed command、Capabilities/Permissions、CSP、navigation制限を構築する。shell/fs/http/process等の不要なfrontend権限を与えない。 |
| 13 | Phase 4 | Mock UI | current/planned display cards、mode選択、Apply/Reset、2-stage presentation、15秒確認、Revert、startup recovery/error画面をmock stateで完成させる。 |
| 14 | Phase 5 | Rust read-only統合 | Step 1〜8のquery/domain mappingをTauri commandへ移植し、raw device pathをfrontendへ公開せず、event lossをauthoritative status commandで再同期できるようにする。 |
| 15 | Phase 6 | safety統合 | packaged watchdog、one-shot worker、journal、begin/confirm/restore/status、presentation ACK、startup recoveryを製品構成へ接続する。安全基盤がreadyでなければmutationを開始しない。 |
| 16 | Phase 7 | Windows実機総合test | 承認されたWindows 10/11、GPU、driver、physical display cellで通常操作、59.94/60、縦置き、hotplug、process crash、rollback、session変化、accessibilityを検証する。 |
| 17 | Phase 8 | installer・初期release判定 | NSIS候補、WebView2、署名、update/repair/uninstall、watchdog同梱・process behavior、recovery evidenceをpackaged環境で確認し、初期リリースのGo/No-Goを決定する。 |

### Step 9の現在位置

Step 9は便宜上`FREEZE_CANDIDATE_AUTHORING`と呼んだ文書作業段階です。これは正式なPhase名、gate結果、実行許可ではありません。`DecisionJournalV1`、`MachineActorRecordV1`、`MachineActorProvisionRecordV1`、one-shot worker oracleのcandidate specification labelを[`docs/implementation-plan.md`](docs/implementation-plan.md)へ記録しました。

**履歴（2026-08-13）**: ユーザーはD01〜D08のrecommended candidateを設計方針として承認し、当時は文書上のfreeze candidate作成だけを許可しました。full-byte fixture、expected SHA-256、artifact hash、Windows file/DACL実証、Reviewer / Approver、Phase 2A専用authorizationは未存在であり、Phase 2A実装、fixture作成・実行、display mutationは未許可でした。[`docs/architecture.md`](docs/architecture.md) 19.7〜19.8は、この承認を固定長slot、`OwnerWalLinkStateV1`、canonical JSON/SID、first-create checkpoint、vector manifestへ反映したcandidate specificationです。

**CANDIDATE-03 review history（不合格）**: `CANDIDATE-03`は77 vectorのfull bytes、individual SHA-256、semantic manifest、artifact index、aggregate hashについて自己整合は確認できました。しかし独立reviewは、D02のsubject/observation canonical-source coverage、D03/SID binding coverage、MAP resume/cleanup semantic scenario、DJ/MAR negative/cross-link coverageに不足を確認しました。したがって`CANDIDATE-03`はfreeze不可です。hash整合はindependent review pass、artifact approval、`FROZEN`のいずれも意味しません。同じcandidate IDを修正して再利用しません。

**現況（CANDIDATE-04 artifact generated / static review clean）**: active profileは`DD-FR-002-WIRE-PROFILE-V1-CANDIDATE-04`です。CANDIDATE-04はD02 canonical subject/observed sources、D03/SID boundary and cross-link coverage、DecisionJournal/MachineActor negative matrices、MAP resume scenario、D04 tuple/readiness/evidence matrixを拡張し、worker oracleの`WOSV1-N-001..009`とstatic `BootIdV1`を維持します。exact vector ID、bytes、hash、catalogは生成器が出力したcandidate-04 manifest/indexを候補正本とします。statusは`FULL_BYTES_GENERATED / SHA256_COMPUTED / FULL_INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING`です。

**CANDIDATE-04 full independent static review**: generatorは590 vectorを再現可能に生成し、self-verifyと別temporary directoryへのbyte-for-byte再生成に成功しました。独立reviewは590件すべてのID、byte length、fixture SHA-256、index link、unindexed file、`SHA256SUMS`、semantic manifest/index/aggregate hashを再計算して`CLEAN`と判定しました。D04はstructural positive 205件（display/recovery 60、maintenance/update/repair 144、initial provision 1）、negative 28件、D04 evidence 162件、readiness companion 1件で、K06 positiveは0件です。現package hashはsemantic manifest=`4211b04dc0f456f3ca9d8e3f527bb27f31bfa96dc7e3e26ab62ca69534375210`、artifact index=`de757449851e60280e949f5000072fc4edde655a1053ef587a46b30a2e246b9a`、aggregate fixture set=`1181f1ff3850877bf0bb5afe19a09f4812571f4b5da0d11b12bc2d0a6ab75179`です。2026-08-13に[`docs/architecture.md`](docs/architecture.md) の`DD-FR-002-D04-C04-RESOLUTION-PACKAGE-01`は一括承認され、許可範囲内のD04 fixture再生成とCandidate 04全体の独立reviewが完了しました。これは最終artifact approvalまたは`FROZEN`ではなく、Phase 2A実装とdisplay mutationも未許可です。

#### Step 9 Windows検証コマンド

Windows PowerShellでrepository rootへ移動し、次を上から順に実行します。Pythonは標準ライブラリだけを使用し、display設定やruntime fileを変更しません。全出力は`step9-windows-validation.txt`へ保存されます。

```powershell
Start-Transcript -Path step9-windows-validation.txt -Force

# .gitattributes追加前にcheckoutしたCRLF版sidecarを、indexの正本bytesで強制的に戻す。
git checkout-index -f -- fixtures/dd-fr-002-wire-v1-candidate-04/SHA256SUMS
git checkout-index -f -- fixtures/dd-fr-002-wire-v1-candidate-04/artifact-index.json
git checkout-index -f -- fixtures/dd-fr-002-wire-v1-candidate-04/freeze-candidate-metadata.json
git checkout-index -f -- fixtures/dd-fr-002-wire-v1-candidate-04/semantic-manifest.jsonl

py -3 -B tools/dd-fr-002-freeze/dd_fr_002_freeze.py verify
py -3 -B tools/displaydeck-evidence/validate_d07_no_go.py tools/displaydeck-evidence/d07-no-go-predicate.template.json
py -3 -B tools/displaydeck-evidence/validate_d07_no_go.py --self-test
py -3 -B tools/displaydeck-evidence/validate_d08_readonly_capture.py tools/displaydeck-evidence/d08-readonly-capture.template.json
py -3 -B tools/displaydeck-evidence/validate_d08_readonly_capture.py --self-test
py -3 -B tools/displaydeck-evidence/validate_g1a_bundle.py tools/displaydeck-evidence/g1a-bundle-manifest.template.json --artifact-root tools/displaydeck-evidence
py -3 -B tools/displaydeck-evidence/validate_g1a_bundle.py --self-test
git diff --check

Stop-Transcript
```

期待結果は`verification=pass vectors=590 side-effects=0`、各templateの`valid`、各`self-test: pass`で、`git diff --check`は無出力です。途中で1件でも失敗した場合はそこで検証失敗とします。この手順はCandidate 04、D07 No-Go、D08 static known-answer、G1A templateを検証するもので、D08 runtime captureを実施したり`FROZEN`を承認したりするものではありません。

#### D08 Windows read-only capture（別実行）

D08の実機sampleは、Windows PowerShell 5.1で次を実行します。7個のraw観測だけを`%TEMP%`の新規JSONへ保存し、既存fileは上書きしません。display設定は変更せず、acceptance thresholdはすべて`UNSET`のままです。

```powershell
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$capture = Join-Path $env:TEMP "displaydeck-d08-$stamp.json"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\displaydeck-evidence\capture_d08_readonly.ps1 -OutputPath $capture
if ($LASTEXITCODE -ne 0) { throw "D08 capture failed: $LASTEXITCODE" }
py -3 -B tools\displaydeck-evidence\validate_d08_readonly_capture.py $capture
if ($LASTEXITCODE -ne 0) { throw "D08 validation failed: $LASTEXITCODE" }
Get-Content -Raw $capture
```

期待結果は`captured: ...`、`valid: ...`、`valid static vector: ...`です。JSONは`captureStatus: CAPTURED`、`probeAuthorization: READ_ONLY_AUTHORIZED`、`result: ACCEPTANCE_NOT_AUTHORIZED`または明示的なrejectになり、acceptance resultは生成しません。raw captureはEvidence Owner、redaction、retention、bundle locationを承認するまでGitへ追加しません。

2026-08-21にWindows 10 `10.0.19045`の最初のsampleがvalidatorを通過しました。投稿されたraw値から導出した非識別summaryは、tick sample span=`109 ms`、UTC span=`98.326 ms`、2つのpredicted bootの差=`10.674 ms`、WMI boot時刻とpredicted bootの絶対差=`約40.019..40.030 s`です。これは`FIRST_WINDOWS_CAPTURE_REPORTED_VALID`であり、raw artifact hash、formal bundle、複数boot/resume sample、tolerance approvalはまだありません。次は[`D08 Windows read-only capture procedure`](tools/displaydeck-evidence/d08-windows-readonly-capture-procedure.md)の同一active session 5回batchを採取します。

今回の限定authorizationは、full-byte fixture、expected SHA-256、semantic manifest、artifact index、aggregate hashの生成・検証、D07 controlled filesystem/DACL evidence、D08 read-only Windows evidence、formal G1A evidence bundleの作成だけです。Phase 2A product/runtime code、Tauri/watchdog/worker統合、runtime serializer/WAL file、fault harness、display mutationは引き続き未許可です。D07は`DIRECTORY_ANCHOR_UNPROVEN / NO_GO_RECORDED`、D08は`READ_ONLY_AUTHORIZED / FIRST_WINDOWS_CAPTURE_REPORTED_VALID / TOLERANCE_EVIDENCE_PENDING`、G1Aはtemplate/validatorのみでformal result evidenceはpendingです。

| Decision | 人間が決める内容 | 現在のrecommended candidate | Status |
| --- | --- | --- | --- |
| `DD-FR-002-D01` | owner WAL state / result / recovery classificationの分離 | exact WAL frame stateだけをlinkし、valid writerの`FAILED_CLOSED`とunreadable classificationを分離 | `POLICY_APPROVED / SPEC_CANDIDATE / BYTE_ARTIFACT_GENERATED / INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING` |
| `DD-FR-002-D02` | critical evidenceのlossless binding | bounded detail codeとpreserved-evidence digestをcanonical fieldへ追加 | `POLICY_APPROVED / SPEC_CANDIDATE / BYTE_ARTIFACT_GENERATED / INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING` |
| `DD-FR-002-D03` | MachineActor JSON / SID表現 | scalarは固定幅string、SIDはfixed-capacity typed object、optional groupは全出現または全省略 | `POLICY_APPROVED / SPEC_CANDIDATE / BYTE_ARTIFACT_GENERATED / INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING` |
| `DD-FR-002-D04` | completion resultとstate × actor binding | bounded `OperationResultV1`とoperation-kind tagged completionを使い、active / maintenance / terminal actorの照合先をstate別に固定 | `POLICY_APPROVED / SPEC_CANDIDATE / BYTE_ARTIFACT_GENERATED / INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING` |
| `DD-FR-002-D05` | machine recordのruntime writer | SYSTEM creator/maintenance writerとinstaller-designated single runtime owner SIDに限定 | `POLICY_APPROVED / SPEC_CANDIDATE / BYTE_ARTIFACT_GENERATED / INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING` |
| `DD-FR-002-D06` | fresh MachineActor provision | separate installer provision recordでcreate前intentとactual file-ID checkpointをdurable化し、最初のvalid maintenance intent / activeを経てowner-bound ordinary cleanへ進む | `POLICY_APPROVED / SPEC_CANDIDATE / BYTE_ARTIFACT_GENERATED / INDEPENDENT_STATIC_REVIEW_CLEAN / HUMAN_FREEZE_APPROVAL_PENDING` |
| `DD-FR-002-D07` | directory anchor / reparse proof | documented handle/APIでrace-resistantに証明できたcellだけadmit。現時点は未証明のためNo-Go | `POLICY_APPROVED / SPEC_CANDIDATE / DIRECTORY_ANCHOR_UNPROVEN / NO_GO_RECORDED / HUMAN_FREEZE_APPROVAL_PENDING` |
| `DD-FR-002-D08` | boot identity | stable WMI boot UTC/version/buildだけをhashし、tick/UTC cross-checkは別acceptance evidenceとして実機でtoleranceをfreeze | `POLICY_APPROVED / SPEC_CANDIDATE / READ_ONLY_AUTHORIZED / FIRST_WINDOWS_CAPTURE_REPORTED_VALID / TOLERANCE_EVIDENCE_PENDING / HUMAN_FREEZE_APPROVAL_PENDING` |

D01〜D08の方針承認と今回の統合freeze-evidence authorizationは、full-byte artifact生成を許可しますが、`FROZEN`、artifact approval、Phase 2A authorizationではありません。CANDIDATE-04のfull-byte生成と独立static reviewは完了しましたが、D07/D08/G1A evidence、Reviewer/Approver、immutable approval referenceが揃うまでcode値やDACL候補の実装正本にしません。D05によりV1のmutation writerはinstaller-bound single runtime ownerだけで、別ownerへの変更はelevated maintenance/rebindを必要とします。

Phase 2Aは、formal G1A result reviewとDD-FR-002 freezeに加え、exact Windows / CPU / environment cell、filesystem / volume / security-product matrix、named-object / file / DACL exact call allowlist、fault / crash / power-loss injection allowlist、evidence fields / redaction / retention / location、Operator / Evidence Owner / Reviewer / execution date、Approverとimmutable approval IDを含むPhase 2A専用recordが全て埋まり承認されるまで`NOT EXECUTABLE`です。

Step 8の完了はPhase 2Aの自動承認ではありません。今回の承認はfreeze-evidence artifact生成に限定されます。Phase 2A product/runtime code、file operation、serializer、fault harnessは、正式なPhase 1A closure / G1A result review、DD-FR-002 full freeze、Phase 2A専用record全項目と専用authorizationが揃った後にだけ開始でき、同Phaseでもdisplay mutationは禁止です。

### Gateと実装完了の定義

- Step 9のPhase 2Aとそのevidence reviewが完了するまで、display設定変更APIを実装・実行しない。
- Step 10以降のmutation、watchdog recovery、product integration、installerは、それぞれ前提phaseの完了と別のhuman authorizationを必要とする。
- Step 1〜8の承認や成功を、後続Phase 2Aまたはmutation phaseの承認として扱わない。
- 初期リリースの実装完了はStep 17のpackaged検証とrelease判定までとする。
- scale変更は初期リリースに含めず、別のPhase 9技術スパイクとして扱う。

詳細なphase依存関係、安全要件、各gateは[`docs/implementation-plan.md`](docs/implementation-plan.md)を正本とします。

## Windows以外で実行した場合

Windows以外ではWindows APIをcompile対象にせず、次のメッセージを表示して終了します。

```text
display-probe is Windows-only. Build and run it on Windows 10 or Windows 11.
```
