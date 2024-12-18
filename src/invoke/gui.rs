use core::fmt;
use std::{
    cmp::Ordering,
    str::FromStr,
    sync::{Arc, RwLock},
};

use chrono::{DateTime, Utc};
use log::trace;
use rand::Rng;
use rust_decimal::{prelude::Zero, Decimal};
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
            if last_order.exit != Decimal::zero() {
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
    pub orders: Vec<Order>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Status {
    fn default() -> Self {
        Status {
            is_recived: false,
            is_running: false,
            is_processing: false,
            message: "off".to_string(),

            ltp: Decimal::new(0, 0),
            orders: Vec::new(),
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

            ltp: Decimal::new(0, 0),
            orders: Vec::new(),
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

    #[allow(unused)]
    pub fn push(&mut self, order: Order) {
        self.orders.push(order);
    }

    #[allow(unused)]
    // 指定配列数に縮小する
    pub fn shrink(&mut self, limit_length: usize) {
        // limit_length以上の古い部分を捨てる
        let l = self.orders.len();
        if l > limit_length {
            let truncate = l - limit_length;
            // 0..truncateの範囲を捨てる
            self.orders.drain(0..truncate);
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
            exit: Decimal::new(0, 0),
            entried_at: Utc::now(),
            exited_at: Utc::now(),
        }
    }

    pub fn done(&mut self, exit: Option<Decimal>) -> Self {
        self.exit = if let Some(exit) = exit {
            exit
        } else {
            Decimal::new(0, 0)
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Setting {
    pub tcp: String,
    pub order_type: String,
    pub speed: String,
    pub vol: String,
    pub interval: u32,
    pub interval_random: bool,
}

impl Setting {
    pub fn new() -> Self {
        Setting {
            tcp: "8080".to_string(),
            order_type: "0".to_string(),
            speed: "1".to_string(),
            vol: "0.1".to_string(),
            interval: 10,
            interval_random: false,
        }
    }
    // CORE: 設定値を条件用数値に変換する
    pub fn speed_to_ms(&self) -> i64 {
        match self.speed.as_str() {
            "0" => 1,
            "1" => 100,
            "2" => 1_000,
            "3" => 3_000,
            _ => 10_000,
        }
    }
    /// 条件分岐に使用する諸情報を取得する
    /// interval_random::trueの場合はランダムな時間待機する
    pub fn get(&self) -> (i64, Decimal) {
        let target_diff_micros = self.speed_to_ms() * 1000;
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
            let message = {
                let mut locked_data = state.write().unwrap();
                locked_data.status.is_running = false;
                locked_data.status.is_processing = false;
                locked_data.status.message = "off".to_string();
                locked_data.status.message.clone()
            };
            message
        }
        1 => {
            // start
            let message = {
                let mut locked_data = state.write().unwrap();
                locked_data.status.is_running = true;
                locked_data.status.message = "on".to_string();
                locked_data.status.message.clone()
            };
            message
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

    Ok(data.clone())
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
        data.status.orders.push(order);

        let wrapped = WrappedData::new(data);

        // Test update without exit price update
        wrapped.update(false, updated_ltp);
        {
            let locked = wrapped.data.read().unwrap();
            assert_eq!(locked.status.ltp, updated_ltp);
            assert_eq!(locked.status.orders[0].exit, Decimal::new(0, 0));
        }

        // Test update with exit price update
        wrapped.update(true, updated_ltp);
        {
            let locked = wrapped.data.read().unwrap();
            assert_eq!(locked.status.ltp, updated_ltp);
            assert_eq!(locked.status.orders[0].exit, updated_ltp);
        }
    }
}
