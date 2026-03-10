# セットアップ・ビルド・MT5 連携

signalrs はライブラリパッケージのため、単体では実行できない。ホストアプリケーション（Tauri 等）に組み込んで使用する。

マウスクリック操作の関係上、**実行環境は Windows 11** が前提。
開発・ビルドは **Windows 11 上の WSL (Ubuntu)** から Windows 向けクロスコンパイルで行う。
詳細なクロスコンパイル手順は [build-guide.md](./build-guide.md) を参照。

## 前提条件

- **実行環境:** Windows 11
- **開発環境:** Windows 11 WSL (Ubuntu 24.04) または Windows ネイティブ
- **Rust:** 1.70+（Edition 2021）
- **MT5/MT4:** トレードシグナル送信元

## ビルド

### パッケージとしてビルド

```bash
cargo build --release
```

`target/release/signalrs.dll`（cdylib）と `target/release/libsignalrs.rlib` が生成される。

### テスト実行

```bash
cargo test
```

主要テスト:
- `test_ticker_stats` - Ticker の生成と mid() 計算
- `test_ticker_stats_diff` - CSV データによるバックテスト
- `test_random_xy` - マウス座標のランダム生成
- `test_wrapped_data_update` - 状態更新と決済価格追跡
- Origin フラグ別テスト (flag 0-6)

---

## ホストアプリケーションへの組み込み

signalrs はライブラリクレートのため、ホストアプリケーション側で依存関係として追加する。以下は Tauri アプリに組み込む場合の例。

### ホスト側の Cargo.toml

```toml
[dependencies]
signalrs = { path = "../signalrs" }
```

### ホスト側の main.rs 例（Tauri v2 の場合）

```rust
use signalrs::{invoke, middleware, order_type};

fn main() {
    let gui_setting = /* Arc<RwLock<Data>> の初期化 */;

    tauri::Builder::default()
        .manage(gui_setting)
        .invoke_handler(tauri::generate_handler![
            invoke::gui::run,
            invoke::gui::get,
            invoke::gui::set,
            invoke::gui::confirm,
        ])
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // 終了時の設定保存処理
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## MT5/MT4 側の設定

### EA/スクリプトの実装要件

MT5/MT4 側で以下の JSON を TCP で送信する EA またはスクリプトを実装する。

#### 送信 JSON フォーマット

```json
{
  "symbol": "USDJPY",
  "bid": "150.123",
  "ask": "150.125",
  "flag": 0,
  "side": 1,
  "server_at": "2024-01-01T12:00:00+09:00"
}
```

#### MQL5 スクリプト例（概念）

```mql5
// TCP ソケットで signalrs に接続
int socket = SocketCreate();
SocketConnect(socket, "127.0.0.1", 8080, 1000);

// Tick ごとに送信
void OnTick() {
    string json = StringFormat(
        "{\"symbol\":\"%s\",\"bid\":\"%s\",\"ask\":\"%s\",\"flag\":%d,\"side\":%d,\"server_at\":\"%s\"}",
        Symbol(),
        DoubleToString(SymbolInfoDouble(Symbol(), SYMBOL_BID), Digits()),
        DoubleToString(SymbolInfoDouble(Symbol(), SYMBOL_ASK), Digits()),
        0,    // flag: 0=none, 1-6=custom
        0,    // side: 0=none, 1=buy, 2=sell
        TimeToString(TimeCurrent(), TIME_DATE|TIME_SECONDS)
    );

    char data[];
    StringToCharArray(json + "\n", data);
    SocketSend(socket, data, ArraySize(data));
}
```

**注意:** 各 JSON メッセージは改行 (`\n`) で区切る。signalrs の TCP パーサーは改行区切りで JSON を読み取る。

---

## マウス座標の設定手順

1. 対象のブラウザ/取引プラットフォームを開く
2. signalrs アプリの設定画面で座標を入力:
   - **Buy ボタン:** 左上 (start_x, start_y) ～ 右下 (end_x, end_y)
   - **Sell ボタン:** 同上
   - **Exit/Close ボタン:** 同上
3. `confirm` コマンドでマウスが正しい位置に移動するか確認
4. クリック回数 `n` を設定（確認ダイアログがある場合は 2 回以上に設定）

### 座標確認のコツ

- Windows の「設定 > ディスプレイ > 拡大縮小」が 100% であることを確認
- マルチモニター環境では、対象モニターの座標体系に注意
- ブラウザのズームレベルを固定しておく

---

## 運用上の注意

### パフォーマンス

- speed 設定が `0`（Ultra）の場合、CPU 使用率が高くなる可能性がある
- 通常の運用では `1`（High: 100ms）で十分

### セキュリティ

- TCP 通信は暗号化されていないため、ローカルホスト (`127.0.0.1`) での使用を推奨
- ファイアウォールで TCP ポートへの外部アクセスをブロックする

### 安定性

- `interval_random = true` でクリック間隔をランダム化し、パターン検知を回避
- マウス座標の範囲を適度に広く設定し、同一ピクセルへの連続クリックを避ける
- ブラウザのウィンドウ位置・サイズを固定しておく（座標ずれ防止）

### ログ

```bash
RUST_LOG=info cargo run
```

`env_logger` により、TCP 接続エラー・注文実行・価格データ等がログ出力される。
