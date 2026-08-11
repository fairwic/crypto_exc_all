#![cfg(all(feature = "okx", feature = "binance"))]

use crypto_exc_all::{
    ExchangeId, PrivateAccountStreamChange, PrivateAccountStreamFrame, PrivateOrderStreamKind,
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
    assert_eq!(account[0].event_identity, "account:USDT:1000:1001");
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
                      "uTime":"1020", "state":"filled", "accFillSz":"0.5",
                      "avgPx":"2500"}]
        }),
    )
    .expect("orders frame");
    let PrivateAccountStreamFrame::Records(orders) = orders else {
        panic!("expected records");
    };
    assert_eq!(orders[0].event_identity, "order:BTC-USDT-SWAP:42:9:1020");
    let PrivateAccountStreamChange::Order(order) = &orders[0].change else {
        panic!("expected order change");
    };
    assert_eq!(order.kind, PrivateOrderStreamKind::Regular);
    assert_eq!(order.filled_size.as_deref(), Some("0.5"));
    assert_eq!(order.average_fill_price.as_deref(), Some("2500"));
}

#[test]
fn okx_protection_algo_only_emits_terminal_parent_facts() {
    let canceled = parse_private_account_stream_frame(
        ExchangeId::Okx,
        json!({
            "arg": {"channel": "orders-algo"},
            "data": [{
                "instId":"ETH-USDT-SWAP", "algoId":"9001",
                "algoClOrdId":"rq0123456789abcdef0123456789abcd", "state":"canceled",
                "ordType":"conditional", "side":"sell", "sz":"0.01",
                "actualPx":"", "triggerTime":"", "cTime":"1000", "pTime":"1010"
            }]
        }),
    )
    .expect("canceled protection algo");
    let PrivateAccountStreamFrame::Records(records) = canceled else {
        panic!("expected protection record");
    };
    assert_eq!(
        records[0].event_identity,
        "protection-algo:ETH-USDT-SWAP:9001:canceled:1010"
    );
    assert_eq!(records[0].provider_transaction_time_ms, None);
    let PrivateAccountStreamChange::Order(order) = &records[0].change else {
        panic!("expected protection order");
    };
    assert_eq!(order.kind, PrivateOrderStreamKind::ProtectionAlgo);
    assert_eq!(order.order_id, "9001");
    assert_eq!(order.filled_size.as_deref(), Some("0"));
    assert_eq!(order.source_updated_at_ms, 1010);

    assert_eq!(
        parse_private_account_stream_frame(
            ExchangeId::Okx,
            json!({
                "arg": {"channel": "orders-algo"},
                "data": [{
                    "instId":"ETH-USDT-SWAP", "algoId":"9001", "state":"effective",
                    "triggerTime":"1020", "pTime":"1021"
                }]
            }),
        )
        .expect("triggered parent is not a fill"),
        PrivateAccountStreamFrame::Control {
            exchange: ExchangeId::Okx,
            event: "orders-algo-nonterminal".to_owned(),
        }
    );
}

#[test]
fn okx_protection_child_preserves_parent_and_child_identities() {
    let frame = parse_private_account_stream_frame(
        ExchangeId::Okx,
        json!({
            "arg": {"channel": "orders"},
            "data": [{
                "instId":"ETH-USDT-SWAP", "ordId":"child-7001", "clOrdId":"",
                "algoId":"9001", "algoClOrdId":"rq0123456789abcdef0123456789abcd",
                "tradeId":"77", "uTime":"1030", "state":"filled",
                "side":"sell", "ordType":"market", "sz":"0.01",
                "accFillSz":"0.01", "avgPx":"2500"
            }]
        }),
    )
    .expect("protection child order");
    let PrivateAccountStreamFrame::Records(records) = frame else {
        panic!("expected child order record");
    };
    let PrivateAccountStreamChange::Order(order) = &records[0].change else {
        panic!("expected child order");
    };

    assert_eq!(
        records[0].event_identity,
        "order:ETH-USDT-SWAP:child-7001:77:1030"
    );
    assert_eq!(order.kind, PrivateOrderStreamKind::ProtectionAlgoChild);
    assert_eq!(order.order_id, "child-7001");
    assert_eq!(order.client_order_id, None);
    assert_eq!(order.parent_order_id.as_deref(), Some("9001"));
    assert_eq!(
        order.parent_client_order_id.as_deref(),
        Some("rq0123456789abcdef0123456789abcd")
    );
    assert_eq!(order.filled_size.as_deref(), Some("0.01"));
    assert_eq!(order.average_fill_price.as_deref(), Some("2500"));

    assert!(
        parse_private_account_stream_frame(
            ExchangeId::Okx,
            json!({
                "arg": {"channel": "orders"},
                "data": [{
                    "instId":"ETH-USDT-SWAP", "ordId":"child-7002",
                    "algoClOrdId":"rq0123456789abcdef0123456789abcd",
                    "uTime":"1031", "state":"filled",
                    "accFillSz":"0.01", "avgPx":"2500"
                }]
            }),
        )
        .is_err()
    );
}

#[test]
fn okx_accepts_connection_count_control_but_rejects_limit_error() {
    let count = parse_private_account_stream_frame(
        ExchangeId::Okx,
        json!({
            "event": "channel-conn-count",
            "channel": "orders",
            "connCount": "1",
            "connId": "connection-id"
        }),
    )
    .expect("connection count control");

    assert_eq!(
        count,
        PrivateAccountStreamFrame::Control {
            exchange: ExchangeId::Okx,
            event: "channel-conn-count".to_owned(),
        }
    );
    assert!(
        parse_private_account_stream_frame(
            ExchangeId::Okx,
            json!({
                "event": "channel-conn-count-error",
                "channel": "orders",
                "connCount": "30",
                "connId": "connection-id"
            }),
        )
        .is_err()
    );
}

#[test]
fn okx_account_updates_use_frame_time_for_event_identity() {
    let first = parse_private_account_stream_frame(
        ExchangeId::Okx,
        json!({
            "arg": {"channel": "account"},
            "data": [{"uTime": "2000", "details": [
                {"ccy": "USDT", "uTime": "1000", "eq": "10", "availEq": "9"}
            ]}]
        }),
    )
    .expect("first account update");
    let second = parse_private_account_stream_frame(
        ExchangeId::Okx,
        json!({
            "arg": {"channel": "account"},
            "data": [{"uTime": "2005", "details": [
                {"ccy": "USDT", "uTime": "1000", "eq": "11", "availEq": "10"}
            ]}]
        }),
    )
    .expect("second account update");
    let PrivateAccountStreamFrame::Records(first) = first else {
        panic!("expected first account records");
    };
    let PrivateAccountStreamFrame::Records(second) = second else {
        panic!("expected second account records");
    };

    assert_eq!(first[0].provider_event_time_ms, 2000);
    assert_eq!(second[0].provider_event_time_ms, 2005);
    assert_ne!(first[0].event_identity, second[0].event_identity);
    let PrivateAccountStreamChange::Balance(first_balance) = &first[0].change else {
        panic!("expected balance change");
    };
    assert_eq!(first_balance.source_updated_at_ms, 1000);
}

#[test]
fn binance_typed_user_events_map_records_and_expiry() {
    let order = parse_private_account_stream_frame(
        ExchangeId::Binance,
        json!({
            "e":"ORDER_TRADE_UPDATE", "E":1999_u64, "T":1998_u64,
            "o": {"s":"BTCUSDT", "c":"TEST", "S":"BUY", "o":"MARKET",
                  "x":"TRADE", "X":"PARTIALLY_FILLED", "i":42_u64,
                  "q":"1", "p":"0", "z":"0.5", "ap":"2500", "t":7_u64,
                  "O":1990_u64}
        }),
    )
    .expect("order update");
    let PrivateAccountStreamFrame::Records(order_records) = order else {
        panic!("expected order records");
    };
    let PrivateAccountStreamChange::Order(order) = &order_records[0].change else {
        panic!("expected order change");
    };
    assert_eq!(order.filled_size.as_deref(), Some("0.5"));
    assert_eq!(order.average_fill_price.as_deref(), Some("2500"));

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
