# 注文タイプ別処理ロジック

## 概要

signalrs は5種類の注文タイプを提供する。各タイプは `order_type/choose.rs` でルーティングされる。

---

## Type 0: Simple（シンプル）

**ファイル:** `order_type/simple.rs`

最も基本的な自動売買戦略。ボラティリティを検出してエントリーし、一定時間後に決済する。

### 処理フロー

```
① 設定・マウス座標を読み込み
② speed に基づく間隔で Ticker を受信
③ diff(target_micros) で価格変動を計算
④ |diff| > vol（閾値） を判定
⑤ 判定 OK の場合:
   ├─ lock() で処理ロック
   ├─ diff > 0 → Buy クリック × n 回
   │  diff < 0 → Sell クリック × n 回
   ├─ interval 秒待機（ランダム化あり）
   ├─ Exit クリック × n 回
   └─ unlock() でロック解除 + 注文記録
⑥ 判定 NG の場合 → ② に戻る
```

### ユースケース
- スキャルピング的な短期売買
- ボラティリティが高い時間帯での自動エントリー/決済

---

## Type 1: Entry Buy（買いエントリーのみ）

**ファイル:** `order_type/entry.rs`

買い方向の価格変動時のみエントリーする。決済は行わない。

### 処理フロー

```
① Ticker 受信
② diff(target_micros) 計算
③ diff > 0 かつ |diff| > vol を判定
④ 判定 OK の場合:
   ├─ lock()
   ├─ Buy クリック × n 回
   └─ unlock() + 注文記録
⑤ diff ≤ 0（売り方向）の場合 → 無視
```

### ユースケース
- 手動で決済したい場合
- 他のシステムと組み合わせて決済を別途管理する場合

---

## Type 2: Entry Sell（売りエントリーのみ）

**ファイル:** `order_type/entry.rs`（Type 1 と同じハンドラ）

売り方向の価格変動時のみエントリーする。

### 処理フロー

```
① Ticker 受信
② diff(target_micros) 計算
③ diff < 0 かつ |diff| > vol を判定
④ 判定 OK の場合:
   ├─ lock()
   ├─ Sell クリック × n 回
   └─ unlock() + 注文記録
⑤ diff ≥ 0（買い方向）の場合 → 無視
```

---

## Type 3: Exit（決済のみ）

**ファイル:** `order_type/exit.rs`

エントリーは行わず、ボラティリティ検出時に決済のみ実行する。

### 処理フロー

```
① Ticker 受信
② diff(target_micros) 計算
③ |diff| > vol を判定
④ 判定 OK の場合:
   ├─ lock()
   ├─ Exit クリック × n 回
   └─ unlock() + 注文記録
```

### ユースケース
- 既存ポジションの自動利確/損切り
- 手動でエントリー後、決済のみ自動化

---

## Type 99: Origin（カスタムフラグ制御）

**ファイル:** `order_type/origin.rs`（320行）

最も高機能な注文タイプ。MT5/MT4 から送信される `flag` 値で注文動作を完全に制御する。

### フラグ定義

```rust
enum Origin {
    None       = 0,   // 何もしない
    EntryBuy   = 1,   // 買いエントリーのみ
    EntrySell  = 2,   // 売りエントリーのみ
    EntryBuyExit  = 3, // 買いエントリー → 決済
    EntrySellExit = 4, // 売りエントリー → 決済
    ExitBuy    = 5,   // 買いポジション決済
    ExitSell   = 6,   // 売りポジション決済
}
```

### 処理フロー（flag=3: EntryBuyExit の例）

```
① Ticker 受信（flag=3 が含まれる）
② flag を Origin enum に変換
③ EntryBuyExit と判定:
   ├─ lock()
   ├─ Buy クリック × n 回
   ├─ interval 秒待機
   ├─ Exit クリック × n 回
   └─ unlock() + 注文記録
```

### ボラティリティ判定との関係

Origin モードでは **flag 値が最優先** される。ボラティリティ閾値による判定は行わず、MT5/MT4 側で判断した結果を flag で伝達する。

### ユースケース
- MT5/MT4 側で独自のインジケーター/EA により判断し、signalrs は純粋な実行エンジンとして使用
- 複雑な売買ロジックを MT5/MT4 側に集約し、signalrs はクリック操作のみ担当

---

## 注文タイプ比較表

| 機能 | Simple(0) | Buy(1) | Sell(2) | Exit(3) | Origin(99) |
|------|-----------|--------|---------|---------|------------|
| ボラティリティ判定 | YES | YES | YES | YES | NO (flag優先) |
| 買いエントリー | YES | YES | - | - | flag依存 |
| 売りエントリー | YES | - | YES | - | flag依存 |
| 自動決済 | YES | - | - | YES | flag依存 |
| MT5 flag 参照 | - | - | - | - | YES |
| 方向フィルタ | 双方向 | 上昇のみ | 下降のみ | 双方向 | flag指定 |

---

## 排他制御

全注文タイプで共通の排他制御が適用される:

```
is_processing = true の場合 → 新規注文を無視
```

`process.rs` の `lock()` / `unlock()` で制御し、二重注文を防止する。
