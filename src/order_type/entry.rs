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
pub fn process(t: u8, logic_setting: Arc<RwLock<invoke::gui::Data>>, tickers: &TickerStats) {
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
        // order_type 1: 買い注文, 2: 売り注文
        let entry_mouse = match t {
            1 => {
                if diff < Decimal::zero() {
                    info!("failed miss match order_type & trade side: {:?}", t);
                    return;
                }

                readed.mouse_entry_buy.clone()
            }
            2 => {
                if diff > Decimal::zero() {
                    info!("failed miss match order_type & trade side: {:?}", t);
                    return;
                }

                readed.mouse_entry_sell.clone()
            }
            _ => {
                info!("failed order_type: {:?}", t);
                return;
            }
        };

        // 新規注文のマウス操作
        let mouse_c = mouse::Mouse::default();
        mouse_c.order(&entry_mouse);
    }

    process::unlock(logic_setting.clone(), None);
}
