# アーキテクチャ

## 概要

signalrs は **ライブラリパッケージ（クレート）** であり、単体ではアプリケーションとして動作しない。Tauri 等のデスクトップアプリプロジェクトから `Cargo.toml` の依存関係として取り込んで使用する。GUI やエントリーポイント（`main.rs`）は含まれていない。

ホストアプリケーションに組み込まれた際の全体構成は3層構造となる。

```
┌─────────────────────────────────────────────────────┐
│  MT5/MT4 (外部)                                      │
│  ├─ EA/スクリプトが価格データを JSON で TCP 送信       │
│  └─ ポート: 設定可能 (デフォルト 8080)                 │
└──────────────────┬──────────────────────────────────┘
                   │ TCP (JSON)
┌──────────────────▼──────────────────────────────────┐
│  signalrs (Rust ライブラリパッケージ)                  │
│                                                      │
│  ┌──────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ invoke/  │  │ middleware/  │  │ order_type/   │  │
│  │ GUI状態  │  │ TCP/Mouse/  │  │ 注文戦略      │  │
│  │ Tauri    │  │ Ticker/File │  │ ルーティング  │  │
│  │ コマンド │  │ Utils       │  │               │  │
│  └──────────┘  └──────────────┘  └───────────────┘  │
└──────────────────┬──────────────────────────────────┘
                   │ mouse-rs (マウス操作)
┌──────────────────▼──────────────────────────────────┐
│  ブラウザ / 取引プラットフォーム UI (Windows)          │
│  ├─ Buy ボタン座標にクリック → 買い注文               │
│  ├─ Sell ボタン座標にクリック → 売り注文              │
│  └─ Exit ボタン座標にクリック → 決済                  │
└─────────────────────────────────────────────────────┘
```

## クレートタイプ

```toml
[lib]
name = "signalrs"
crate-type = ["rlib", "cdylib"]
```

- **rlib**: 他の Rust プロジェクト（Tauri アプリ本体）からライブラリとして利用
- **cdylib**: ネイティブ動的ライブラリとしてもビルド可能

signalrs は `main.rs` を持たないライブラリクレートである。利用側のアプリケーション（例: Tauri プロジェクト）が `Cargo.toml` で依存関係として追加し、エクスポートされたモジュール・関数を呼び出す形で使用する。

```toml
# ホスト側 Cargo.toml の例
[dependencies]
signalrs = { path = "../signalrs" }
# または crates.io から
# signalrs = "0.2.0"
```

## スレッドモデル

```
ホストアプリのメインスレッド
  │
  ├─ TCP受信スレッド (middleware/tcp.rs)
  │   └─ BufReader + lines() で JSON をデシリアライズ
  │
  ├─ OrderDispatcher ワーカースレッド (order_type/choose.rs)
  │   └─ sync_channel(1) でバックプレッシャー制御
  │   └─ 注文タイプ別に price分析 → マウス操作
  │
  └─ ホスト側 UI（Tauri 2.x WebView）
      └─ エクスポートされたコマンド経由で状態取得・設定変更
```

### OrderDispatcher（注文ディスパッチャー）

従来は `choose::by()` が毎tick `thread::spawn` で新スレッドを生成していたが、`OrderDispatcher` に置き換え:

- `sync_channel(1)` による単一ワーカースレッド
- `try_send` で worker が busy なら tick をスキップ（バックプレッシャー）
- `is_running` / `is_processing` チェックは送信前に実施

## 共有状態管理

全てのスレッドは `WrappedData = Arc<RwLock<Data>>` を介して状態を共有する。

```rust
pub type WrappedData = Arc<RwLock<Data>>;
```

- **Arc**: 複数スレッド間でのオーナーシップ共有
- **RwLock**: 読み取りは並行、書き込みは排他制御

## ディレクトリ構成

```
src/
├── lib.rs                 # ライブラリルート（全モジュールをエクスポート）
├── consts.rs              # 定数定義（バッファサイズ、履歴上限等）
├── error.rs               # SignalError 列挙型（thiserror）
├── invoke/
│   ├── mod.rs             # モジュール宣言
│   └── gui.rs             # GUI状態・Tauriコマンド・OrderType/Speed enum
├── middleware/
│   ├── mod.rs             # モジュール宣言
│   ├── file.rs            # JSON設定ファイルの読み書き
│   ├── mouse.rs           # MouseController トレイト + mouse-rs 実装
│   ├── tcp.rs             # TCP クライアント/サーバー（BufReader + Result返却）
│   ├── ticker.rs          # 価格データ構造体・統計分析（VecDeque + 二分探索）
│   └── utils.rs           # ユーティリティ（sleep, SID[Windows限定], ログ）
└── order_type/
    ├── mod.rs             # モジュール宣言
    ├── choose.rs          # OrderDispatcher（sync_channel 単一ワーカー）
    ├── simple.rs          # シンプル戦略（エントリー＋決済）
    ├── entry.rs           # エントリーのみ（OrderType enum 使用）
    ├── exit.rs            # 決済のみ
    ├── origin.rs          # カスタムフラグ制御
    └── process.rs         # ロック/アンロック状態管理（poison 対策済み）
tests/
└── integration_test.rs    # 統合テスト（データライフサイクル、JSON契約、TCP）
```

## 主要依存クレート

| クレート | バージョン | 用途 |
|----------|-----------|------|
| tauri | 2.x | Tauri コマンドマクロ（ホストが Tauri の場合に利用） |
| serde / serde_json | 1.x | JSON シリアライズ |
| rust_decimal | 1.40.0 | 金融精度の10進数演算 |
| chrono | 0.4.44 | 日時計算・レイテンシ測定 |
| mouse-rs | 0.4.2 | クロスプラットフォームマウス操作 |
| rand | 0.8.5 | ランダム座標生成（検知回避） |
| thiserror | 2 | エラー型定義 |
| ta | 0.4.0 | テクニカル分析指標 |
| csv | 1.4.0 | CSVファイル読み込み（バックテスト用） |
| winapi | 0.3 | Windows API（SID取得、`cfg(windows)` 限定） |
| log / env_logger | 0.4.29 / 0.11.9 | ロギング |
