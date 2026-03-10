use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use log::error;
use rust_decimal::{prelude::Zero, Decimal, MathematicalOps};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Ticker {
    pub symbol: String,
    pub bid: Decimal,
    pub ask: Decimal,
    // optional fields
    pub flag: Option<u8>,
    pub side: Option<u8>,
    pub server_at: Option<DateTime<Utc>>,
    pub recived_at: Option<DateTime<Utc>>,
    pub diff_micros: Option<i64>,
}

impl Ticker {
    // CORE: フラグを取得する
    pub fn flag(&self) -> u8 {
        self.flag.unwrap_or(0)
    }

    pub fn mid(&self) -> Decimal {
        (self.bid + self.ask) / Decimal::TWO
    }

    // CORE: サーバー時刻と受信時刻の差分を計算する
    pub fn culc_diff_micros(&mut self) {
        self.recived_at = Some(Utc::now());
        let server_at = self.server_at.unwrap_or(Utc::now());
        let recived_at = self.recived_at.unwrap_or(Utc::now());
        let diff = recived_at.signed_duration_since(server_at);
        self.diff_micros = Some(diff.num_microseconds().unwrap_or(0));
    }
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TickerStats {
    data: VecDeque<Ticker>,
}

impl From<Vec<Ticker>> for TickerStats {
    fn from(tickers: Vec<Ticker>) -> Self {
        TickerStats {
            data: VecDeque::from(tickers),
        }
    }
}

impl TickerStats {
    pub fn new() -> Self {
        TickerStats {
            data: VecDeque::new(),
        }
    }

    pub fn push(&mut self, t: Ticker) {
        self.data.push_back(t);
    }

    pub fn last(&self) -> Option<&Ticker> {
        self.data.back()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    // mid price
    pub fn mid(&self) -> Decimal {
        let t = self.last().unwrap();
        (t.bid + t.ask) / Decimal::TWO
    }

    pub fn zscore(&self, field: &str) -> Result<Decimal, String> {
        self.data.calculate_zscore_last(field)
    }

    // 現在より指定micros以上前のデータを取得する
    // 配列で最新かつ指定micros以上前のデータを取得する
    // 現在値との差分を計算するために使用する
    pub fn filter_micros(&self, micros: i64) -> Option<&Ticker> {
        let latest = self.last().and_then(|f| f.recived_at.as_ref()).unwrap();

        // Timestamps are monotonic, so we can binary search.
        // We want the last element whose recived_at is more than `micros` before `latest`.
        // Use partition_point on slices obtained from the VecDeque.
        let (front, back) = self.data.as_slices();

        // Helper: check if a ticker's timestamp is within the micros threshold
        // We want the partition point where diff > micros transitions to diff <= micros
        // Since timestamps are monotonic (oldest first), diffs decrease as we go forward.
        // partition_point finds first element where predicate is false.
        // Predicate: diff > micros (i.e., element is old enough)
        let pred = |t: &Ticker| -> bool {
            let diff = latest.signed_duration_since(t.recived_at.unwrap());
            diff.num_microseconds().unwrap() > micros
        };

        // Search back slice first (it contains later elements)
        let back_point = back.partition_point(|t| pred(t));
        if back_point > 0 {
            return Some(&back[back_point - 1]);
        }

        // Then search front slice
        let front_point = front.partition_point(|t| pred(t));
        if front_point > 0 {
            return Some(&front[front_point - 1]);
        }

        None
    }

    // CORE: 指定配列と現在の価格との差分を計算する
    // ターゲットとなる価格と現在の価格を比較して差分を計算する
    // ターゲットとなる価格は指定micros以上前の価格とし、filter_microsで取得する
    pub fn diff(&self, micros: i64) -> Decimal {
        let target = self.filter_micros(micros);
        if let Some(t) = target {
            let mid = self.mid();
            let target_mid = t.mid();
            mid - target_mid
        } else {
            Decimal::zero()
        }
    }

    // 指定配列数に縮小する
    pub fn shrink(&mut self, limit_length: usize) {
        while self.data.len() > limit_length {
            self.data.pop_front();
        }
    }
}

// Vec<Ticker> / VecDeque<Ticker>への汎用的な処理
pub trait Tickers {
    #[allow(unused)]
    fn std(&self) -> Option<Decimal>;
    #[allow(unused)]
    fn mean(&self) -> Decimal;
    fn mid(&self) -> Decimal;

    fn zscore_bid(&self) -> Result<Decimal, String>;
    fn zscore_ask(&self) -> Result<Decimal, String>;
    fn calculate_zscore_last(&self, field: &str) -> Result<Decimal, String>;
}


impl Tickers for Vec<Ticker> {
    fn std(&self) -> Option<Decimal> {
        let mid = self.mean();
        let sum = self.iter().map(|t| { let diff = t.mid() - mid; diff * diff }).sum::<Decimal>();
        let count = Decimal::from(self.len());
        (sum / count).sqrt()
    }
    fn mean(&self) -> Decimal {
        let sum = self.iter().map(|t| t.mid()).sum::<Decimal>();
        sum / Decimal::from(self.len())
    }
    fn mid(&self) -> Decimal { self.mean() }
    fn zscore_bid(&self) -> Result<Decimal, String> { zscore_field(self.iter(), self.len(), self.last().map(|t| t.bid), |t| t.bid) }
    fn zscore_ask(&self) -> Result<Decimal, String> { zscore_field(self.iter(), self.len(), self.last().map(|t| t.ask), |t| t.ask) }
    fn calculate_zscore_last(&self, field: &str) -> Result<Decimal, String> {
        match field { "bid" => self.zscore_bid(), "ask" => self.zscore_ask(), _ => Err("無効なフィールド名です。".to_string()) }
    }
}

impl Tickers for VecDeque<Ticker> {
    fn std(&self) -> Option<Decimal> {
        let mid = self.mean();
        let sum = self.iter().map(|t| { let diff = t.mid() - mid; diff * diff }).sum::<Decimal>();
        let count = Decimal::from(self.len());
        (sum / count).sqrt()
    }
    fn mean(&self) -> Decimal {
        let sum = self.iter().map(|t| t.mid()).sum::<Decimal>();
        sum / Decimal::from(self.len())
    }
    fn mid(&self) -> Decimal { self.mean() }
    fn zscore_bid(&self) -> Result<Decimal, String> { zscore_field(self.iter(), self.len(), self.back().map(|t| t.bid), |t| t.bid) }
    fn zscore_ask(&self) -> Result<Decimal, String> { zscore_field(self.iter(), self.len(), self.back().map(|t| t.ask), |t| t.ask) }
    fn calculate_zscore_last(&self, field: &str) -> Result<Decimal, String> {
        match field { "bid" => self.zscore_bid(), "ask" => self.zscore_ask(), _ => Err("無効なフィールド名です。".to_string()) }
    }
}

/// Shared zscore calculation
fn zscore_field<'a>(
    iter: impl Iterator<Item = &'a Ticker>,
    len: usize,
    last_value: Option<Decimal>,
    field_fn: fn(&Ticker) -> Decimal,
) -> Result<Decimal, String> {
    if len < 2 {
        error!("データ数が2つ未満です。Zスコアを計算できません。");
        return Ok(Decimal::ZERO);
    }
    let n = Decimal::from(len as i64);
    let (sum, sum_sq) = iter.fold((Decimal::ZERO, Decimal::ZERO), |(s, sq), x| {
        let v = field_fn(x);
        (s + v, sq + v * v)
    });
    let mean = sum / n;
    let variance = (sum_sq - sum * sum / n) / (n - Decimal::ONE);
    let std_dev = variance.sqrt().unwrap();
    let last = last_value.unwrap();
    Ok((last - mean) / std_dev)
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, vec};

    use super::*;
    use rust_decimal::prelude::FromPrimitive;

    #[test]
    fn test_ticker_stats() {
        let tickers = vec![
            Ticker {
                symbol: "USDJPY".to_string(),
                bid: Decimal::from_f32(110.0).unwrap(),
                ask: Decimal::from_f32(110.1).unwrap(),
                flag: Some(0),
                side: Some(0),
                server_at: Some(Utc::now()),
                recived_at: Some(Utc::now()),
                diff_micros: Some(0),
            },
            Ticker {
                symbol: "USDJPY".to_string(),
                bid: Decimal::from_f32(110.1).unwrap(),
                ask: Decimal::from_f32(110.2).unwrap(),
                flag: Some(0),
                side: Some(0),
                server_at: Some(Utc::now()),
                recived_at: Some(Utc::now()),
                diff_micros: Some(0),
            },
        ];

        let ts = TickerStats::from(tickers);

        assert_eq!(ts.len(), 2);
        assert_eq!(ts.last().unwrap().symbol, "USDJPY");
        assert_eq!(ts.mid(), Decimal::from_f32(110.15).unwrap());
    }

    #[test]
    #[ignore]
    fn test_ticker_stats_diff() {
        let mut ticker_from_csv = csv::Reader::from_path(
            r"C:\Users\o9oem\Documents\go-numb\_past\mt4-volatility\_data\USDJPY.csv",
        )
        .unwrap();

        // let threashold = Decimal::from_f32(0.05).unwrap();

        let mut results = vec![];

        let mut tickers = TickerStats::new();

        for (i, result) in ticker_from_csv.records().enumerate() {
            // 擬似的にserver_atを設定
            let mut server_at = Utc::now();
            server_at += chrono::Duration::milliseconds(i as i64 * 10);

            let record = result.unwrap();
            let row: Ticker = Ticker {
                symbol: "USDJPY".to_string(),
                bid: Decimal::from_str(record.get(3).unwrap()).unwrap(),
                ask: Decimal::from_str(record.get(2).unwrap()).unwrap(),
                flag: Some(0),
                side: Some(0),
                server_at: Some(server_at),
                recived_at: Some(server_at),
                diff_micros: Some(0),
            };

            tickers.push(row.clone());
            tickers.shrink(250);

            // let start = Utc::now();
            let zscore_bid = tickers.data.calculate_zscore_last("bid").unwrap();
            let zscore_ask = tickers.data.calculate_zscore_last("ask").unwrap();

            // let log_mean = tickers.data.log_mean().unwrap();
            results.push((zscore_ask.to_string(), zscore_bid.to_string()));
        }

        // save to csv
        let mut wtr = csv::Writer::from_path("../resource/result.csv").unwrap();
        wtr.write_record(["ask", "bid"]).unwrap();
        for (ask, bid) in results {
            wtr.write_record([&ask, &bid]).unwrap();
        }
        wtr.flush().unwrap();

        println!("{} : {:?}", tickers.len(), tickers.last().unwrap());
    }

    #[test]
    fn test_ticker_mid() {
        let t = Ticker {
            bid: Decimal::from_str("100.0").unwrap(),
            ask: Decimal::from_str("100.2").unwrap(),
            ..Default::default()
        };
        assert_eq!(t.mid(), Decimal::from_str("100.1").unwrap());
    }

    #[test]
    fn test_ticker_flag_default() {
        let t = Ticker::default();
        assert_eq!(t.flag(), 0);
    }

    #[test]
    fn test_ticker_flag_some() {
        let t = Ticker { flag: Some(3), ..Default::default() };
        assert_eq!(t.flag(), 3);
    }

    #[test]
    fn test_ticker_culc_diff_micros() {
        let mut t = Ticker {
            server_at: Some(Utc::now()),
            ..Default::default()
        };
        t.culc_diff_micros();
        assert!(t.recived_at.is_some());
        assert!(t.diff_micros.is_some());
        // diff should be very small (same process)
        assert!(t.diff_micros.unwrap().abs() < 1_000_000); // less than 1 second
    }

    #[test]
    fn test_ticker_stats_new_empty() {
        let ts = TickerStats::new();
        assert!(ts.is_empty());
        assert_eq!(ts.len(), 0);
        assert!(ts.last().is_none());
    }

    #[test]
    fn test_ticker_stats_push_and_len() {
        let mut ts = TickerStats::new();
        ts.push(Ticker::default());
        assert_eq!(ts.len(), 1);
        assert!(!ts.is_empty());
        ts.push(Ticker::default());
        assert_eq!(ts.len(), 2);
    }

    #[test]
    fn test_ticker_stats_shrink() {
        let mut ts = TickerStats::new();
        for _ in 0..20 {
            ts.push(Ticker {
                bid: Decimal::from_str("100.0").unwrap(),
                ask: Decimal::from_str("100.2").unwrap(),
                ..Default::default()
            });
        }
        assert_eq!(ts.len(), 20);
        ts.shrink(10);
        assert_eq!(ts.len(), 10);
        ts.shrink(10); // no-op
        assert_eq!(ts.len(), 10);
    }

    #[test]
    fn test_ticker_stats_diff_no_data() {
        let ts = TickerStats::from(vec![
            Ticker {
                bid: Decimal::from_str("100.0").unwrap(),
                ask: Decimal::from_str("100.2").unwrap(),
                recived_at: Some(Utc::now()),
                ..Default::default()
            },
        ]);
        // Only 1 ticker, filter_micros should find nothing
        let diff = ts.diff(1_000_000); // 1 second ago
        assert_eq!(diff, Decimal::ZERO);
    }

    #[test]
    fn test_ticker_stats_filter_micros() {
        use chrono::Duration;
        let now = Utc::now();
        let ts = TickerStats::from(vec![
            Ticker {
                bid: Decimal::from_str("100.0").unwrap(),
                ask: Decimal::from_str("100.0").unwrap(),
                recived_at: Some(now - Duration::milliseconds(500)),
                ..Default::default()
            },
            Ticker {
                bid: Decimal::from_str("101.0").unwrap(),
                ask: Decimal::from_str("101.0").unwrap(),
                recived_at: Some(now),
                ..Default::default()
            },
        ]);
        // Looking for ticker > 100ms ago - should find the first one
        let found = ts.filter_micros(100_000); // 100ms in micros
        assert!(found.is_some());
        assert_eq!(found.unwrap().bid, Decimal::from_str("100.0").unwrap());
    }

    #[test]
    fn test_ticker_stats_diff_positive() {
        use chrono::Duration;
        let now = Utc::now();
        let ts = TickerStats::from(vec![
            Ticker {
                bid: Decimal::from_str("100.0").unwrap(),
                ask: Decimal::from_str("100.0").unwrap(),
                recived_at: Some(now - Duration::milliseconds(200)),
                ..Default::default()
            },
            Ticker {
                bid: Decimal::from_str("101.0").unwrap(),
                ask: Decimal::from_str("101.0").unwrap(),
                recived_at: Some(now),
                ..Default::default()
            },
        ]);
        let diff = ts.diff(100_000); // 100ms
        assert_eq!(diff, Decimal::from_str("1.0").unwrap()); // 101 - 100 = 1
    }

    #[test]
    fn test_ticker_stats_zscore_insufficient_data() {
        let ts = TickerStats::from(vec![
            Ticker {
                bid: Decimal::from_str("100.0").unwrap(),
                ask: Decimal::from_str("100.0").unwrap(),
                ..Default::default()
            },
        ]);
        // Only 1 data point, zscore should return 0
        let result = ts.zscore("bid").unwrap();
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn test_ticker_stats_zscore_invalid_field() {
        let ts = TickerStats::from(vec![
            Ticker::default(),
            Ticker::default(),
        ]);
        let result = ts.zscore("invalid");
        assert!(result.is_err());
    }
}
