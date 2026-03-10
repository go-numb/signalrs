use std::sync::{Arc, RwLock};

use crate::invoke::{self, gui::Order};

pub fn lock(s: Arc<RwLock<invoke::gui::Data>>) {
    match s.write() {
        Ok(mut rw) => rw.status.processing(),
        Err(e) => log::error!("Lock poisoned in lock(): {:?}", e),
    }
}

pub fn unlock(s: Arc<RwLock<invoke::gui::Data>>, order: Option<Order>) {
    match s.write() {
        Ok(mut rw) => {
            rw.status.message = if let Some(order) = order {
                let msg = format!("{:?}", order);
                rw.status.push(order);
                rw.status.shrink(crate::consts::DEFAULT_ORDER_HISTORY_LIMIT);
                msg
            } else {
                "undefined".to_string()
            };
            rw.status.processed();
        }
        Err(e) => log::error!("Lock poisoned in unlock(): {:?}", e),
    }
}
