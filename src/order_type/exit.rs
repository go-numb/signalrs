use std::sync::{Arc, RwLock};

use crate::{
    invoke,
    middleware::{mouse, ticker::TickerStats, utils},
    order_type::process,
};

use log::info;

/// 決済のみの注文を行う
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
        // 決済のマウス操作
        let mouse_c = mouse::Mouse::default();

        // 決済注文のマウス操作
        let mouse = readed.mouse_exit.clone();
        let n = mouse.n;
        for _ in 0..n {
            mouse_c.order(&mouse);
            utils::sleep(1, 0);
        }
    }

    process::unlock(logic_setting.clone(), None);
}
