# モジュール詳細リファレンス

## consts.rs

アプリケーション定数。

```rust
pub const DEFAULT_TICKER_BUFFER_SIZE: usize = 144;
pub const DEFAULT_ORDER_HISTORY_LIMIT: usize = 8;
pub const DEFAULT_SAVE_PATH: &str = "./.save/setting.json";
```

---

## error.rs

`thiserror` ベースのエラー型。

```rust
pub enum SignalError {
    TcpBind { addr: String, source: std::io::Error },
    MouseOp(String),
    LockPoisoned(String),
    ChannelClosed,
    Parse(String),
    Io(std::io::Error),
}
```

---

## invoke/gui.rs

アプリケーションの中核。状態管理・Tauri コマンド・型定義。

### 列挙型

#### `OrderType`

注文タイプ。`serde(rename)` により JSON では `"0"`, `"1"` 等の文字列として送受信される（フロントエンド互換）。

```rust
pub enum OrderType {
    #[serde(rename = "0")]  Simple,     // 方向不問
    #[serde(rename = "1")]  BuyEntry,   // 買いエントリーのみ
    #[serde(rename = "2")]  SellEntry,  // 売りエントリーのみ
    #[serde(rename = "3")]  ExitOnly,   // 決済注文のみ
    #[serde(rename = "99")] Custom,     // 独自フラグ
}
```

#### `Speed`

処理速度。

```rust
pub enum Speed {
    #[serde(rename = "0")] UltraFast,  // 1ms
    #[serde(rename = "1")] Fast,       // 100ms
    #[serde(rename = "2")] Medium,     // 1,000ms
    #[serde(rename = "3")] Slow,       // 3,000ms
}
```

### 主要構造体

#### `Setting`

```rust
pub struct Setting {
    pub tcp: String,              // TCP ポート番号
    pub order_type: OrderType,    // 注文タイプ（enum）
    pub speed: Speed,             // 速度設定（enum）
    pub vol: String,              // ボラティリティ閾値
    pub interval: u32,            // エントリー→決済の待機秒数
    pub interval_random: bool,    // 待機時間のランダム化
}
```

#### `Status`

```rust
pub struct Status {
    pub is_recived: bool,
    pub is_running: bool,
    pub is_processing: bool,
    pub message: String,
    pub ltp: Decimal,
    pub orders: VecDeque<Order>,  // 注文履歴（VecDeque で O(1) shrink）
    pub updated_at: DateTime<Utc>,
}
```

#### `Order`

```rust
pub struct Order {
    pub side: String,
    pub entry: Decimal,
    pub exit: Decimal,
    pub entried_at: DateTime<Utc>,
    pub exited_at: DateTime<Utc>,
}
```

#### `Mouse`

```rust
pub struct Mouse {
    pub start_x: u32,   // クリック範囲の左上 X
    pub start_y: u32,   // クリック範囲の左上 Y
    pub end_x: u32,     // クリック範囲の右下 X
    pub end_y: u32,     // クリック範囲の右下 Y
    pub n: u8,          // クリック回数
}
```

#### `ParsedSetting`

`Setting` の型付きアクセス用。文字列パースを1回で済ませる。

```rust
pub struct ParsedSetting {
    pub order_type: OrderType,
    pub vol: Decimal,
    pub interval: u32,
    pub interval_random: bool,
}
```

### Tauri コマンド

| コマンド | 引数 | 動作 |
|---------|------|------|
| `run(t: u8)` | 0=停止, 1=開始, 2=状態取得 | アプリの起動/停止/状態確認 |
| `get()` | なし | 全設定・状態の JSON を返す |
| `set(t: u8, v: Value)` | t=設定種別, v=JSON値 | 設定変更 |
| `confirm(t: u8, n: u8)` | t=マウス種別, n=回数 | マウス位置テスト |

---

## middleware/tcp.rs

ジェネリック TCP クライアント/サーバー。`BufReader` + `lines()` でメッセージフレーミング。

```rust
pub fn received_server(&self) -> Result<(), SignalError>
```

- bind 失敗時は `SignalError::TcpBind` を返す（パニックしない）
- `tx.send()` 失敗時はログ出力のみ（receiver 切断 = アプリ終了中）

---

## middleware/ticker.rs

価格データの構造体と統計分析。`VecDeque` ベースのスライディングウィンドウ。

### `TickerStats`

```rust
pub struct TickerStats {
    data: VecDeque<Ticker>,  // O(1) の pop_front による shrink
}
```

| メソッド | 説明 |
|---------|------|
| `diff(micros)` | 指定マイクロ秒前との価格差を計算 |
| `filter_micros(micros)` | `partition_point` による O(log n) 二分探索 |
| `zscore(field)` | 最新価格のZスコア（bid/ask 専用メソッドに委譲） |
| `shrink(limit)` | `pop_front()` で O(1) トリミング |
| `mid()` | `Decimal::TWO` 定数で中間値計算 |

---

## middleware/mouse.rs

`MouseController` トレイトと `mouse-rs` 実装。

```rust
pub trait MouseController {
    fn random_xy(&self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> (i32, i32);
    fn move_to(&self, x: i32, y: i32);
    fn order(&self, setting: &gui::Mouse);
}
```

テスト時はモック実装を注入可能。

---

## order_type/choose.rs

`OrderDispatcher` — 単一ワーカースレッドによる注文処理。

```rust
pub struct OrderDispatcher {
    tx: SyncSender<OrderRequest>,
}

impl OrderDispatcher {
    pub fn new() -> Self;                    // ワーカースレッド起動
    pub fn dispatch(&self, setting, tickers); // try_send でバックプレッシャー
}
```

`OrderType` enum によるディスパッチ:

```rust
match request.order_type {
    OrderType::Simple    => simple::process(),
    OrderType::BuyEntry | OrderType::SellEntry => entry::process(),
    OrderType::ExitOnly  => exit::process(),
    OrderType::Custom    => origin::process(),
}
```

---

## order_type/process.rs

処理状態のロック/アンロック。RwLock poison 対策済み。

```rust
pub fn lock(s: Arc<RwLock<Data>>)              // is_processing = true
pub fn unlock(s: Arc<RwLock<Data>>, order)     // is_processing = false + 注文記録
```

---

## テスト

### ユニットテスト（55テスト）

| モジュール | テスト数 | カバー対象 |
|-----------|---------|-----------|
| invoke/gui.rs | 26 | OrderType/Speed serde、Setting 全メソッド、Status 遷移、Order、Mouse、Data |
| middleware/ticker.rs | 15 | Ticker mid/flag/diff_micros、TickerStats 全メソッド |
| order_type/process.rs | 4 | lock/unlock、履歴 shrink |
| order_type/choose.rs | 3 | OrderDispatcher 生成・ディスパッチガード |
| order_type/origin.rs | 7 | 全フラグタイプ (0-6) |

### 統合テスト（11テスト）

`tests/integration_test.rs`:
- データライフサイクル（WrappedData 更新）
- OrderDispatcher スキップ動作
- TickerStats リアルデータ diff
- Setting/Data/Status の JSON フロントエンド互換性
- 定数値検証
- TCP サーバー bind/受信
