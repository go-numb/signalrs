use std::sync::{Arc, RwLock};

use crate::invoke::{self, gui::Order};

pub fn lock(s: Arc<RwLock<invoke::gui::Data>>) {
    let mut rw = s.write().unwrap();
    rw.status.processing();
}

pub fn unlock(s: Arc<RwLock<invoke::gui::Data>>, order: Option<Order>) {
    let mut rw = s.write().unwrap();

    rw.status.message = if let Some(order) = order {
        rw.status.push(order.clone());
        rw.status.shrink(8);

        format!("{:?}", order)
    } else {
        "undefined".to_string()
    };

    rw.status.processed();
}
