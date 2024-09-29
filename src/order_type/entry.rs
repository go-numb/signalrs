use std::sync::{Arc, RwLock};

use crate::{
    invoke,
    middleware::{mouse, ticker::TickerStats},
    order_type::process,
};

use log::info;
use rust_decimal::{prelude::Zero, Decimal};

/// シンプルな注文及び決済処理を行う
/// 指定時間遡り、直近のTicker mid値と現在のTicker mid値の差分を計算し、設定値以上差が生じれば注文を行う
/// 指定時間待機し、決済注文を行う
pub fn process(logic_setting: Arc<RwLock<invoke::gui::Data>>, tickers: &TickerStats) {
    process::lock(logic_setting.clone());

    let readed = {
        let read = match logic_setting.read() {
            Ok(setting) => setting,
            Err(e) => {
                info!("failed to read setting: {:?}", e);
                return;
            }
        };

        read.clone()
    };
    let (target_diff_micros, target_diff_ticks) = readed.setting.get();

    let diff = tickers.diff(target_diff_micros);
    if target_diff_ticks < diff.abs() {
        // 0値は上記条件で弾かれるため内包する
        // 0 < diff = buy, 0 > diff = sell
        let entry_mouse = if diff > Decimal::zero() {
            readed.mouse_entry_buy.clone()
        } else {
            readed.mouse_entry_sell.clone()
        };

        // 新規注文のマウス操作
        let mouse_c = mouse::Mouse::default();
        mouse_c.order(&entry_mouse);
    }

    process::unlock(logic_setting.clone(), None);
}
