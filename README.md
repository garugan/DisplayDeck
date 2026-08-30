# DisplayDeck

Windows向けのTauri 2デスクトップアプリです。DisplayDeck 0.1.0は、ディスプレイ情報と利用可能なmode候補を表示するread-only MVPです。

## 現在の状態

- Gate C承認済み、read-only MVP完成
- Windows設定を変更するdisplay APIは実行しない
- `Apply`はdisabled、15秒transactionはfake simulationのみ
- actual D07は`DirectoryAnchorUnproven`でNo-Go
- Windows 11、他hardware cell、署名、auto-update、public distributionは未承認

Qualified artifact:

```text
DisplayDeck_0.1.0_x64-setup.exe
Length: 2160426
SHA256: 3307DB604C5C96B4E753D499ECB006E2209695006965F9BA7D65A1BF6F1EFD2F
Product source commit: e598cc4
```

検証済み範囲はWindows 10 Home `10.0.19045` x64、NVIDIA GeForce RTX 4070 driver `32.0.16.1088`、local consoleの記録済みexact cellだけです。

## 主な機能

- current display / mode / candidateまたは変更不能理由の表示
- read-only support assessment
- fake safety transaction
- operator操作によるlocal diagnostic JSON export
- current-user NSIS install / launch / uninstall

解像度、refresh rate、配置、registry、Windows display profileは変更しません。

## Build

Windows PowerShellで実行します。

```powershell
npm.cmd ci
cargo fmt --all -- --check
cargo test --workspace --all-targets
npm.cmd run build
npm.cmd run tauri build -- --no-bundle
```

NSIS installerを作る場合:

```powershell
npm.cmd run bundle:windows
```

artifact bytesまたは製品sourceを変更するとGate C release 01とは別candidateになります。

## 構成

- `src/`: React / TypeScript UI
- `src-tauri/`: Tauri application coreとWindows package設定
- `native/display-probe/`: read-only Windows display inventory
- `native/displaydeck-safety/`: fake watchdog / worker / WAL safety core

## 文書

- [実装計画と現在地](docs/implementation-plan.md)
- [要件](docs/requirements.md)
- [architecture](docs/architecture.md)
- [security](docs/security.md)
- [testing strategy](docs/testing-strategy.md)
- [Windows display research](docs/windows-display-research.md)
- [Windows検証履歴・手順書](docs/windows-validation-history.md)
