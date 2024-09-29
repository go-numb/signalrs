use std::sync::{Arc, RwLock};

use crate::{
    invoke::gui::Data,
    middleware::ticker::TickerStats,
    order_type::{entry, exit, origin, simple},
};

use log::{trace, warn};

fn prepare_settings(logic_setting: Arc<RwLock<Data>>) -> Result<(u8, Data), ()> {
    let read_setting = match logic_setting.read() {
        Ok(setting) => setting,
        Err(e) => {
            warn!("failed to read setting: {:?}", e);
            return Err(());
        }
    };
    let order_type = read_setting.setting.order_type.parse::<u8>().unwrap(); // 設定値を読み込む
    let cloned_setting = read_setting.clone(); // 設定データをクローン

    Ok((order_type, cloned_setting))
}

pub fn by(logic_setting: Arc<RwLock<Data>>, tickers: &TickerStats) {
    // 設定値を読み込み、設定値に応じて処理を分岐する
    // read lockはここで解放される
    let (order_type, cloned_setting) = match prepare_settings(logic_setting.clone()) {
        Ok((order_type, cloned_setting)) => (order_type, cloned_setting),
        Err(_) => return,
    };

    // 注文処理が停止中の場合は処理を行わない
    if !cloned_setting.status.is_running {
        trace!("is not running");
        return;
    }

    // すでにprocessが動いている場合は処理を行わない
    if cloned_setting.status.is_processing {
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
            trace!("switch to buy entry only logic");
            std::thread::spawn(move || entry::process(setting, &cloned_tickers));
        }
        3 => {
            trace!("switch to settlement order only logic");
            std::thread::spawn(move || exit::process(setting, &cloned_tickers));
        }
        // 4..=5 => {
        //     let side = if order_type == 4 { "sell" } else { "buy" };
        //     trace!("switch to settlement for {} order only logic", side);
        //     std::thread::spawn(move || simple::process(setting, &cloned_tickers));
        // }
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
