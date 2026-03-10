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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_lock_sets_processing() {
        let data = invoke::gui::Data::default();
        let s = Arc::new(RwLock::new(data));

        lock(s.clone());

        let read = s.read().unwrap();
        assert!(read.status.is_processing);
    }

    #[test]
    fn test_unlock_clears_processing() {
        let data = invoke::gui::Data::default();
        let s = Arc::new(RwLock::new(data));

        lock(s.clone());
        unlock(s.clone(), None);

        let read = s.read().unwrap();
        assert!(!read.status.is_processing);
        assert_eq!(read.status.message, "undefined");
    }

    #[test]
    fn test_unlock_with_order() {
        let data = invoke::gui::Data::default();
        let s = Arc::new(RwLock::new(data));

        let order = Order::new(Decimal::new(100, 0));
        unlock(s.clone(), Some(order));

        let read = s.read().unwrap();
        assert!(!read.status.is_processing);
        assert_eq!(read.status.orders.len(), 1);
        assert_eq!(read.status.orders[0].entry, Decimal::new(100, 0));
    }

    #[test]
    fn test_unlock_shrinks_orders() {
        let data = invoke::gui::Data::default();
        let s = Arc::new(RwLock::new(data));

        // Add more than DEFAULT_ORDER_HISTORY_LIMIT orders
        for i in 0..12 {
            let order = Order::new(Decimal::new(i, 0));
            unlock(s.clone(), Some(order));
        }

        let read = s.read().unwrap();
        assert_eq!(read.status.orders.len(), crate::consts::DEFAULT_ORDER_HISTORY_LIMIT);
    }
}
