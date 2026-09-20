# DisplayDeck

Windows向けのTauri 2デスクトップアプリです。DisplayDeck v0.1.0は、ディスプレイ構成・現在の表示モードを安全に取得し、診断情報として確認・出力するread-onlyアプリです。

## v0.1.0の範囲

- Display列挙、現在モード取得、read-only UI
- diagnostic JSON出力
- fail-closed（取得・検証できない情報を推測で補わず、不明・エラーとして扱う）
- Installer、uninstall

Display設定変更、Apply / Restore、WAL、crash recovery、watchdog、mutation safety、Gate Bはv0.1.0に含めず、v0.2以降の対象とします。これらはv0.1.0の完成条件・リリース前提ではありません。

## リリース進捗

現行のチェックリストは[実装計画のv0.1.0 Release / v0.2 Mutation](docs/implementation-plan.md#v010-release)で管理します。v0.1.0はRC3とInstaller SHA256を固定済みです。最終Smoke Testも10項目PASSです。releaseコピーとのSHA256一致と証跡更新も完了しました。2026-09-21にGate C承認済みです。Windows保存済みmetadataの最終同期も完了し、v0.1.0 Release作業は終了しました。v0.2 Mutationは別トラックです。

## v0.1.0 Release承認済み

```text
Artifact: DisplayDeck_0.1.0_x64-setup.exe
Size: 2160643 bytes
SHA256: 25DEFCA4CC6DA01F350CC82E1302FF0CE1783BBCEE2A88B77EE2734E0C637EFD
Source: e598cc4d08a37ec6815a80a2f864f562c59ed8e6
Tag: v0.1.0
```

承認範囲は今回検証したWindows環境限定のread-only版です。public distributionは含みません。

### リリース作業完了

Windows保存済みmanifestも`RELEASED` / `APPROVED`へ同期済みです。InstallerのSHA256は変更されていません。追加のWindows操作・build・試験は不要です。

保存先: `D:\project\displaydeck\release\v0.1.0`。製品sourceに付けた`v0.1.0`tagはpush済みです。

承認・完了記録: [v0.1.0 RC3 Gate C承認](docs/windows-validation-history.md#v010-rc3-gate-c承認2026-09-21)。

## 過去のrelease 01検証・承認記録

- release 01はGate C承認済み、read-only MVP完成（現行チェックリストの完了を意味しない）
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
- operator操作によるlocal diagnostic JSON export
- current-user NSIS install / launch / uninstall

解像度、refresh rate、配置、registry、Windows display profileは変更しません。

既存のfake simulationとsafety coreは将来機能の開発資産であり、v0.1.0の提供機能には数えません。上記の範囲定義は、検証済みrelease 01のartifact内容を変更したという意味ではありません。

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
