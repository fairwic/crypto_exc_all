#![cfg(all(feature = "okx", feature = "binance"))]

use crypto_exc_all::{
    ExchangeId, PrivateAccountStreamChange, PrivateAccountStreamFrame,
    parse_private_account_stream_frame,
};
use serde_json::json;

#[test]
fn okx_account_positions_and_orders_map_provider_identities() {
    let account = parse_private_account_stream_frame(
        ExchangeId::Okx,
        json!({
            "arg": {"channel": "account"},
            "data": [{"uTime": "1000", "details": [
                {"ccy": "USDT", "uTime": "1001", "eq": "10", "availEq": "9"}
            ]}]
        }),
    )
    .expect("account frame");
    let PrivateAccountStreamFrame::Records(account) = account else {
        panic!("expected records");
    };
    assert_eq!(account[0].event_identity, "account:USDT:1001");
    let PrivateAccountStreamChange::Balance(balance) = &account[0].change else {
        panic!("expected balance change");
    };
    assert_eq!(balance.total, "10");
    assert_eq!(balance.available.as_deref(), Some("9"));

    let positions = parse_private_account_stream_frame(
        ExchangeId::Okx,
        json!({
            "arg": {"channel": "positions"},
            "data": [{"posId":"7", "instId":"BTC-USDT-SWAP", "posSide":"net",
                       "uTime":"1010", "pTime":"1011", "pos":"1"}]
        }),
    )
    .expect("positions frame");
    let PrivateAccountStreamFrame::Records(positions) = positions else {
        panic!("expected records");
    };
    assert_eq!(positions[0].event_identity, "position:7:1010:1011");

    let orders = parse_private_account_stream_frame(
        ExchangeId::Okx,
        json!({
            "arg": {"channel": "orders"},
            "data": [{"instId":"BTC-USDT-SWAP", "ordId":"42", "tradeId":"9",
                      "uTime":"1020", "state":"filled"}]
        }),
    )
    .expect("orders frame");
    let PrivateAccountStreamFrame::Records(orders) = orders else {
        panic!("expected records");
    };
    assert_eq!(orders[0].event_identity, "order:BTC-USDT-SWAP:42:9:1020");
}

#[test]
fn binance_typed_user_events_map_records_and_expiry() {
    let update = parse_private_account_stream_frame(
        ExchangeId::Binance,
        json!({
            "e":"ACCOUNT_UPDATE", "E":2001_u64, "T":2000_u64,
            "a": {"m":"ORDER",
                "B":[{"a":"USDT","wb":"10","cw":"9","bc":"1"}],
                "P":[{"s":"BTCUSDT","pa":"1","ep":"100","bep":"100","cr":"0",
                       "up":"1","mt":"cross","iw":"0","ps":"BOTH"}]}
        }),
    )
    .expect("account update");
    let PrivateAccountStreamFrame::Records(records) = update else {
        panic!("expected records");
    };
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].event_identity, "balance:USDT:2000:ORDER");
    assert!(matches!(
        records[1].change,
        PrivateAccountStreamChange::Position(_)
    ));
    let PrivateAccountStreamChange::Balance(balance) = &records[0].change else {
        panic!("expected balance change");
    };
    assert_eq!(balance.available, None);

    let expired = parse_private_account_stream_frame(
        ExchangeId::Binance,
        json!({"e":"listenKeyExpired", "E":3000_u64, "listenKey":"masked"}),
    )
    .expect("expiry");
    assert_eq!(
        expired,
        PrivateAccountStreamFrame::Expired {
            exchange: ExchangeId::Binance,
            provider_event_time_ms: 3000
        }
    );
}

#[test]
fn binance_account_update_accepts_balance_only_provider_frames() {
    let update = parse_private_account_stream_frame(
        ExchangeId::Binance,
        json!({
            "e":"ACCOUNT_UPDATE", "E":2101_u64, "T":2100_u64,
            "a": {"m":"FUNDING_FEE",
                "B":[{"a":"USDT","wb":"10","cw":"9","bc":"1"}]}
        }),
    )
    .expect("balance-only account update");

    let PrivateAccountStreamFrame::Records(records) = update else {
        panic!("expected records");
    };
    assert_eq!(records.len(), 1);
    assert!(matches!(
        records[0].change,
        PrivateAccountStreamChange::Balance(_)
    ));
}

#[test]
fn unknown_private_events_fail_closed() {
    assert!(
        parse_private_account_stream_frame(
            ExchangeId::Binance,
            json!({"e":"ACCOUNT_CONFIG_UPDATE", "E":1_u64, "T":1_u64,
               "ac":{"s":"BTCUSDT","l":5}}),
        )
        .is_err()
    );
    assert!(
        parse_private_account_stream_frame(
            ExchangeId::Okx,
            json!({"arg":{"channel":"balance_and_position"},"data":[]}),
        )
        .is_err()
    );
}
