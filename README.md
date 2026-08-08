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

## Step 3: 利用可能な解像度・refresh rate一覧の確認

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

利用可能な解像度・整数refresh rate一覧をWindows実機で確認できたら、Step 3は完了です。CCDによるtopologyとrational refreshの取得はStep 4で実装します。

### Windows以外で実行した場合

Windows以外ではWindows APIをcompile対象にせず、次のメッセージを表示して終了します。

```text
display-probe is Windows-only. Build and run it on Windows 10 or Windows 11.
```
