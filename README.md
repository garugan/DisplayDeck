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

- mode index: `0`から開始し、APIが失敗を返すまで1ずつ増加
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
    Mode 0: 1920x1080 @ 60 Hz
    Mode 1: 2560x1440 @ 60 Hz
    Mode 2: 2560x1440 @ 144 Hz
```

同じ解像度・refresh rateが複数回表示される場合でも、列挙recordを勝手に統合しません。表示していないbit depthやorientationなどが異なる可能性があるためです。

APIが1件もmodeを返さないadapterは`AvailableModes: 0`と表示します。fieldが取得できないrecordや`dmDisplayFrequency`が`0/1`のrecordは、Step 2と同じ規則で`unavailable`または`driver default`と表示します。

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

## Step 6: CurrentObservation統合（実装済み・Windows実機検証待ち）

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
- mappingは同じでもcurrent mode evidenceが変化した場合、Step 6だけを`StaleSnapshot`にする
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

実装とREADMEの確認手順は完了しています。Windows実機で上記を確認し、標準出力を保存するまではStep 6を完了扱いにしません。59.94/60、DRR、clone、hotplugなどの追加support cellは、通常の3台・extend環境とは分けてStep 8までにfail-closed分類を確認します。

## 初期リリースまでの実装roadmap

現在、Step 1〜5のread-only display列挙、mode取得、CCD取得、GDI ↔ CCD exact cross-mapまで完了しています。Step 6の`CurrentObservation`統合は実装済みで、Windows実機検証待ちです。検証完了後に着手するのはStep 7です。

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

### Gateと実装完了の定義

- Step 9のPhase 2Aとそのevidence reviewが完了するまで、display設定変更APIを実装・実行しない。
- Step 10以降のmutation、watchdog recovery、product integration、installerは、それぞれ前提phaseの完了と別のhuman authorizationを必要とする。
- Step 1〜6の承認や成功を、後続mutation phaseの承認として扱わない。
- 初期リリースの実装完了はStep 17のpackaged検証とrelease判定までとする。
- scale変更は初期リリースに含めず、別のPhase 9技術スパイクとして扱う。

詳細なphase依存関係、安全要件、各gateは[`docs/implementation-plan.md`](docs/implementation-plan.md)を正本とします。

## Windows以外で実行した場合

Windows以外ではWindows APIをcompile対象にせず、次のメッセージを表示して終了します。

```text
display-probe is Windows-only. Build and run it on Windows 10 or Windows 11.
```
