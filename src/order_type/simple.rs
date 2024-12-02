use std::sync::{Arc, RwLock};

use crate::{
    invoke::{
        self,
        gui::{Order, Setting},
    },
    middleware::{mouse, ticker::TickerStats, utils},
    order_type::process,
};

use log::{trace, warn};
use rust_decimal::{prelude::Zero, Decimal};

/// シンプルな注文及び決済処理を行う
/// 指定時間遡り、直近のTicker mid値と現在のTicker mid値の差分を計算し、設定値以上差が生じれば注文を行う
/// 指定時間待機し、決済注文を行う
pub fn process(logic_setting: Arc<RwLock<invoke::gui::Data>>, tickers: &TickerStats) {
    let (readed_setting, buy_mouse, sell_mouse, exit_mouse) = {
        let readed = match logic_setting.read() {
            Ok(setting) => setting,
            Err(e) => {
                warn!("failed to read setting: {:?}", e);
                return;
            }
        };

        let readed_setting = readed.setting.clone();
        let readed_entry_buy_mouse = readed.mouse_entry_buy.clone();
        let readed_entry_sell_mouse = readed.mouse_entry_sell.clone();
        let readed_exit_mouse = readed.mouse_exit.clone();
        (
            readed_setting,
            readed_entry_buy_mouse,
            readed_entry_sell_mouse,
            readed_exit_mouse,
        )
    };

    // 設定条件を取得
    let (target_diff_micros, target_diff_ticks) = readed_setting.get();
    trace!(
        "target order params - diff_micros: {}, diff_ticks: {}",
        target_diff_micros,
        target_diff_ticks,
    );

    let diff = tickers.diff(target_diff_micros);

    if target_diff_ticks < diff.abs() {
        // 処理中フラグを立てる
        process::lock(logic_setting.clone());

        let entry_price = tickers.last().unwrap().mid();
        let mut order = Order::new(entry_price);

        let entry_mouse = {
            // 0値は上記条件で弾かれるため内包する
            // 0 < diff = buy, 0 > diff = sell
            if diff > Decimal::zero() {
                order.side = "buy".to_string();
                buy_mouse
            } else {
                order.side = "sell".to_string();
                sell_mouse
            }
        };

        // 新規注文のマウス操作
        let mouse_c = mouse::Mouse::default();
        mouse_c.order(&entry_mouse);

        // 設定値待機する
        let target_sleep_ms = readed_setting.get_sleep_ms();
        utils::sleep(0, target_sleep_ms);

        // 決済注文のマウス操作
        let n = exit_mouse.n;
        for _ in 0..n {
            mouse_c.order(&exit_mouse);
            utils::sleep(1, 0);
        }

        order.done(None);

        // フラグを下げる
        process::unlock(logic_setting.clone(), Some(order));
    }
}
