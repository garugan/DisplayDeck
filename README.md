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

## Step 5: GDI ↔ CCD exact cross-mapの確認（実装済み・Windows実機検証待ち）

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

安定したWindows実機環境でactive pathがすべて一意に`Exact`となり、GDIとCCDのsource/target identityが期待する物理displayへ対応することを標準出力と目視で確認できたらStep 5完了です。完全一致または整合性検査が成立しない環境では推測で補完せず、そのsupport cellを`Unmapped`、`Ambiguous`、または`Inconsistent`として記録します。

### Windows以外で実行した場合

Windows以外ではWindows APIをcompile対象にせず、次のメッセージを表示して終了します。

```text
display-probe is Windows-only. Build and run it on Windows 10 or Windows 11.
```
