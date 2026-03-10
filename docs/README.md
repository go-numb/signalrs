# signalrs ドキュメント

signalrs は MT5/MT4 からのトレードシグナルを TCP 経由で受信し、Windows 上のブラウザに対してマウス操作（クリック）で自動受発注を行うための **Rust ライブラリパッケージ（クレート）** です。

本クレート単体はアプリケーションではなく、Tauri 等のデスクトップアプリプロジェクトに依存関係として取り込んで使用します。GUI（フロントエンド）や `main.rs` は含まれていません。

**signal-clicker** ([go-numb/signal-clicker](https://github.com/go-numb/signal-clicker)) が本パッケージを活用した Tauri v2 デスクトップアプリ（GUI）です。

## ドキュメント一覧

| ファイル | 内容 |
|---------|------|
| [architecture.md](./architecture.md) | システム全体のアーキテクチャと設計思想 |
| [signal-flow.md](./signal-flow.md) | MT5/MT4 からマウスクリックまでのデータフロー |
| [modules.md](./modules.md) | 各モジュールの詳細リファレンス |
| [configuration.md](./configuration.md) | 設定項目・パラメータの解説 |
| [order-types.md](./order-types.md) | 注文タイプ別の処理ロジック |
| [setup.md](./setup.md) | ビルド・セットアップ・MT5 連携手順 |
| [build-guide.md](./build-guide.md) | WSL/Linux からの Windows クロスコンパイル手順 |

## 基本情報

- **バージョン:** 0.2.0
- **言語:** Rust (Edition 2021)
- **種別:** ライブラリクレート（`rlib` + `cdylib`）
- **想定ホスト:** Tauri 2.x アプリから依存関係として利用
- **ライセンス:** MIT
- **対応OS:** Windows（マウス操作に WinAPI を使用）、Linux（ビルド・開発は可能）
