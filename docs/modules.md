# モジュール詳細リファレンス

## invoke/gui.rs（579行）

アプリケーションの中核。状態管理と Tauri コマンドを定義する。

### 主要構造体

#### `Data`
アプリケーション全状態を保持するルート構造体。

```rust
pub struct Data {
    pub version: String,
    pub description: String,
    pub host: String,
    pub status: Status,
    pub mouse_entry_buy: Mouse,
    pub mouse_entry_sell: Mouse,
    pub mouse_exit: Mouse,
    pub setting: Setting,
    pub order_type: Vec<Op>,
    pub speed: Vec<Op>,
}
```

#### `Status`
実行状態・注文履歴・メッセージを保持。

```rust
pub struct Status {
    pub is_running: bool,
    pub is_processing: bool,   // 注文処理中フラグ（排他制御）
    pub orders: Vec<Order>,    // 注文履歴（最新8件）
    pub message: String,       // 表示メッセージ
}
```

#### `Mouse`
マウスクリック座標の範囲とクリック回数。

```rust
pub struct Mouse {
    pub start_x: u32,   // クリック範囲の左上 X
    pub start_y: u32,   // クリック範囲の左上 Y
    pub end_x: u32,     // クリック範囲の右下 X
    pub end_y: u32,     // クリック範囲の右下 Y
    pub n: u8,          // クリック回数
}
```

#### `Setting`
ユーザー設定。

```rust
pub struct Setting {
    pub tcp: String,            // TCP ポート番号
    pub order_type: String,     // 注文タイプ (0/1/2/3/99)
    pub speed: String,          // 速度設定 (0/1/2/3)
    pub vol: String,            // ボラティリティ閾値
    pub interval: u32,          // エントリー→決済の待機秒数
    pub interval_random: bool,  // 待機時間のランダム化
}
```

#### `Order`
注文記録。

```rust
pub struct Order {
    pub side: String,
    pub entry: Decimal,
    pub exit: Decimal,
    pub entried_at: DateTime<Local>,
    pub exited_at: DateTime<Local>,
}
```

### Tauri コマンド

| コマンド | 引数 | 動作 |
|---------|------|------|
| `run(t: u8)` | 0=停止, 1=開始, 2=状態取得 | アプリの起動/停止/状態確認 |
| `get()` | なし | 全設定・状態の JSON を返す |
| `set(t: u8, v: Value)` | t=設定種別, v=JSON値 | 設定変更 |
| `confirm(t: u8, n: u8)` | t=マウス種別, n=回数 | マウス位置テスト |

`set` の種別:
- `1`: Setting 全体
- `2`: Exit クリック回数
- `3`: Entry Buy マウス座標
- `4`: Entry Sell マウス座標
- `5`: Exit マウス座標

---

## middleware/tcp.rs

ジェネリック TCP クライアント/サーバー。

### `TcpClient<T>`

```rust
pub struct TcpClient<T> {
    host: String,
    sender: Sender<T>,
    receiver: Receiver<T>,
}
```

| メソッド | 説明 |
|---------|------|
| `new(host)` | チャネル付きクライアント生成 |
| `connect()` | 外部サーバーへ接続（クライアントモード） |
| `received_server()` | 指定ポートでリッスン（サーバーモード） |
| `recv()` | チャネルから受信データを取得 |

受信データは改行区切りの JSON として読み取り、`serde_json::from_str::<T>()` でデシリアライズする。

---

## middleware/ticker.rs（288行）

価格データの構造体と統計分析。

### `Ticker`

```rust
pub struct Ticker {
    pub symbol: String,
    pub bid: Decimal,
    pub ask: Decimal,
    pub flag: Option<u8>,
    pub side: Option<u8>,
    pub server_at: DateTime<Local>,
    pub recived_at: DateTime<Local>,
    pub diff_micros: Option<i64>,
}
```

| メソッド | 説明 |
|---------|------|
| `mid()` | 中間値 `(bid + ask) / 2` |

### `TickerStats`

```rust
pub struct TickerStats {
    pub tickers: Vec<Ticker>,
}
```

| メソッド | 説明 |
|---------|------|
| `diff(micros)` | 指定マイクロ秒前との価格差を計算 |
| `zscore_last()` | 最新価格のZスコア（標準偏差ベース） |
| `shrink()` | 固定サイズにトリミング（メモリ管理） |
| `filter_micros(micros)` | 指定マイクロ秒前のティッカーを検索 |

---

## middleware/mouse.rs

`mouse-rs` クレートのラッパー。

| 関数 | 説明 |
|------|------|
| `random_xy(min, max)` | 範囲内ランダム座標を生成 |
| `move_to(x, y)` | カーソルを移動 |
| `click()` | 左クリック |
| `order(setting)` | 座標移動＋クリックの一連動作 |

---

## middleware/file.rs

JSON ファイルの永続化。

| 関数 | 説明 |
|------|------|
| `write<T>(filename, data)` | 構造体を JSON ファイルに保存 |
| `read<T>(filename)` | JSON ファイルから構造体を読み込み |
| `create_save_dir()` | 保存ディレクトリの作成 |

---

## middleware/utils.rs

ユーティリティ関数群。

| 関数 | 説明 |
|------|------|
| `sleep(sec, ms)` | スレッドスリープ |
| `ok(is_production, sid, jtc_s)` | ライセンス/有効期限検証 |
| `set_env_for_logger()` | ロガー初期化 |
| Windows SID 取得 | WinAPI でプロセストークンからSIDを抽出 |

---

## order_type/choose.rs

注文タイプのルーター。設定値に応じて処理を分岐する。

```rust
match order_type {
    0  => simple::process(),    // シンプル（エントリー＋決済）
    1  => entry::process(),     // 買いエントリーのみ
    2  => entry::process(),     // 売りエントリーのみ
    3  => exit::process(),      // 決済のみ
    99 => origin::process(),    // カスタムフラグ制御
}
```

---

## order_type/process.rs（24行）

処理状態のロック/アンロック。

| 関数 | 説明 |
|------|------|
| `lock()` | `is_processing = true`（二重注文防止） |
| `unlock()` | `is_processing = false` + 注文を履歴に記録 |

---

## テスト

各モジュールにユニットテストが含まれる。

| モジュール | テスト内容 |
|-----------|-----------|
| middleware/ticker.rs | Ticker生成、中間値計算、CSVバックテスト、Zスコア |
| middleware/mouse.rs | ランダム座標生成の範囲検証 |
| invoke/gui.rs | ランダムXY、価格更新、決済価格追跡 |
| order_type/origin.rs | 全フラグタイプ (0-6) の処理分岐テスト |
