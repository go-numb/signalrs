use std::sync::{Arc, RwLock};

use crate::{
    invoke,
    invoke::gui::OrderType,
    middleware::{mouse, ticker::TickerStats},
    order_type::process,
};

use log::info;
use rust_decimal::{prelude::Zero, Decimal};

/// シンプルな注文及び決済処理を行う
/// 指定時間遡り、直近のTicker mid値と現在のTicker mid値の差分を計算し、設定値以上差が生じれば注文を行う
/// 指定時間待機し、決済注文を行う
pub fn process(t: OrderType, logic_setting: Arc<RwLock<invoke::gui::Data>>, tickers: &TickerStats) {
    process::lock(logic_setting.clone());

    let (setting, mouse_entry_buy, mouse_entry_sell) = {
        let read = match logic_setting.read() {
            Ok(setting) => setting,
            Err(e) => {
                info!("failed to read setting: {:?}", e);
                return;
            }
        };

        (
            read.setting.clone(),
            read.mouse_entry_buy.clone(),
            read.mouse_entry_sell.clone(),
        )
    };
    let (target_diff_micros, target_diff_ticks) = setting.get();

    let diff = tickers.diff(target_diff_micros);
    if target_diff_ticks < diff.abs() {
        // order_type BuyEntry: 買い注文, SellEntry: 売り注文
        let entry_mouse = match t {
            OrderType::BuyEntry => {
                if diff < Decimal::zero() {
                    info!("failed miss match order_type & trade side: {:?}", t);
                    return;
                }

                mouse_entry_buy
            }
            OrderType::SellEntry => {
                if diff > Decimal::zero() {
                    info!("failed miss match order_type & trade side: {:?}", t);
                    return;
                }

                mouse_entry_sell
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
