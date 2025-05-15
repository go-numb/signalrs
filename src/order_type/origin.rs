use std::sync::{Arc, RwLock};

use crate::{
    invoke,
    middleware::{mouse, ticker::TickerStats, utils},
    order_type::process,
};

use log::{info, warn};

enum Flag {
    None,
    // 新規注文のみ
    EntryBuy,
    EntrySell,
    // 新規注文後待機を経て決済注文を行う
    EntryBuyExit,
    EntrySellExit,

    // 決済注文のみ
    ExitBuy,
    ExitSell,
}

// u8 to Flag
impl From<u8> for Flag {
    fn from(value: u8) -> Self {
        match value {
            0 => Flag::None,
            1 => Flag::EntryBuy,
            2 => Flag::EntrySell,
            3 => Flag::EntryBuyExit,
            4 => Flag::EntrySellExit,
            5 => Flag::ExitBuy,
            6 => Flag::ExitSell,
            _ => Flag::None,
        }
    }
}

/// フラグを受け取り、処理を分岐する
/// 注文可否の判定は親関数で行う
pub fn process(_logic_setting: Arc<RwLock<invoke::gui::Data>>, tickers: &TickerStats) {
    let lastest_ticker = match tickers.last() {
        Some(ticker) => ticker,
        None => {
            info!("no ticker data");
            return;
        }
    };

    info!("flag: {}", lastest_ticker.flag());

    #[allow(unreachable_code)]
    match Flag::from(lastest_ticker.flag()) {
        Flag::None => {
            info!("undefined flag");
        }
        Flag::EntryBuy => {
            info!("switch to entry buy");
            entry(true, _logic_setting);
        }
        Flag::EntrySell => {
            info!("switch to entry sell");
            entry(false, _logic_setting);
        }
        Flag::EntryBuyExit => {
            info!("switch to buy entry, wait until exit logic");
            entry_and_exit(true, _logic_setting);
        }
        Flag::EntrySellExit => {
            info!("switch to sell entry, wait until exit logic");
            entry_and_exit(false, _logic_setting);
        }
        Flag::ExitBuy => {
            info!("switch to buy exit only logic");
            exit(true, _logic_setting);
        }
        Flag::ExitSell => {
            info!("switch to sell exit only logic");
            exit(false, _logic_setting);
        }
    }
}

fn entry(is_buy: bool, logic_setting: Arc<RwLock<invoke::gui::Data>>) {
    process::lock(logic_setting.clone());

    let entry_mouse = {
        let read = match logic_setting.read() {
            Ok(setting) => setting,
            Err(e) => {
                warn!("failed to read setting: {:?}", e);
                return;
            }
        };

        if is_buy {
            read.mouse_entry_buy.clone()
        } else {
            read.mouse_entry_sell.clone()
        }
    };

    // 新規注文のマウス操作
    let mouse_c = mouse::Mouse::default();
    mouse_c.order(&entry_mouse);

    process::unlock(logic_setting.clone(), None);
}

fn entry_and_exit(is_buy: bool, logic_setting: Arc<RwLock<invoke::gui::Data>>) {
    process::lock(logic_setting.clone());

    let (readed_setting, readed_entry_mouse, readed_exit_mouse) = {
        let read = match logic_setting.read() {
            Ok(setting) => setting,
            Err(e) => {
                warn!("failed to read setting: {:?}", e);
                return;
            }
        };

        let readed_setting = read.setting.clone();
        let entry = if is_buy {
            read.mouse_entry_buy.clone()
        } else {
            read.mouse_entry_sell.clone()
        };
        let readed_exit_mouse = read.mouse_exit.clone();
        (readed_setting, entry, readed_exit_mouse)
    };

    // 新規注文のマウス操作
    let mouse_c = mouse::Mouse::default();
    mouse_c.order(&readed_entry_mouse);

    // 設定値待機する
    let target_sleep_ms = readed_setting.get_sleep_ms();
    utils::sleep(0, target_sleep_ms);

    // 決済注文のマウス操作
    let n = readed_exit_mouse.n;
    for _ in 0..n {
        mouse_c.order(&readed_exit_mouse);
        utils::sleep(1, 0);
    }

    process::unlock(logic_setting.clone(), None);
}

fn exit(_is_buy: bool, logic_setting: Arc<RwLock<invoke::gui::Data>>) {
    process::lock(logic_setting.clone());

    let exit_mouse = {
        let read = match logic_setting.read() {
            Ok(setting) => setting,
            Err(e) => {
                warn!("failed to read setting: {:?}", e);
                return;
            }
        };

        read.mouse_exit.clone()
    };

    // 新規注文のマウス操作
    let mouse_c = mouse::Mouse::default();
    mouse_c.order(&exit_mouse);

    process::unlock(logic_setting.clone(), None);
}

#[cfg(test)]
mod test {
    use crate::middleware::ticker::Ticker;

    use super::*;
    use std::env;

    #[test]
    fn test_process_none_flag() {
        env::set_var("RUST_LOG", "info");
        let _ = env_logger::builder().is_test(true).try_init();

        // モック: TickerStatsにflag=0 (None) のティッカーを追加
        let mut tickers = TickerStats::default();
        tickers.push(Ticker {
            flag: Some(0),
            ..Default::default()
        });

        let logic_setting = Arc::new(RwLock::new(invoke::gui::Data::default()));
        process(logic_setting, &tickers);
        // ログ出力やpanicしないことを確認
    }

    #[test]
    fn test_process_entry_buy_flag() {
        env::set_var("RUST_LOG", "info");
        let _ = env_logger::builder().is_test(true).try_init();

        let mut tickers = TickerStats::default();
        tickers.push(Ticker {
            flag: Some(1),
            ..Default::default()
        });

        let logic_setting = Arc::new(RwLock::new(invoke::gui::Data::default()));
        {
            let mut write = logic_setting.write().unwrap();
            write.mouse_entry_buy = invoke::gui::Mouse {
                n: 1,
                start_x: 0,
                start_y: 0,
                end_x: 10,
                end_y: 10,
            };
        }

        process(logic_setting, &tickers);
    }

    #[test]
    fn test_process_entry_sell_flag() {
        env::set_var("RUST_LOG", "info");
        let _ = env_logger::builder().is_test(true).try_init();

        let mut tickers = TickerStats::default();
        tickers.push(Ticker {
            flag: Some(2),
            ..Default::default()
        });

        let logic_setting = Arc::new(RwLock::new(invoke::gui::Data::default()));
        process(logic_setting, &tickers);
    }

    #[test]
    fn test_process_entry_buy_exit_flag() {
        env::set_var("RUST_LOG", "info");
        let _ = env_logger::builder().is_test(true).try_init();

        let mut tickers = TickerStats::default();
        tickers.push(Ticker {
            flag: Some(3),
            ..Default::default()
        });

        let logic_setting = Arc::new(RwLock::new(invoke::gui::Data::default()));
        process(logic_setting, &tickers);
    }

    #[test]
    fn test_process_entry_sell_exit_flag() {
        env::set_var("RUST_LOG", "info");
        let _ = env_logger::builder().is_test(true).try_init();

        let mut tickers = TickerStats::default();
        tickers.push(Ticker {
            flag: Some(4),
            ..Default::default()
        });

        let logic_setting = Arc::new(RwLock::new(invoke::gui::Data::default()));
        process(logic_setting, &tickers);
    }

    #[test]
    fn test_process_exit_buy_flag() {
        env::set_var("RUST_LOG", "info");
        let _ = env_logger::builder().is_test(true).try_init();

        let mut tickers = TickerStats::default();
        tickers.push(Ticker {
            flag: Some(5),
            ..Default::default()
        });

        let logic_setting = Arc::new(RwLock::new(invoke::gui::Data::default()));
        process(logic_setting, &tickers);
    }

    #[test]
    fn test_process_exit_sell_flag() {
        env::set_var("RUST_LOG", "info");
        let _ = env_logger::builder().is_test(true).try_init();

        let mut tickers = TickerStats::default();
        tickers.push(Ticker {
            flag: Some(6),
            ..Default::default()
        });

        let logic_setting = Arc::new(RwLock::new(invoke::gui::Data::default()));
        process(logic_setting, &tickers);
    }

    // テスト用のヘルパー関数をTickerStatsに実装する必要があります
    // 例:
    // impl TickerStats {
    //     pub fn push(&mut self, flag: u8) {
    //         self.tickers.push(Ticker {
    //             flag,
    //             // 他のフィールドも適宜初期化
    //             ..Default::default()
    //         });
    //     }
    // }
    fn test_process_no_ticker() {
        env::set_var("RUST_LOG", "info");
        let _ = env_logger::builder().is_test(true).try_init();

        let tickers = TickerStats::default();
        let logic_setting = Arc::new(RwLock::new(invoke::gui::Data::default()));
        process(logic_setting, &tickers);
        // no panic, logs "no ticker data"
    }
}
