use std::sync::{Arc, RwLock};

use crate::{
    invoke::gui::Data,
    middleware::ticker::TickerStats,
    order_type::{entry, exit, origin, simple},
};

use log::{trace, warn};

fn prepare_settings(logic_setting: &Arc<RwLock<Data>>) -> Result<(u8, bool, bool), ()> {
    let read_setting = match logic_setting.read() {
        Ok(setting) => setting,
        Err(e) => {
            warn!("failed to read setting: {:?}", e);
            return Err(());
        }
    };
    let order_type = read_setting.setting.order_type.parse::<u8>().unwrap_or(0);
    Ok((order_type, read_setting.status.is_running, read_setting.status.is_processing))
}

pub fn by(logic_setting: Arc<RwLock<Data>>, tickers: &TickerStats) {
    // 設定値を読み込み、設定値に応じて処理を分岐する
    // read lockはここで解放される
    let (order_type, is_running, is_processing) = match prepare_settings(&logic_setting) {
        Ok(result) => result,
        Err(_) => return,
    };

    // 注文処理が停止中の場合は処理を行わない
    if !is_running {
        trace!("is not running");
        return;
    }

    // すでにprocessが動いている場合は処理を行わない
    if is_processing {
        trace!("is processing");
        return;
    }

    let setting = logic_setting.clone();
    let cloned_tickers = tickers.clone();
    match order_type {
        0 => {
            trace!("switch to simple logic");
            std::thread::spawn(move || simple::process(setting, &cloned_tickers));
        }
        1..=2 => {
            trace!("switch to buy/sell entry only logic");
            std::thread::spawn(move || entry::process(order_type, setting, &cloned_tickers));
        }
        3 => {
            trace!("switch to settlement order only logic");
            std::thread::spawn(move || exit::process(setting, &cloned_tickers));
        }
        99 => {
            trace!("switch to original order_type logic");
            std::thread::spawn(move || origin::process(setting, &cloned_tickers));
        }
        _ => {
            trace!("switch to simple logic");
            std::thread::spawn(move || simple::process(setting, &cloned_tickers));
        }
    }
}
