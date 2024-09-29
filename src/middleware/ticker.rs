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
        (self.bid + self.ask) / Decimal::from(2)
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
    data: Vec<Ticker>,
}

impl From<Vec<Ticker>> for TickerStats {
    fn from(tickers: Vec<Ticker>) -> Self {
        TickerStats { data: tickers }
    }
}

impl TickerStats {
    pub fn new() -> Self {
        TickerStats { data: Vec::new() }
    }

    pub fn push(&mut self, t: Ticker) {
        self.data.push(t);
    }

    pub fn last(&self) -> Option<&Ticker> {
        self.data.last()
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
        (t.bid + t.ask) / Decimal::from(2)
    }

    pub fn zscore(&self, field: &str) -> Result<Decimal, String> {
        self.data.calculate_zscore_last(field)
    }

    // 現在より指定micros以上前のデータを取得する
    // 配列で最新かつ指定micros以上前のデータを取得する
    // 現在値との差分を計算するために使用する
    pub fn filter_micros(&self, micros: i64) -> Option<&Ticker> {
        let latest = self.last().and_then(|f| f.recived_at.as_ref()).unwrap();

        for t in self.data.iter().rev() {
            let diff = latest.signed_duration_since(t.recived_at.unwrap());
            if diff.num_microseconds().unwrap() > micros {
                return Some(t);
            }
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
        // limit_length以上の古い部分を捨てる
        if self.len() > limit_length {
            let truncate = self.len() - limit_length;
            // 0..truncateの範囲を捨てる
            self.data.drain(0..truncate);
        }
    }
}

// Vec<Ticker>への汎用的な処理
pub trait Tickers {
    #[allow(unused)]
    fn std(&self) -> Option<Decimal>;
    #[allow(unused)]
    fn mean(&self) -> Decimal;
    fn mid(&self) -> Decimal;

    fn calculate_zscore_last(&self, field: &str) -> Result<Decimal, String>;
}

impl Tickers for Vec<Ticker> {
    fn std(&self) -> Option<Decimal> {
        let mid = self.mid();
        let sum = self
            .iter()
            .map(|t| {
                let diff = t.mid() - mid;
                diff * diff
            })
            .sum::<Decimal>();

        let count = Decimal::from(self.len());
        let variance = sum / count;
        variance.sqrt()
    }

    fn mean(&self) -> Decimal {
        let sum = self.iter().map(|t| t.mid()).sum::<Decimal>();
        let count = Decimal::from(self.len());
        sum / count
    }

    fn mid(&self) -> Decimal {
        let sum = self.iter().map(|t| t.mid()).sum::<Decimal>();
        let count = Decimal::from(self.len());
        sum / count
    }

    fn calculate_zscore_last(&self, field: &str) -> Result<Decimal, String> {
        if field != "bid" && field != "ask" {
            return Err("無効なフィールド名です。".to_string());
        } else if self.len() < 2 {
            error!("データ数が2つ未満です。Zスコアを計算できません。");
            return Ok(Decimal::ZERO);
        }

        let (sum, sum_sq): (Decimal, Decimal) =
            self.iter()
                .fold((Decimal::ZERO, Decimal::ZERO), |(sum, sum_sq), x| {
                    let value = match field {
                        "bid" => x.bid,
                        "ask" => x.ask,
                        _ => return (Decimal::ZERO, Decimal::ZERO),
                    };
                    (sum + value, sum_sq + value * value)
                });

        let n = Decimal::from(self.len() as i64);
        let mean = sum / n;
        let variance = (sum_sq - sum * sum / n) / (n - Decimal::ONE);
        let std_dev = variance.sqrt().unwrap();

        let last_value = match field {
            "bid" => self.last().unwrap().bid,
            "ask" => self.last().unwrap().ask,
            _ => return Err("無効なフィールド名です。".to_string()),
        };
        Ok((last_value - mean) / std_dev)
    }
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
}
