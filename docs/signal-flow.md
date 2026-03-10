# シグナルフロー（データフロー）

## 全体フロー

```
[MT5/MT4]                [signalrs]                         [ブラウザ]
    │                        │                                  │
    │  ① JSON/TCP送信        │                                  │
    │ ──────────────────►    │                                  │
    │                        │                                  │
    │                   ② Ticker へ                             │
    │                      デシリアライズ                        │
    │                        │                                  │
    │                   ③ TickerStats に                        │
    │                      価格を蓄積                           │
    │                        │                                  │
    │                   ④ ボラティリティ                         │
    │                      判定                                 │
    │                        │                                  │
    │                   ⑤ choose.rs で                          │
    │                      注文タイプ分岐                        │
    │                        │                                  │
    │                   ⑥ マウス操作                  ⑥ クリック │
    │                        │ ──────────────────────────────►  │
    │                        │                                  │
    │                   ⑦ 注文記録                               │
    │                        │                                  │
```

## 各ステップ詳細

### ① MT5/MT4 から TCP でデータ送信

MT5/MT4 の EA またはスクリプトが以下の JSON を TCP で送信する。

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

| フィールド | 型 | 説明 |
|-----------|-----|------|
| symbol | String | 通貨ペア名 |
| bid | Decimal | 売値 |
| ask | Decimal | 買値 |
| flag | Option\<u8\> | カスタム注文フラグ (0-6, 99) |
| side | Option\<u8\> | 売買方向 |
| server_at | DateTime | MT5/MT4 側のタイムスタンプ |

### ② TCP 受信とデシリアライズ

`middleware/tcp.rs` の `TcpClient<T>` が受信。

```rust
// TCP サーバーモード（指定ポートでリッスン）
tcp_client.received_server()

// TCP クライアントモード（外部サーバーへ接続）
tcp_client.connect()
```

JSON 文字列を `Ticker` 構造体にデシリアライズする。受信時に `recived_at` タイムスタンプを付与し、`diff_micros`（MT5 送信時刻との差分マイクロ秒）を計算する。

### ③ TickerStats にスライディングウィンドウで蓄積

`middleware/ticker.rs` の `TickerStats` が価格履歴を保持する。

```rust
pub struct TickerStats {
    tickers: Vec<Ticker>,  // スライディングウィンドウ
}
```

- `shrink()` で固定サイズに維持（メモリ制限）
- 各 Ticker は `mid()` メソッドで中間値 `(bid + ask) / 2` を計算

### ④ ボラティリティ判定

`diff(micros)` メソッドで指定マイクロ秒前の価格との差分を計算。

```rust
// 例: 過去 100ms (100,000 マイクロ秒) の価格変動
let diff = ticker_stats.diff(100_000);
```

設定された `vol`（ボラティリティ閾値）と比較:
- `|diff| > vol` → 注文実行トリガー
- `diff > 0` → 買い方向
- `diff < 0` → 売り方向

`zscore_last()` による統計的分析（標準偏差ベース）も利用可能。

### ⑤ 注文タイプによるルーティング

`order_type/choose.rs` が設定に基づき適切なハンドラに分岐する。

```
order_type = 0  →  simple::process()   (エントリー＋決済)
order_type = 1  →  entry::process()    (買いエントリーのみ)
order_type = 2  →  entry::process()    (売りエントリーのみ)
order_type = 3  →  exit::process()     (決済のみ)
order_type = 99 →  origin::process()   (フラグベース制御)
```

詳細は [order-types.md](./order-types.md) を参照。

### ⑥ マウス操作による注文実行

`middleware/mouse.rs` が Windows 上のブラウザに対してマウス操作を行う。

```
1. random_xy() で設定範囲内のランダム座標を生成
2. move_to(x, y) でカーソルを移動
3. click() で左クリック実行
4. 必要に応じて n 回繰り返し（設定の clicks 数）
```

ランダム座標を使用する理由:
- ボタンの中心を毎回正確にクリックすると機械的と判定されるリスク
- 設定範囲（start_x/y ～ end_x/y）内でランダムにずらす

### ⑦ 注文記録

注文完了後、`Order` 構造体として記録される。

```rust
Order {
    side: String,       // "buy" or "sell"
    entry: Decimal,     // エントリー価格
    exit: Decimal,      // 決済価格
    entried_at: DateTime,
    exited_at: DateTime,
}
```

最新8件を保持し、Tauri フロントエンドに表示する。

## レイテンシ追跡

各 Ticker は以下のタイムスタンプを持つ:
- `server_at`: MT5/MT4 側の送信時刻
- `recived_at`: signalrs 側の受信時刻
- `diff_micros`: 両者の差分（マイクロ秒）

これにより、シグナル伝達の遅延を監視できる。
