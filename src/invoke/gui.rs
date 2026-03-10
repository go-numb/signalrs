use core::fmt;
use std::{
    cmp::Ordering,
    collections::VecDeque,
    str::FromStr,
    sync::{Arc, RwLock},
};

use chrono::{DateTime, Utc};
use log::trace;
use rand::Rng;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use tauri::State;

use crate::middleware::{mouse, utils};

#[derive(Debug, Clone, Default)]
pub struct WrappedData {
    pub data: Arc<RwLock<Data>>,
}

impl WrappedData {
    pub fn new(data: Data) -> Self {
        WrappedData {
            data: Arc::new(RwLock::new(data)),
        }
    }

    pub fn update(&self, update_exit_price: bool, ltp: Decimal) {
        let mut locked_data = self.data.write().unwrap();
        locked_data.status.update_ltp(ltp);

        if !update_exit_price {
            return;
        }

        if let Some(last_order) = locked_data.status.orders.last_mut() {
            if last_order.exit != Decimal::ZERO {
                return;
            }
            // 現在価格を追記する
            last_order.exit = ltp;
            // 終了時間を追記する
            last_order.exited_at = Utc::now();

            locked_data.status.message = format!("order update: {:?}", last_order);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    pub version: String,
    pub description: String,
    pub host: String,
    // 稼働状態
    pub status: Status,
    // 設定
    pub mouse_entry_buy: Mouse,
    pub mouse_entry_sell: Mouse,
    pub mouse_exit: Mouse,
    pub setting: Setting,

    // その他の設定
    // 選択可能なオプション
    pub order_type: Vec<Op>,
    pub speed: Vec<Op>,
}

impl Default for Data {
    fn default() -> Self {
        Data::new(None, None)
    }
}

impl Data {
    pub fn new(order_types: Option<Vec<Op>>, speeds: Option<Vec<Op>>) -> Self {
        let version = env!("CARGO_PKG_VERSION");
        let description = env!("CARGO_PKG_DESCRIPTION");

        let order_types = if let Some(values) = order_types {
            values
        } else {
            vec![
                Op {
                    value: "0".to_string(),
                    label: "方向不問".to_string(),
                },
                Op {
                    value: "1".to_string(),
                    label: "買いエントリーのみ".to_string(),
                },
                Op {
                    value: "2".to_string(),
                    label: "売りエントリーのみ".to_string(),
                },
                Op {
                    value: "3".to_string(),
                    label: "決済注文のみ".to_string(),
                },
                Op {
                    value: "99".to_string(),
                    label: "独自フラグ".to_string(),
                },
            ]
        };

        let speeds = if let Some(values) = speeds {
            values
        } else {
            vec![
                Op {
                    value: "0".to_string(),
                    label: "超高速".to_string(),
                },
                Op {
                    value: "1".to_string(),
                    label: "高速".to_string(),
                },
                Op {
                    value: "2".to_string(),
                    label: "中速".to_string(),
                },
                Op {
                    value: "3".to_string(),
                    label: "低速".to_string(),
                },
            ]
        };

        Data {
            version: version.to_string(),
            description: description.to_string(),
            host: "localhost".to_string(),
            status: Status::new(),
            mouse_entry_buy: Mouse::new(),
            mouse_entry_sell: Mouse::new(),
            mouse_exit: Mouse::new(),
            setting: Setting::new(),

            order_type: order_types,
            speed: speeds,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub is_recived: bool,
    pub is_running: bool,
    pub is_processing: bool,
    pub message: String,

    pub ltp: Decimal,
    pub orders: VecDeque<Order>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Status {
    fn default() -> Self {
        Status {
            is_recived: false,
            is_running: false,
            is_processing: false,
            message: "off".to_string(),

            ltp: Decimal::ZERO,
            orders: VecDeque::new(),
            updated_at: Utc::now(),
        }
    }
}

impl Status {
    pub fn new() -> Self {
        Status {
            is_recived: false,
            is_running: false,
            is_processing: false,
            message: "off".to_string(),

            ltp: Decimal::ZERO,
            orders: VecDeque::new(),
            updated_at: Utc::now(),
        }
    }

    pub fn processing(&mut self) {
        self.is_processing = true;
    }

    pub fn processed(&mut self) {
        self.is_processing = false;
    }

    // 注文履歴
    pub fn update_ltp(&mut self, ltp: Decimal) {
        if self.ltp.cmp(&ltp) != Ordering::Equal {
            self.is_recived = true;
            self.ltp = ltp;
            self.updated_at = Utc::now();
        } else {
            self.is_recived = false;
        }
    }

    pub fn push(&mut self, order: Order) {
        self.orders.push_back(order);
    }

    // 指定配列数に縮小する
    pub fn shrink(&mut self, limit_length: usize) {
        // limit_length以上の古い部分を捨てる
        while self.orders.len() > limit_length {
            self.orders.pop_front();
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Order {
    pub side: String,
    pub entry: Decimal,
    pub exit: Decimal,
    pub entried_at: DateTime<Utc>,
    pub exited_at: DateTime<Utc>,
}

impl fmt::Debug for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "side: {}, ref entry: {}, ref exit: {}, entried_at: {}, exited_at: {}",
            self.side,
            self.entry,
            self.exit,
            self.entried_at.format("%H:%M:%S UTC"),
            self.exited_at.format("%H:%M:%S UTC")
        )
    }
}

impl Order {
    pub fn new(entry: Decimal) -> Self {
        Order {
            side: String::new(),
            entry,
            exit: Decimal::ZERO,
            entried_at: Utc::now(),
            exited_at: Utc::now(),
        }
    }

    pub fn done(&mut self, exit: Option<Decimal>) -> Self {
        self.exit = if let Some(exit) = exit {
            exit
        } else {
            Decimal::ZERO
        };
        self.exited_at = Utc::now();
        self.clone()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mouse {
    pub start_x: u32,
    pub start_y: u32,
    pub end_x: u32,
    pub end_y: u32,
    pub n: u8,
}

impl Mouse {
    pub fn new() -> Self {
        Mouse {
            start_x: 1,
            start_y: 1,
            end_x: 2,
            end_y: 2,
            n: 1,
        }
    }

    pub fn ok(&self) -> Result<(), String> {
        // start_x < end_x, start_y < end_y
        if self.start_x < self.end_x && self.start_y < self.end_y {
            Ok(())
        } else {
            Err("error: invalid mouse setting".to_string())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    #[serde(rename = "0")]
    Simple,
    #[serde(rename = "1")]
    BuyEntry,
    #[serde(rename = "2")]
    SellEntry,
    #[serde(rename = "3")]
    ExitOnly,
    #[serde(rename = "99")]
    Custom,
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Simple
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Speed {
    #[serde(rename = "0")]
    UltraFast,
    #[serde(rename = "1")]
    Fast,
    #[serde(rename = "2")]
    Medium,
    #[serde(rename = "3")]
    Slow,
}

impl Default for Speed {
    fn default() -> Self {
        Speed::Fast
    }
}

impl Speed {
    pub fn to_ms(&self) -> i64 {
        match self {
            Speed::UltraFast => 1,
            Speed::Fast => 100,
            Speed::Medium => 1_000,
            Speed::Slow => 3_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub tcp: String,
    pub order_type: OrderType,
    pub speed: Speed,
    pub vol: String,
    pub interval: u32,
    pub interval_random: bool,
}

impl Default for Setting {
    fn default() -> Self {
        Setting::new()
    }
}

impl Setting {
    pub fn new() -> Self {
        Setting {
            tcp: "8080".to_string(),
            order_type: OrderType::Simple,
            speed: Speed::Fast,
            vol: "0.1".to_string(),
            interval: 10,
            interval_random: false,
        }
    }
    // CORE: 設定値を条件用数値に変換する
    pub fn speed_to_ms(&self) -> i64 {
        self.speed.to_ms()
    }
    /// 条件分岐に使用する諸情報を取得する
    /// interval_random::trueの場合はランダムな時間待機する
    pub fn get(&self) -> (i64, Decimal) {
        let target_diff_micros = self.speed.to_ms() * 1000;
        let target_diff_ticks = Decimal::from_str(self.vol.as_str()).unwrap();

        (target_diff_micros, target_diff_ticks)
    }

    // 設定値から待機時間を取得する
    pub fn get_sleep_ms(&self) -> u64 {
        if self.interval_random {
            // interval sec to min, max millisec
            let min_ms = (self.interval as f32 * 0.5 * 1000f32) as u32;
            let max_ms = (self.interval as f32 * 1.5 * 1000f32) as u32;
            let mut rng = rand::thread_rng();
            rng.gen_range(min_ms..max_ms) as u64
        } else {
            self.interval as u64 * 1000u64
        }
    }

    /// 文字列フィールドをパース済みの型付き構造体として返す
    pub fn parsed(&self) -> ParsedSetting {
        ParsedSetting {
            order_type: self.order_type,
            vol: Decimal::from_str(self.vol.as_str()).unwrap_or(Decimal::new(1, 1)),
            interval: self.interval,
            interval_random: self.interval_random,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedSetting {
    pub order_type: OrderType,
    pub vol: Decimal,
    pub interval: u32,
    pub interval_random: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Op {
    value: String,
    label: String,
}

impl Default for Op {
    fn default() -> Self {
        Op {
            value: "0".to_string(),
            label: "option".to_string(),
        }
    }
}

// 稼働命令を受ける関数
#[tauri::command]
pub async fn run(state: State<'_, Arc<RwLock<Data>>>, t: u8) -> Result<String, String> {
    let s = match t {
        0 => {
            // stop
            {
                let mut locked_data = state.write().unwrap();
                locked_data.status.is_running = false;
                locked_data.status.is_processing = false;
                locked_data.status.message = "off".to_string();
            }
            "off".to_string()
        }
        1 => {
            // start
            {
                let mut locked_data = state.write().unwrap();
                locked_data.status.is_running = true;
                locked_data.status.message = "on".to_string();
            }
            "on".to_string()
        }
        2 => {
            // get status, confirm running
            let status = {
                let locked_data = state.read().unwrap();

                locked_data.status.clone()
            };
            json!(status).to_string()
        }
        _ => Err("error: undefined".to_string())?,
    };

    Ok(s)
}

/// get:: 設定を受ける関数
/// t: 1: setting, 2: mouse_exit, 3: mouse_entry_buy, 4: mouse_entry_sell
#[tauri::command]
pub async fn get(state: State<'_, Arc<RwLock<Data>>>, _t: u8) -> Result<Data, String> {
    let data = {
        let locked_data = state.read().unwrap();
        locked_data.clone()
    };

    trace!("{:?}", data);

    Ok(data)
}

/// set:: 設定を受ける関数
/// t: 1: setting, 2: mouse_click_times, 3: mouse_entry_buy, 4: mouse_entry_sell, 5: mouse_exit
#[tauri::command]
pub async fn set(state: State<'_, Arc<RwLock<Data>>>, t: u8, v: Value) -> Result<String, String> {
    match t {
        1 => {
            // 設定を受け取る
            let recived: Setting = serde_json::from_value(v).unwrap();
            let recived = {
                let mut locked_data = state.write().unwrap();

                locked_data.setting = recived;
                locked_data.setting.clone()
            };
            trace!("request value: {:?}", recived);
            Ok(format!("{:?}", recived))
        }
        2 => {
            // 決済クリックの回数を受け取る
            let n: i32 = serde_json::from_value(v).unwrap();
            let n = {
                let mut locked_data = state.write().unwrap();
                locked_data.mouse_exit.n = n as u8;
                n
            };
            Ok(format!("{:?}", n))
        }
        3 => {
            // 買い注文座標の設定を受け取る
            let mouse: Mouse = serde_json::from_value(v).unwrap();
            mouse.ok()?;
            let mouse_entry_buy = {
                let mut locked_data = state.write().unwrap();
                locked_data.mouse_entry_buy = mouse;
                locked_data.mouse_entry_buy.clone()
            };

            Ok(format!("{:?}", mouse_entry_buy))
        }
        4 => {
            // 売り注文座標の設定を受け取る
            let mouse: Mouse = serde_json::from_value(v).unwrap();
            mouse.ok()?;
            let mouse_entry_sell = {
                let mut locked_data = state.write().unwrap();
                locked_data.mouse_entry_sell = mouse;
                locked_data.mouse_entry_sell.clone()
            };

            Ok(format!("{:?}", mouse_entry_sell))
        }
        5 => {
            // 決済座標の設定を受け取る
            let mouse: Mouse = serde_json::from_value(v).unwrap();
            mouse.ok()?;
            let mouse_exit = {
                let mut locked_data = state.write().unwrap();
                locked_data.mouse_exit = mouse;
                locked_data.mouse_exit.clone()
            };
            Ok(format!("{:?}", mouse_exit))
        }

        _ => Ok("stoped".to_string()),
    }
}

/// confirm:: マウスの座標を確認する
/// t: 3: mouse_entry_buy, 4: mouse_entry_sell, 5: mouse_exit
/// n: 確認回数
#[tauri::command]
pub async fn confirm(state: State<'_, Arc<RwLock<Data>>>, t: u8, n: u8) -> Result<String, String> {
    let mouse_setting = {
        let locked_data = state.read().unwrap();
        // リクエスト分岐
        // 使用設定値を決定する
        // 1: entry, 2: exit
        match t {
            3 => locked_data.mouse_entry_buy.clone(),
            4 => locked_data.mouse_entry_sell.clone(),
            5 => locked_data.mouse_exit.clone(),
            _ => Mouse::default(),
        }
    };

    let mouse_c = mouse::Mouse::default();

    let (min_x, min_y, max_x, max_y) = (
        mouse_setting.start_x as i32,
        mouse_setting.start_y as i32,
        mouse_setting.end_x as i32,
        mouse_setting.end_y as i32,
    );

    let mut xs = Vec::with_capacity(n as usize);
    let mut ys = Vec::with_capacity(n as usize);

    for _ in 0..n {
        let (x, y) = mouse_c.random_xy(min_x, min_y, max_x, max_y);
        mouse_c.move_to(x, y);
        xs.push(x);
        ys.push(y);
        utils::sleep(0, 500);
    }

    Ok(format!("{} times, x: {:?}, y: {:?}", n, xs, ys))
}

// 乱数の出現テストを行う
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_random_xy() {
        let mouse_c = mouse::Mouse::default();
        let (min_x, min_y, max_x, max_y) = (1, 1, 2, 2);
        let (x, y) = mouse_c.random_xy(min_x, min_y, max_x, max_y);

        assert!(min_x <= x && x <= max_x);
        assert!(min_y <= y && y <= max_y);
    }

    #[test]
    fn test_wrapped_data_update() {
        // Setup initial data
        let mut data = Data::default();
        let initial_ltp = Decimal::new(100, 0);
        let updated_ltp = Decimal::new(200, 0);

        // Add an order
        let mut order = Order::new(initial_ltp);
        order.side = "BUY".to_string();
        data.status.orders.push_back(order);

        let wrapped = WrappedData::new(data);

        // Test update without exit price update
        wrapped.update(false, updated_ltp);
        {
            let locked = wrapped.data.read().unwrap();
            assert_eq!(locked.status.ltp, updated_ltp);
            assert_eq!(locked.status.orders[0].exit, Decimal::ZERO);
        }

        // Test update with exit price update
        wrapped.update(true, updated_ltp);
        {
            let locked = wrapped.data.read().unwrap();
            assert_eq!(locked.status.ltp, updated_ltp);
            assert_eq!(locked.status.orders[0].exit, updated_ltp);
        }
    }

    use std::str::FromStr;

    // --- OrderType enum ---
    #[test]
    fn test_order_type_default() {
        assert_eq!(OrderType::default(), OrderType::Simple);
    }

    #[test]
    fn test_order_type_serde_roundtrip() {
        // Verify serde rename works with JSON strings
        let json = serde_json::to_string(&OrderType::Simple).unwrap();
        assert_eq!(json, "\"0\"");
        let json = serde_json::to_string(&OrderType::BuyEntry).unwrap();
        assert_eq!(json, "\"1\"");
        let json = serde_json::to_string(&OrderType::Custom).unwrap();
        assert_eq!(json, "\"99\"");

        // Deserialize
        let ot: OrderType = serde_json::from_str("\"0\"").unwrap();
        assert_eq!(ot, OrderType::Simple);
        let ot: OrderType = serde_json::from_str("\"99\"").unwrap();
        assert_eq!(ot, OrderType::Custom);
    }

    // --- Speed enum ---
    #[test]
    fn test_speed_to_ms() {
        assert_eq!(Speed::UltraFast.to_ms(), 1);
        assert_eq!(Speed::Fast.to_ms(), 100);
        assert_eq!(Speed::Medium.to_ms(), 1_000);
        assert_eq!(Speed::Slow.to_ms(), 3_000);
    }

    #[test]
    fn test_speed_serde_roundtrip() {
        let json = serde_json::to_string(&Speed::Fast).unwrap();
        assert_eq!(json, "\"1\"");
        let sp: Speed = serde_json::from_str("\"2\"").unwrap();
        assert_eq!(sp, Speed::Medium);
    }

    // --- Setting ---
    #[test]
    fn test_setting_new_defaults() {
        let s = Setting::new();
        assert_eq!(s.tcp, "8080");
        assert_eq!(s.order_type, OrderType::Simple);
        assert_eq!(s.speed, Speed::Fast);
        assert_eq!(s.vol, "0.1");
        assert_eq!(s.interval, 10);
        assert!(!s.interval_random);
    }

    #[test]
    fn test_setting_speed_to_ms() {
        let mut s = Setting::new();
        assert_eq!(s.speed_to_ms(), 100); // Fast
        s.speed = Speed::Slow;
        assert_eq!(s.speed_to_ms(), 3_000);
    }

    #[test]
    fn test_setting_get() {
        let s = Setting::new();
        let (micros, vol) = s.get();
        assert_eq!(micros, 100_000); // 100ms * 1000
        assert_eq!(vol, Decimal::from_str("0.1").unwrap());
    }

    #[test]
    fn test_setting_get_sleep_ms_fixed() {
        let s = Setting::new(); // interval=10, random=false
        assert_eq!(s.get_sleep_ms(), 10_000);
    }

    #[test]
    fn test_setting_get_sleep_ms_random() {
        let mut s = Setting::new();
        s.interval = 10;
        s.interval_random = true;
        let ms = s.get_sleep_ms();
        assert!(ms >= 5_000 && ms <= 15_000, "random sleep was {} ms", ms);
    }

    #[test]
    fn test_setting_parsed() {
        let s = Setting::new();
        let p = s.parsed();
        assert_eq!(p.order_type, OrderType::Simple);
        assert_eq!(p.vol, Decimal::from_str("0.1").unwrap());
        assert_eq!(p.interval, 10);
        assert!(!p.interval_random);
    }

    #[test]
    fn test_setting_serde_roundtrip() {
        let s = Setting::new();
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: Setting = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.order_type, s.order_type);
        assert_eq!(deserialized.speed, s.speed);
    }

    // --- Status ---
    #[test]
    fn test_status_processing_flag() {
        let mut status = Status::new();
        assert!(!status.is_processing);
        status.processing();
        assert!(status.is_processing);
        status.processed();
        assert!(!status.is_processing);
    }

    #[test]
    fn test_status_update_ltp_changed() {
        let mut status = Status::new();
        status.update_ltp(Decimal::new(100, 0));
        assert!(status.is_recived);
        assert_eq!(status.ltp, Decimal::new(100, 0));
    }

    #[test]
    fn test_status_update_ltp_unchanged() {
        let mut status = Status::new();
        status.update_ltp(Decimal::ZERO);
        assert!(!status.is_recived); // same as initial value
    }

    #[test]
    fn test_status_push_and_shrink() {
        let mut status = Status::new();
        for i in 0..12 {
            status.push(Order::new(Decimal::new(i, 0)));
        }
        assert_eq!(status.orders.len(), 12);
        status.shrink(8);
        assert_eq!(status.orders.len(), 8);
        // First remaining order should be entry=4 (oldest 4 removed)
        assert_eq!(status.orders[0].entry, Decimal::new(4, 0));
    }

    // --- Order ---
    #[test]
    fn test_order_new() {
        let order = Order::new(Decimal::new(150, 0));
        assert_eq!(order.entry, Decimal::new(150, 0));
        assert_eq!(order.exit, Decimal::ZERO);
        assert!(order.side.is_empty());
    }

    #[test]
    fn test_order_done_with_exit() {
        let mut order = Order::new(Decimal::new(100, 0));
        let result = order.done(Some(Decimal::new(105, 0)));
        assert_eq!(result.exit, Decimal::new(105, 0));
    }

    #[test]
    fn test_order_done_without_exit() {
        let mut order = Order::new(Decimal::new(100, 0));
        let result = order.done(None);
        assert_eq!(result.exit, Decimal::ZERO);
    }

    // --- Mouse ---
    #[test]
    fn test_mouse_ok_valid() {
        let m = Mouse { start_x: 0, start_y: 0, end_x: 10, end_y: 10, n: 1 };
        assert!(m.ok().is_ok());
    }

    #[test]
    fn test_mouse_ok_invalid() {
        let m = Mouse { start_x: 10, start_y: 10, end_x: 5, end_y: 5, n: 1 };
        assert!(m.ok().is_err());
    }

    #[test]
    fn test_mouse_ok_equal_coords() {
        let m = Mouse { start_x: 5, start_y: 5, end_x: 5, end_y: 5, n: 1 };
        assert!(m.ok().is_err()); // equal is invalid
    }

    // --- Data ---
    #[test]
    fn test_data_default() {
        let d = Data::default();
        assert_eq!(d.order_type.len(), 5);
        assert_eq!(d.speed.len(), 4);
        assert_eq!(d.host, "localhost");
    }

    #[test]
    fn test_data_new_with_custom_options() {
        let custom_ops = vec![Op::default()];
        let d = Data::new(Some(custom_ops.clone()), Some(custom_ops));
        assert_eq!(d.order_type.len(), 1);
        assert_eq!(d.speed.len(), 1);
    }
}
