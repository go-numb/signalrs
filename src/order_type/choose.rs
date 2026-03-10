use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, RwLock};

use crate::{
    invoke::gui::{Data, OrderType},
    middleware::ticker::TickerStats,
    order_type::{entry, exit, origin, simple},
};

use log::{trace, warn};

pub struct OrderRequest {
    pub order_type: OrderType,
    pub setting: Arc<RwLock<Data>>,
    pub tickers: TickerStats,
}

pub struct OrderDispatcher {
    tx: SyncSender<OrderRequest>,
}

impl OrderDispatcher {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::sync_channel::<OrderRequest>(1);

        std::thread::spawn(move || {
            for request in rx {
                match request.order_type {
                    OrderType::Simple => {
                        simple::process(request.setting, &request.tickers);
                    }
                    OrderType::BuyEntry | OrderType::SellEntry => {
                        entry::process(request.order_type, request.setting, &request.tickers);
                    }
                    OrderType::ExitOnly => {
                        exit::process(request.setting, &request.tickers);
                    }
                    OrderType::Custom => {
                        origin::process(request.setting, &request.tickers);
                    }
                }
            }
        });

        OrderDispatcher { tx }
    }

    pub fn dispatch(&self, logic_setting: Arc<RwLock<Data>>, tickers: &TickerStats) {
        // Check preconditions before sending
        let order_type = {
            let read_setting = match logic_setting.read() {
                Ok(setting) => setting,
                Err(e) => {
                    warn!("failed to read setting: {:?}", e);
                    return;
                }
            };

            if !read_setting.status.is_running {
                trace!("is not running");
                return;
            }

            if read_setting.status.is_processing {
                trace!("is processing");
                return;
            }

            read_setting.setting.order_type
        };

        let request = OrderRequest {
            order_type,
            setting: logic_setting,
            tickers: tickers.clone(),
        };

        // try_send: if worker is busy, skip this tick (backpressure)
        if let Err(_) = self.tx.try_send(request) {
            trace!("worker busy, skipping tick");
        }
    }
}
