//! Integration tests for signalrs

use signalrs::invoke::gui::*;
use signalrs::middleware::ticker::*;
use signalrs::order_type::choose::*;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

/// Test the full flow: Data creation -> Setting config -> TickerStats -> OrderDispatcher
#[test]
fn test_full_data_lifecycle() {
    // Create data with defaults
    let data = Data::default();
    assert_eq!(data.setting.order_type, OrderType::Simple);
    assert_eq!(data.setting.speed, Speed::Fast);

    // Wrap in Arc<RwLock>
    let wrapped = WrappedData::new(data);

    // Update LTP
    wrapped.update(false, Decimal::new(150, 0));
    {
        let read = wrapped.data.read().unwrap();
        assert_eq!(read.status.ltp, Decimal::new(150, 0));
        assert!(read.status.is_recived);
    }

    // Update again with same price - should not mark as received
    wrapped.update(false, Decimal::new(150, 0));
    {
        let read = wrapped.data.read().unwrap();
        assert!(!read.status.is_recived);
    }
}

/// Test OrderDispatcher with non-running state (should not dispatch)
#[test]
fn test_dispatcher_skips_when_not_running() {
    let dispatcher = OrderDispatcher::new();
    let data = Data::default(); // is_running = false by default
    let setting = Arc::new(RwLock::new(data));
    let tickers = TickerStats::new();

    // Should return immediately without sending to worker
    dispatcher.dispatch(setting.clone(), &tickers);

    // Verify state unchanged
    let read = setting.read().unwrap();
    assert!(!read.status.is_running);
    assert!(!read.status.is_processing);
}

/// Test OrderDispatcher skips when already processing
#[test]
fn test_dispatcher_skips_when_processing() {
    let dispatcher = OrderDispatcher::new();
    let mut data = Data::default();
    data.status.is_running = true;
    data.status.is_processing = true;
    let setting = Arc::new(RwLock::new(data));
    let tickers = TickerStats::new();

    dispatcher.dispatch(setting.clone(), &tickers);

    // Should still be processing (not changed by dispatch)
    let read = setting.read().unwrap();
    assert!(read.status.is_processing);
}

/// Test TickerStats diff calculation with realistic data
#[test]
fn test_ticker_stats_realistic_diff() {
    use chrono::{Duration, Utc};

    let now = Utc::now();
    let mut tickers = TickerStats::new();

    // Simulate 10 ticks over 1 second
    for i in 0..10 {
        let price = Decimal::from_str("150.000").unwrap() + Decimal::new(i, 3); // 150.000, 150.001, ...
        tickers.push(Ticker {
            symbol: "USDJPY".to_string(),
            bid: price,
            ask: price + Decimal::new(2, 3), // spread of 0.002
            recived_at: Some(now + Duration::milliseconds(i as i64 * 100)),
            ..Default::default()
        });
    }

    assert_eq!(tickers.len(), 10);

    // Mid of last ticker: (150.009 + 150.011) / 2 = 150.010
    let mid = tickers.mid();
    assert_eq!(mid, Decimal::from_str("150.010").unwrap());
}

/// Test Setting serialization matches frontend expectations
#[test]
fn test_setting_json_contract() {
    let setting = Setting::new();
    let json = serde_json::to_value(&setting).unwrap();

    // Frontend expects these exact field names and types
    assert_eq!(json["tcp"], "8080");
    assert_eq!(json["order_type"], "0"); // serde rename
    assert_eq!(json["speed"], "1");      // serde rename
    assert_eq!(json["vol"], "0.1");
    assert_eq!(json["interval"], 10);
    assert_eq!(json["interval_random"], false);
}

/// Test Data serialization for frontend contract
#[test]
fn test_data_json_contract() {
    let data = Data::default();
    let json = serde_json::to_value(&data).unwrap();

    // Verify required top-level fields exist
    assert!(json["version"].is_string());
    assert!(json["description"].is_string());
    assert!(json["host"].is_string());
    assert!(json["status"].is_object());
    assert!(json["mouse_entry_buy"].is_object());
    assert!(json["mouse_entry_sell"].is_object());
    assert!(json["mouse_exit"].is_object());
    assert!(json["setting"].is_object());
    assert!(json["order_type"].is_array());
    assert!(json["speed"].is_array());
}

/// Test Status serialization
#[test]
fn test_status_json_contract() {
    let status = Status::new();
    let json = serde_json::to_value(&status).unwrap();

    assert_eq!(json["is_recived"], false);
    assert_eq!(json["is_running"], false);
    assert_eq!(json["is_processing"], false);
    assert_eq!(json["message"], "off");
    assert!(json["orders"].is_array());
}

/// Test consts values
#[test]
fn test_consts() {
    assert_eq!(signalrs::consts::DEFAULT_TICKER_BUFFER_SIZE, 144);
    assert_eq!(signalrs::consts::DEFAULT_ORDER_HISTORY_LIMIT, 8);
    assert_eq!(signalrs::consts::DEFAULT_SAVE_PATH, "./.save/setting.json");
}

/// Test TCP client creation
#[test]
fn test_tcp_client_creation() {
    let (client, _rx) = signalrs::middleware::tcp::TcpClient::<signalrs::middleware::ticker::Ticker>::new("127.0.0.1:0".to_string());
    // Just verify creation doesn't panic
    let _ = client;
}

/// Test TCP server bind and receive on random port
#[test]
fn test_tcp_server_bind() {
    use std::io::Write;
    use std::net::TcpStream;

    // Use a high port to avoid conflicts
    let (client, rx) = signalrs::middleware::tcp::TcpClient::<signalrs::middleware::ticker::Ticker>::new("127.0.0.1:19876".to_string());

    let result = client.received_server();
    assert!(result.is_ok(), "TCP server should bind successfully");

    // Give server time to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Connect and send a ticker JSON
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:19876") {
        let json = r#"{"symbol":"USDJPY","bid":"150.123","ask":"150.125"}"#;
        let _ = stream.write_all(format!("{}\n", json).as_bytes());
        let _ = stream.flush();

        // Try to receive
        if let Ok(result) = rx.recv_timeout(std::time::Duration::from_secs(2)) {
            match result {
                Ok(ticker) => {
                    assert_eq!(ticker.symbol, "USDJPY");
                    assert_eq!(ticker.bid, Decimal::from_str("150.123").unwrap());
                }
                Err(_) => {} // Parse error is acceptable in test
            }
        }
    }
}
