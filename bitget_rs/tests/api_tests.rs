use bitget_rs::api::account::{AccountBillRequest, BitgetAccount};
use bitget_rs::api::announcements::{AnnouncementListRequest, BitgetAnnouncements};
use bitget_rs::api::asset::{
    BitgetAsset, DepositAddressRequest, TransferRequest, WalletHistoryRequest, WithdrawRequest,
};
use bitget_rs::api::market::{BitgetMarket, TickerRequest};
use bitget_rs::api::trade::{
    BitgetTrade, CancelAllOrdersRequest, CancelOrderRequest, ClosePositionsRequest,
    ModifyOrderRequest, NewOrderRequest, OrderQueryRequest,
};
use bitget_rs::api::websocket::{
    BitgetWebsocket, BitgetWebsocketCancelOrderParams, BitgetWebsocketChannel,
    BitgetWebsocketEvent, BitgetWebsocketManager, BitgetWebsocketPlaceOrderParams, ConnectionState,
    ReconnectConfig,
};
use bitget_rs::client::BitgetClient;
use bitget_rs::config::{Config, Credentials};
use bitget_rs::utils::generate_signature;
use futures_util::{SinkExt, StreamExt};
use mockito::{Matcher, Server};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

fn public_client(server_url: String) -> BitgetClient {
    BitgetClient::with_config(None, Config::default().with_api_url_for_test(server_url)).unwrap()
}

fn signed_client(server_url: String) -> BitgetClient {
    let mut client = BitgetClient::with_config(
        Some(Credentials::new("test-key", "test-secret", "test-pass")),
        Config::default().with_api_url_for_test(server_url),
    )
    .unwrap();
    client.set_timestamp_provider(|| 1_684_814_440_729);
    client
}

trait TestConfigExt {
    fn with_api_url_for_test(self, api_url: String) -> Self;
}

impl TestConfigExt for Config {
    fn with_api_url_for_test(mut self, api_url: String) -> Self {
        self.api_url = api_url;
        self
    }
}

#[tokio::test]
async fn market_get_ticker_maps_bitget_v2_endpoint() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock(
            "GET",
            "/api/v2/mix/market/ticker?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"00000",
                "msg":"success",
                "requestTime":1695794095685,
                "data":[{
                    "symbol":"BTCUSDT",
                    "lastPr":"70000.1",
                    "askPr":"70000.2",
                    "bidPr":"69999.9",
                    "baseVolume":"12.3",
                    "quoteVolume":"861000",
                    "ts":"1695794098184"
                }]
            }"#,
        )
        .create_async()
        .await;

    let market = BitgetMarket::new(public_client(server.url()));
    let tickers = market
        .get_ticker(TickerRequest::new("BTCUSDT", "USDT-FUTURES"))
        .await
        .unwrap();

    assert_eq!(tickers[0].symbol, "BTCUSDT");
    assert_eq!(tickers[0].last_price, "70000.1");
    assert_eq!(tickers[0].bid_price, "69999.9");
    mock.assert_async().await;
}

#[test]
fn websocket_builds_v2_urls_login_and_subscription_payloads() {
    let websocket = BitgetWebsocket::new(
        Credentials::new("test-key", "test-secret", "test-pass"),
        Config::default(),
    )
    .unwrap();
    let ticker = BitgetWebsocketChannel::new("USDT-FUTURES", "ticker").with_inst_id("BTCUSDT");
    let account = BitgetWebsocketChannel::new("USDT-FUTURES", "account").with_coin("default");

    assert_eq!(websocket.public_url(), "wss://ws.bitget.com/v2/ws/public");
    assert_eq!(websocket.private_url(), "wss://ws.bitget.com/v2/ws/private");

    let login = websocket.login_request_at(1_684_814_440_729).unwrap();
    assert_eq!(login["op"], "login");
    assert_eq!(login["args"][0]["apiKey"], "test-key");
    assert_eq!(login["args"][0]["passphrase"], "test-pass");
    assert_eq!(login["args"][0]["timestamp"], "1684814440729");
    assert_eq!(
        login["args"][0]["sign"],
        "rfrIzESBgZTPxHAmChBrefCd1WNDxD4qIr2vsTlCHFc="
    );

    let subscribe = BitgetWebsocket::subscribe_request(&[ticker.clone(), account.clone()]);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&subscribe).unwrap(),
        json!({
            "op": "subscribe",
            "args": [
                {"instType":"USDT-FUTURES","channel":"ticker","instId":"BTCUSDT"},
                {"instType":"USDT-FUTURES","channel":"account","coin":"default"}
            ]
        })
    );

    let unsubscribe = BitgetWebsocket::unsubscribe_request(&[ticker]);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unsubscribe).unwrap(),
        json!({
            "op": "unsubscribe",
            "args": [
                {"instType":"USDT-FUTURES","channel":"ticker","instId":"BTCUSDT"}
            ]
        })
    );
}

#[test]
fn websocket_builds_trade_requests_and_parses_trade_ack() {
    let place_params =
        BitgetWebsocketPlaceOrderParams::limit("buy", "2", "501", "USDT", "crossed", "gtc")
            .with_client_order_id("client-123")
            .with_trade_side("open")
            .with_reduce_only("NO")
            .with_stp_mode("cancel_taker");

    let place = BitgetWebsocket::place_order_request(
        "NEWclient-123",
        "USDT-FUTURES",
        "BTCUSDT",
        place_params,
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&place).unwrap(),
        json!({
            "op":"trade",
            "args":[{
                "channel":"place-order",
                "id":"NEWclient-123",
                "instId":"BTCUSDT",
                "instType":"USDT-FUTURES",
                "params":{
                    "orderType":"limit",
                    "side":"buy",
                    "size":"2",
                    "force":"gtc",
                    "price":"501",
                    "marginCoin":"USDT",
                    "marginMode":"crossed",
                    "clientOid":"client-123",
                    "tradeSide":"open",
                    "reduceOnly":"NO",
                    "stpMode":"cancel_taker"
                }
            }]
        })
    );

    let cancel_params = BitgetWebsocketCancelOrderParams::new()
        .with_order_id("1234567890")
        .with_client_order_id("client-123");
    let cancel = BitgetWebsocket::cancel_order_request(
        "CANCELclient-123",
        "USDT-FUTURES",
        "BTCUSDT",
        cancel_params,
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&cancel).unwrap(),
        json!({
            "op":"trade",
            "args":[{
                "channel":"cancel-order",
                "id":"CANCELclient-123",
                "instId":"BTCUSDT",
                "instType":"USDT-FUTURES",
                "params":{
                    "orderId":"1234567890",
                    "clientOid":"client-123"
                }
            }]
        })
    );

    let ack = BitgetWebsocketEvent::parse(
        r#"{
            "event":"trade",
            "arg":[{
                "id":"NEWclient-123",
                "instType":"USDT-FUTURES",
                "channel":"place-order",
                "instId":"BTCUSDT",
                "params":{
                    "orderId":"1234567890",
                    "clientOid":"client-123"
                }
            }],
            "code":0,
            "msg":"Success"
        }"#,
    )
    .unwrap();

    match ack {
        BitgetWebsocketEvent::Trade {
            code, msg, args, ..
        } => {
            assert_eq!(code.as_deref(), Some("0"));
            assert_eq!(msg.as_deref(), Some("Success"));
            assert_eq!(args[0].id.as_deref(), Some("NEWclient-123"));
            assert_eq!(args[0].channel, "place-order");
            assert_eq!(args[0].inst_id.as_deref(), Some("BTCUSDT"));
            let params = args[0].params.as_ref().unwrap();
            assert_eq!(params.order_id.as_deref(), Some("1234567890"));
            assert_eq!(params.client_order_id.as_deref(), Some("client-123"));
        }
        other => panic!("expected trade ack, got {other:?}"),
    }
}

#[test]
fn websocket_event_parser_maps_ticker_orders_account_and_positions() {
    let ticker = BitgetWebsocketEvent::parse(
        r#"{
            "action":"snapshot",
            "arg":{"instType":"USDT-FUTURES","channel":"ticker","instId":"BTCUSDT"},
            "data":[{
                "instId":"BTCUSDT",
                "lastPr":"87673.6",
                "bidPr":"87673.5",
                "askPr":"87673.7",
                "baseVolume":"17398.1612",
                "quoteVolume":"1521198076.61216",
                "markPrice":"87673.7",
                "indexPrice":"87714.0732915359034044",
                "fundingRate":"0.000055",
                "ts":"1766674540816"
            }],
            "ts":1766674540817
        }"#,
    )
    .unwrap();
    match ticker {
        BitgetWebsocketEvent::Ticker {
            action, arg, data, ..
        } => {
            assert_eq!(action.as_deref(), Some("snapshot"));
            assert_eq!(arg.channel, "ticker");
            assert_eq!(data[0].inst_id, Some("BTCUSDT".to_string()));
            assert_eq!(data[0].last_price.as_deref(), Some("87673.6"));
            assert_eq!(data[0].bid_price.as_deref(), Some("87673.5"));
            assert_eq!(data[0].ask_price.as_deref(), Some("87673.7"));
            assert_eq!(data[0].base_volume.as_deref(), Some("17398.1612"));
            assert_eq!(data[0].timestamp.as_deref(), Some("1766674540816"));
        }
        other => panic!("expected ticker event, got {other:?}"),
    }

    let orders = BitgetWebsocketEvent::parse(
        r#"{
            "action":"snapshot",
            "arg":{"instType":"USDT-FUTURES","channel":"orders","instId":"default"},
            "data":[{
                "orderId":"13333333333333333333",
                "clientOid":"12354678990111",
                "instId":"ETHUSDT",
                "side":"buy",
                "tradeSide":"open",
                "orderType":"limit",
                "price":"3000",
                "size":"0.4",
                "accBaseVolume":"0",
                "priceAvg":"0",
                "status":"live",
                "uTime":"1760461517274"
            }],
            "ts":1760461517285
        }"#,
    )
    .unwrap();
    match orders {
        BitgetWebsocketEvent::Orders { data, .. } => {
            assert_eq!(data[0].order_id.as_deref(), Some("13333333333333333333"));
            assert_eq!(data[0].client_order_id.as_deref(), Some("12354678990111"));
            assert_eq!(data[0].side.as_deref(), Some("buy"));
            assert_eq!(data[0].status.as_deref(), Some("live"));
            assert_eq!(data[0].filled_size.as_deref(), Some("0"));
        }
        other => panic!("expected orders event, got {other:?}"),
    }

    let account = BitgetWebsocketEvent::parse(
        r#"{
            "action":"snapshot",
            "arg":{"instType":"USDT-FUTURES","channel":"account","coin":"default"},
            "data":[{
                "marginCoin":"USDT",
                "frozen":"0.00000000",
                "available":"11.98545761",
                "equity":"11.98545761",
                "usdtEquity":"11.985457617660",
                "unrealizedPL":"0.000000000000",
                "assetsMode":"union"
            }],
            "ts":1695717225146
        }"#,
    )
    .unwrap();
    match account {
        BitgetWebsocketEvent::Account { data, .. } => {
            assert_eq!(data[0].margin_coin.as_deref(), Some("USDT"));
            assert_eq!(data[0].available.as_deref(), Some("11.98545761"));
            assert_eq!(data[0].frozen.as_deref(), Some("0.00000000"));
            assert_eq!(data[0].equity.as_deref(), Some("11.98545761"));
            assert_eq!(data[0].assets_mode.as_deref(), Some("union"));
        }
        other => panic!("expected account event, got {other:?}"),
    }

    let positions = BitgetWebsocketEvent::parse(
        r#"{
            "action":"snapshot",
            "arg":{"instType":"USDT-FUTURES","channel":"positions","instId":"default"},
            "data":[{
                "posId":"1",
                "instId":"ETHUSDT",
                "marginCoin":"USDT",
                "marginMode":"crossed",
                "holdSide":"short",
                "total":"0.1",
                "available":"0.1",
                "openPriceAvg":"1900",
                "leverage":20,
                "unrealizedPL":"0",
                "liquidationPrice":"5788.108475905242",
                "markPrice":"2500",
                "uTime":"1695711602568"
            }],
            "ts":1695717430441
        }"#,
    )
    .unwrap();
    match positions {
        BitgetWebsocketEvent::Positions { data, .. } => {
            assert_eq!(data[0].position_id.as_deref(), Some("1"));
            assert_eq!(data[0].inst_id.as_deref(), Some("ETHUSDT"));
            assert_eq!(data[0].hold_side.as_deref(), Some("short"));
            assert_eq!(data[0].total.as_deref(), Some("0.1"));
            assert_eq!(data[0].leverage.as_deref(), Some("20"));
            assert_eq!(data[0].mark_price.as_deref(), Some("2500"));
        }
        other => panic!("expected positions event, got {other:?}"),
    }
}

#[test]
fn websocket_event_parser_maps_orderbook_trades_and_candles() {
    let orderbook = BitgetWebsocketEvent::parse(
        r#"{
            "action":"snapshot",
            "arg":{"instType":"USDT-FUTURES","channel":"books5","instId":"BTCUSDT"},
            "data":[{
                "asks":[["27000.5","8.760"],["27001.0","0.400"]],
                "bids":[["27000.0","2.710"],["26999.5","1.460"]],
                "checksum":0,
                "seq":123,
                "ts":"1695716059516"
            }],
            "ts":1695716059516
        }"#,
    )
    .unwrap();
    match orderbook {
        BitgetWebsocketEvent::OrderBook {
            action, arg, data, ..
        } => {
            assert_eq!(action.as_deref(), Some("snapshot"));
            assert_eq!(arg.channel, "books5");
            assert_eq!(data[0].asks[0].price, "27000.5");
            assert_eq!(data[0].asks[0].size, "8.760");
            assert_eq!(data[0].bids[0].price, "27000.0");
            assert_eq!(data[0].checksum, Some(0));
            assert_eq!(data[0].sequence, Some(123));
            assert_eq!(data[0].timestamp.as_deref(), Some("1695716059516"));
        }
        other => panic!("expected orderbook event, got {other:?}"),
    }

    let trades = BitgetWebsocketEvent::parse(
        r#"{
            "action":"snapshot",
            "arg":{"instType":"USDT-FUTURES","channel":"trade","instId":"BTCUSDT"},
            "data":[
                {"ts":"1695716760565","price":"27000.5","size":"0.001","side":"buy","tradeId":"1111111111"},
                {"ts":"1695716759514","price":"27000.0","size":"0.001","side":"sell","tradeId":"1111111112"}
            ],
            "ts":1695716761589
        }"#,
    )
    .unwrap();
    match trades {
        BitgetWebsocketEvent::Trades { data, .. } => {
            assert_eq!(data[0].timestamp.as_deref(), Some("1695716760565"));
            assert_eq!(data[0].price.as_deref(), Some("27000.5"));
            assert_eq!(data[0].size.as_deref(), Some("0.001"));
            assert_eq!(data[0].side.as_deref(), Some("buy"));
            assert_eq!(data[0].trade_id.as_deref(), Some("1111111111"));
            assert_eq!(data[1].side.as_deref(), Some("sell"));
        }
        other => panic!("expected trades event, got {other:?}"),
    }

    let candles = BitgetWebsocketEvent::parse(
        r#"{
            "action":"snapshot",
            "arg":{"instType":"USDT-FUTURES","channel":"candle1m","instId":"BTCUSDT"},
            "data":[["1695685500000","27000","27000.5","27000","27000.5","0.057","1539.0155","1539.0155"]],
            "ts":1695715462250
        }"#,
    )
    .unwrap();
    match candles {
        BitgetWebsocketEvent::Candles {
            action, arg, data, ..
        } => {
            assert_eq!(action.as_deref(), Some("snapshot"));
            assert_eq!(arg.channel, "candle1m");
            assert_eq!(data[0].start_time, "1695685500000");
            assert_eq!(data[0].open, "27000");
            assert_eq!(data[0].high, "27000.5");
            assert_eq!(data[0].low, "27000");
            assert_eq!(data[0].close, "27000.5");
            assert_eq!(data[0].base_volume, "0.057");
            assert_eq!(data[0].quote_volume, "1539.0155");
            assert_eq!(data[0].usdt_volume, "1539.0155");
        }
        other => panic!("expected candles event, got {other:?}"),
    }
}

#[test]
fn websocket_event_parser_maps_private_fill_channel() {
    let fill = BitgetWebsocketEvent::parse(
        r#"{
            "action":"snapshot",
            "arg":{"instType":"USDT-FUTURES","channel":"fill","instId":"default"},
            "data":[{
                "orderId":"111",
                "clientOid":"client-111",
                "tradeId":"222",
                "symbol":"BTCUSDT",
                "side":"buy",
                "orderType":"market",
                "posMode":"one_way_mode",
                "price":"51000.5",
                "baseVolume":"0.01",
                "quoteVolume":"510.005",
                "profit":"0",
                "tradeSide":"open",
                "tradeScope":"taker",
                "feeDetail":[{
                    "feeCoin":"USDT",
                    "deduction":"no",
                    "totalDeductionFee":"0",
                    "totalFee":"-0.183717"
                }],
                "cTime":"1703577336606",
                "uTime":"1703577336606"
            }],
            "ts":1703577336700
        }"#,
    )
    .unwrap();

    match fill {
        BitgetWebsocketEvent::Fill {
            action, arg, data, ..
        } => {
            assert_eq!(action.as_deref(), Some("snapshot"));
            assert_eq!(arg.channel, "fill");
            assert_eq!(data[0].order_id.as_deref(), Some("111"));
            assert_eq!(data[0].client_order_id.as_deref(), Some("client-111"));
            assert_eq!(data[0].trade_id.as_deref(), Some("222"));
            assert_eq!(data[0].symbol.as_deref(), Some("BTCUSDT"));
            assert_eq!(data[0].side.as_deref(), Some("buy"));
            assert_eq!(data[0].order_type.as_deref(), Some("market"));
            assert_eq!(data[0].position_mode.as_deref(), Some("one_way_mode"));
            assert_eq!(data[0].price.as_deref(), Some("51000.5"));
            assert_eq!(data[0].base_volume.as_deref(), Some("0.01"));
            assert_eq!(data[0].quote_volume.as_deref(), Some("510.005"));
            assert_eq!(data[0].profit.as_deref(), Some("0"));
            assert_eq!(data[0].trade_side.as_deref(), Some("open"));
            assert_eq!(data[0].trade_scope.as_deref(), Some("taker"));
            assert_eq!(data[0].create_time.as_deref(), Some("1703577336606"));
            assert_eq!(data[0].update_time.as_deref(), Some("1703577336606"));
            assert_eq!(data[0].fee_detail[0].fee_coin.as_deref(), Some("USDT"));
            assert_eq!(data[0].fee_detail[0].deduction.as_deref(), Some("no"));
            assert_eq!(
                data[0].fee_detail[0].total_deduction_fee.as_deref(),
                Some("0")
            );
            assert_eq!(
                data[0].fee_detail[0].total_fee.as_deref(),
                Some("-0.183717")
            );
        }
        other => panic!("expected fill event, got {other:?}"),
    }
}

#[tokio::test]
async fn websocket_session_handles_ping_pong_and_subscriptions() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();

        let ping = socket.next().await.unwrap().unwrap();
        assert_eq!(ping, Message::Text("ping".into()));
        socket.send(Message::Text("pong".into())).await.unwrap();

        let subscribe = socket.next().await.unwrap().unwrap();
        let Message::Text(subscribe) = subscribe else {
            panic!("expected text subscribe message");
        };
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&subscribe).unwrap(),
            json!({
                "op":"subscribe",
                "args":[{"instType":"USDT-FUTURES","channel":"ticker","instId":"BTCUSDT"}]
            })
        );
        socket
            .send(Message::Text(
                r#"{"event":"subscribe","arg":{"instType":"USDT-FUTURES","channel":"ticker","instId":"BTCUSDT"}}"#
                    .into(),
            ))
            .await
            .unwrap();
        socket
            .send(Message::Text(
                r#"{"action":"snapshot","arg":{"instType":"USDT-FUTURES","channel":"ticker","instId":"BTCUSDT"},"data":[{"lastPr":"70000.1"}]}"#
                    .into(),
            ))
            .await
            .unwrap();
    });

    let websocket = BitgetWebsocket::new_public_with_urls(&url, "ws://127.0.0.1/private");
    let mut session = websocket.connect_public().await.unwrap();
    let ticker = BitgetWebsocketChannel::new("USDT-FUTURES", "ticker").with_inst_id("BTCUSDT");

    session.ping().await.unwrap();
    session.subscribe(&[ticker]).await.unwrap();

    assert_eq!(
        session.recv_event().await.unwrap(),
        BitgetWebsocketEvent::Pong
    );
    match session.recv_event().await.unwrap() {
        BitgetWebsocketEvent::Subscribed { arg, .. } => {
            assert_eq!(arg.channel, "ticker");
            assert_eq!(arg.inst_id.as_deref(), Some("BTCUSDT"));
        }
        other => panic!("expected subscribe event, got {other:?}"),
    }
    match session.recv_event().await.unwrap() {
        BitgetWebsocketEvent::Ticker { action, data, .. } => {
            assert_eq!(action.as_deref(), Some("snapshot"));
            assert_eq!(data[0].last_price.as_deref(), Some("70000.1"));
        }
        other => panic!("expected ticker event, got {other:?}"),
    }
}

#[tokio::test]
async fn websocket_manager_reconnects_and_replays_subscriptions() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (subscription_tx, mut subscription_rx) = mpsc::channel::<serde_json::Value>(2);

    tokio::spawn(async move {
        for connection in 0..2 {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let subscribe = socket.next().await.unwrap().unwrap();
            let Message::Text(subscribe) = subscribe else {
                panic!("expected subscription replay");
            };
            subscription_tx
                .send(serde_json::from_str(&subscribe).unwrap())
                .await
                .unwrap();
            socket
                .send(Message::Text(
                    format!(
                        r#"{{"action":"snapshot","arg":{{"instType":"USDT-FUTURES","channel":"ticker","instId":"BTCUSDT"}},"data":[{{"connection":{connection}}}]}}"#
                    )
                    .into(),
                ))
                .await
                .unwrap();
            if connection == 1 {
                let _ = socket.next().await;
            }
        }
    });

    let mut manager = BitgetWebsocketManager::new(
        url,
        ReconnectConfig::new(Duration::from_millis(10), 2)
            .with_ping_interval(Duration::from_secs(30)),
    );
    manager.add_subscription(
        BitgetWebsocketChannel::new("USDT-FUTURES", "ticker").with_inst_id("BTCUSDT"),
    );

    let mut events = manager.start().await.unwrap();
    let first_subscribe = timeout(Duration::from_secs(2), subscription_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let first_event = timeout(Duration::from_secs(2), events.recv())
        .await
        .unwrap()
        .unwrap();
    let second_subscribe = timeout(Duration::from_secs(2), subscription_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let second_event = timeout(Duration::from_secs(2), events.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(first_subscribe["op"], "subscribe");
    assert_eq!(second_subscribe["op"], "subscribe");
    assert!(matches!(first_event, BitgetWebsocketEvent::Ticker { .. }));
    assert!(matches!(second_event, BitgetWebsocketEvent::Ticker { .. }));
    assert_eq!(manager.connection_state(), ConnectionState::Connected);
    assert_eq!(manager.metrics().messages_received, 2);
    assert!(manager.metrics().reconnects >= 1);

    manager.stop().await;
    assert_eq!(manager.connection_state(), ConnectionState::Stopped);
}

#[tokio::test]
async fn websocket_manager_subscribes_while_running() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (subscription_tx, mut subscription_rx) = mpsc::channel::<serde_json::Value>(1);

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let subscribe = socket.next().await.unwrap().unwrap();
        let Message::Text(subscribe) = subscribe else {
            panic!("expected runtime subscription");
        };
        subscription_tx
            .send(serde_json::from_str(&subscribe).unwrap())
            .await
            .unwrap();
        socket
            .send(Message::Text(
                r#"{"action":"snapshot","arg":{"instType":"USDT-FUTURES","channel":"ticker","instId":"BTCUSDT"},"data":[{"lastPr":"71000.1"}]}"#
                    .into(),
            ))
            .await
            .unwrap();
    });

    let mut manager = BitgetWebsocketManager::new(
        url,
        ReconnectConfig::new(Duration::from_millis(10), 0)
            .with_ping_interval(Duration::from_secs(30)),
    );
    let mut events = manager.start().await.unwrap();
    manager
        .subscribe(BitgetWebsocketChannel::new("USDT-FUTURES", "ticker").with_inst_id("BTCUSDT"))
        .await
        .unwrap();

    let subscribe = timeout(Duration::from_secs(2), subscription_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        subscribe,
        json!({
            "op":"subscribe",
            "args":[{"instType":"USDT-FUTURES","channel":"ticker","instId":"BTCUSDT"}]
        })
    );
    match timeout(Duration::from_secs(2), events.recv())
        .await
        .unwrap()
        .unwrap()
    {
        BitgetWebsocketEvent::Ticker { data, .. } => {
            assert_eq!(data[0].last_price.as_deref(), Some("71000.1"));
        }
        other => panic!("expected ticker event, got {other:?}"),
    }

    manager.stop().await;
}

#[tokio::test]
async fn websocket_manager_unsubscribes_while_running_and_skips_replay() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (message_tx, mut message_rx) = mpsc::channel::<serde_json::Value>(3);

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();

        for _ in 0..2 {
            let message = socket.next().await.unwrap().unwrap();
            let Message::Text(message) = message else {
                panic!("expected text subscription command");
            };
            message_tx
                .send(serde_json::from_str(&message).unwrap())
                .await
                .unwrap();
        }
        socket.close(None).await.unwrap();

        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        assert!(
            timeout(Duration::from_millis(40), socket.next())
                .await
                .is_err(),
            "manager replayed a channel that was unsubscribed"
        );
        socket
            .send(Message::Text(
                r#"{"action":"snapshot","arg":{"instType":"USDT-FUTURES","channel":"ticker","instId":"ETHUSDT"},"data":[{"lastPr":"4200.1"}]}"#
                    .into(),
            ))
            .await
            .unwrap();
    });

    let mut manager = BitgetWebsocketManager::new(
        url,
        ReconnectConfig::new(Duration::from_millis(10), 1)
            .with_ping_interval(Duration::from_secs(30)),
    );
    let btc_ticker = BitgetWebsocketChannel::new("USDT-FUTURES", "ticker").with_inst_id("BTCUSDT");
    let mut events = manager.start().await.unwrap();
    manager.subscribe(btc_ticker.clone()).await.unwrap();
    manager.unsubscribe(btc_ticker).await.unwrap();

    let subscribe = timeout(Duration::from_secs(2), message_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let unsubscribe = timeout(Duration::from_secs(2), message_rx.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(subscribe["op"], "subscribe");
    assert_eq!(unsubscribe["op"], "unsubscribe");
    assert!(manager.subscriptions().is_empty());
    match timeout(Duration::from_secs(2), events.recv())
        .await
        .unwrap()
        .unwrap()
    {
        BitgetWebsocketEvent::Ticker { arg, data, .. } => {
            assert_eq!(arg.inst_id.as_deref(), Some("ETHUSDT"));
            assert_eq!(data[0].last_price.as_deref(), Some("4200.1"));
        }
        other => panic!("expected ticker event after reconnect, got {other:?}"),
    }

    manager.stop().await;
}

#[tokio::test]
async fn websocket_manager_reconnects_when_inbound_messages_stall() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (subscription_tx, mut subscription_rx) = mpsc::channel::<serde_json::Value>(2);

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let subscribe = socket.next().await.unwrap().unwrap();
        let Message::Text(subscribe) = subscribe else {
            panic!("expected first subscription");
        };
        subscription_tx
            .send(serde_json::from_str(&subscribe).unwrap())
            .await
            .unwrap();

        while let Some(Ok(message)) = socket.next().await {
            if matches!(message, Message::Close(_)) {
                break;
            }
        }

        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let subscribe = socket.next().await.unwrap().unwrap();
        let Message::Text(subscribe) = subscribe else {
            panic!("expected replayed subscription");
        };
        subscription_tx
            .send(serde_json::from_str(&subscribe).unwrap())
            .await
            .unwrap();
        socket
            .send(Message::Text(
                r#"{"action":"snapshot","arg":{"instType":"USDT-FUTURES","channel":"ticker","instId":"BTCUSDT"},"data":[{"connection":1}]}"#
                    .into(),
            ))
            .await
            .unwrap();
    });

    let mut manager = BitgetWebsocketManager::new(
        url,
        ReconnectConfig::new(Duration::from_millis(10), 2)
            .with_ping_interval(Duration::from_millis(20)),
    );
    manager.add_subscription(
        BitgetWebsocketChannel::new("USDT-FUTURES", "ticker").with_inst_id("BTCUSDT"),
    );

    let mut events = manager.start().await.unwrap();
    let first_subscribe = timeout(Duration::from_secs(2), subscription_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let second_subscribe = timeout(Duration::from_secs(2), subscription_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let event = timeout(Duration::from_secs(2), events.recv())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(first_subscribe["op"], "subscribe");
    assert_eq!(second_subscribe["op"], "subscribe");
    assert!(matches!(event, BitgetWebsocketEvent::Ticker { .. }));
    assert!(manager.metrics().reconnects >= 1);

    manager.stop().await;
}

#[tokio::test]
async fn websocket_manager_replays_login_before_private_subscriptions() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (message_tx, mut message_rx) = mpsc::channel::<serde_json::Value>(4);

    tokio::spawn(async move {
        for connection in 0..2 {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();

            let login = socket.next().await.unwrap().unwrap();
            let Message::Text(login) = login else {
                panic!("expected login message");
            };
            let login: serde_json::Value = serde_json::from_str(&login).unwrap();
            message_tx.send(login.clone()).await.unwrap();
            socket
                .send(Message::Text(
                    r#"{"event":"login","code":"0","msg":"success"}"#.into(),
                ))
                .await
                .unwrap();

            let subscribe = socket.next().await.unwrap().unwrap();
            let Message::Text(subscribe) = subscribe else {
                panic!("expected private subscription replay");
            };
            message_tx
                .send(serde_json::from_str(&subscribe).unwrap())
                .await
                .unwrap();
            socket
                .send(Message::Text(
                    format!(
                        r#"{{"action":"snapshot","arg":{{"instType":"USDT-FUTURES","channel":"orders","instId":"default"}},"data":[{{"connection":{connection}}}]}}"#
                    )
                    .into(),
                ))
                .await
                .unwrap();
            if connection == 1 {
                let _ = socket.next().await;
            }
        }
    });

    let mut manager = BitgetWebsocketManager::new(
        url,
        ReconnectConfig::new(Duration::from_millis(10), 2)
            .with_ping_interval(Duration::from_secs(30)),
    )
    .with_login_credentials(Credentials::new("test-key", "test-secret", "test-pass"));
    manager.add_subscription(
        BitgetWebsocketChannel::new("USDT-FUTURES", "orders").with_inst_id("default"),
    );

    let mut events = manager.start().await.unwrap();
    let first_login = timeout(Duration::from_secs(2), message_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let first_subscribe = timeout(Duration::from_secs(2), message_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let first_event = timeout(Duration::from_secs(2), events.recv())
        .await
        .unwrap()
        .unwrap();
    let second_login = timeout(Duration::from_secs(2), message_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let second_subscribe = timeout(Duration::from_secs(2), message_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let second_event = timeout(Duration::from_secs(2), events.recv())
        .await
        .unwrap()
        .unwrap();

    assert_login_payload(&first_login);
    assert_login_payload(&second_login);
    assert_eq!(first_subscribe["op"], "subscribe");
    assert_eq!(second_subscribe["op"], "subscribe");
    assert!(matches!(first_event, BitgetWebsocketEvent::Login { .. }));
    assert!(matches!(second_event, BitgetWebsocketEvent::Orders { .. }));
    assert!(manager.metrics().reconnects >= 1);

    manager.stop().await;
}

#[tokio::test]
async fn websocket_manager_waits_for_login_ack_before_private_subscriptions() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();

        let login = socket.next().await.unwrap().unwrap();
        let Message::Text(login) = login else {
            panic!("expected login message");
        };
        let login: serde_json::Value = serde_json::from_str(&login).unwrap();
        assert_login_payload(&login);

        assert!(
            timeout(Duration::from_millis(40), socket.next())
                .await
                .is_err(),
            "manager sent subscription before login ack"
        );

        socket
            .send(Message::Text(
                r#"{"event":"login","code":"0","msg":"success"}"#.into(),
            ))
            .await
            .unwrap();

        let subscribe = timeout(Duration::from_secs(2), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let Message::Text(subscribe) = subscribe else {
            panic!("expected private subscription after login ack");
        };
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&subscribe).unwrap(),
            json!({
                "op":"subscribe",
                "args":[{"instType":"USDT-FUTURES","channel":"orders","instId":"default"}]
            })
        );
        socket
            .send(Message::Text(
                r#"{"action":"snapshot","arg":{"instType":"USDT-FUTURES","channel":"orders","instId":"default"},"data":[{"orderId":"1"}]}"#
                    .into(),
            ))
            .await
            .unwrap();
    });

    let mut manager = BitgetWebsocketManager::new(
        url,
        ReconnectConfig::new(Duration::from_millis(10), 0)
            .with_ping_interval(Duration::from_secs(30)),
    )
    .with_login_credentials(Credentials::new("test-key", "test-secret", "test-pass"));
    manager.add_subscription(
        BitgetWebsocketChannel::new("USDT-FUTURES", "orders").with_inst_id("default"),
    );

    let mut events = manager.start().await.unwrap();
    assert!(matches!(
        timeout(Duration::from_secs(2), events.recv())
            .await
            .unwrap()
            .unwrap(),
        BitgetWebsocketEvent::Login { .. }
    ));
    assert!(matches!(
        timeout(Duration::from_secs(2), events.recv())
            .await
            .unwrap()
            .unwrap(),
        BitgetWebsocketEvent::Orders { .. }
    ));

    manager.stop().await;
}

fn assert_login_payload(value: &serde_json::Value) {
    assert_eq!(value["op"], "login");
    let args = value["args"].as_array().unwrap();
    let payload = &args[0];
    assert_eq!(payload["apiKey"], "test-key");
    assert_eq!(payload["passphrase"], "test-pass");

    let timestamp = payload["timestamp"].as_str().unwrap();
    let expected_sign =
        generate_signature("test-secret", &format!("{timestamp}GET/user/verify")).unwrap();
    assert_eq!(payload["sign"], expected_sign);
}

#[tokio::test]
async fn market_contracts_orderbook_and_candles_use_bitget_v2_paths() {
    let mut server = Server::new_async().await;
    let contracts = server
        .mock(
            "GET",
            "/api/v2/mix/market/contracts?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT"}]}"#)
        .create_async()
        .await;
    let orderbook = server
        .mock(
            "GET",
            "/api/v2/mix/market/orderbook?limit=5&productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"asks":[],"bids":[]}}"#)
        .create_async()
        .await;
    let candles = server
        .mock(
            "GET",
            "/api/v2/mix/market/candles?granularity=5m&limit=100&productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[]}"#)
        .create_async()
        .await;

    let market = BitgetMarket::new(public_client(server.url()));

    let contracts_value = market
        .get_contracts("USDT-FUTURES", Some("BTCUSDT"))
        .await
        .unwrap();
    let orderbook_value = market
        .get_orderbook("BTCUSDT", "USDT-FUTURES", Some("5"))
        .await
        .unwrap();
    let candles_value = market
        .get_candles("BTCUSDT", "USDT-FUTURES", "5m", Some(100))
        .await
        .unwrap();

    assert_eq!(contracts_value[0]["symbol"], "BTCUSDT");
    assert!(orderbook_value["asks"].is_array());
    assert!(candles_value.is_array());
    contracts.assert_async().await;
    orderbook.assert_async().await;
    candles.assert_async().await;
}

#[tokio::test]
async fn market_public_helpers_cover_time_ratios_and_oi_paths() {
    let mut server = Server::new_async().await;
    let time = server
        .mock("GET", "/api/v2/public/time")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"serverTime":"1688008631614"}}"#)
        .create_async()
        .await;
    let long_short = server
        .mock(
            "GET",
            "/api/v2/mix/market/long-short?period=5m&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"longShortRatio":"1.2"}]}"#)
        .create_async()
        .await;
    let account_long_short = server
        .mock(
            "GET",
            "/api/v2/mix/market/account-long-short?period=5m&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"longShortAccountRatio":"1.1"}]}"#)
        .create_async()
        .await;
    let oi_limit = server
        .mock(
            "GET",
            "/api/v2/mix/market/oi-limit?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT"}]}"#)
        .create_async()
        .await;
    let position_tier = server
        .mock(
            "GET",
            "/api/v2/mix/market/query-position-lever?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"level":"1"}]}"#)
        .create_async()
        .await;
    let taker_volume = server
        .mock(
            "GET",
            "/api/v2/mix/market/taker-buy-sell?period=5m&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"buyVolume":"10"}]}"#)
        .create_async()
        .await;

    let market = BitgetMarket::new(public_client(server.url()));

    let time_value = market.get_server_time().await.unwrap();
    let long_short_value = market
        .get_long_short_ratio("BTCUSDT", Some("5m"))
        .await
        .unwrap();
    let account_ratio_value = market
        .get_account_long_short_ratio("BTCUSDT", Some("5m"))
        .await
        .unwrap();
    let oi_limit_value = market
        .get_open_interest_limit("USDT-FUTURES", Some("BTCUSDT"))
        .await
        .unwrap();
    let tier_value = market
        .get_position_tier("BTCUSDT", "USDT-FUTURES")
        .await
        .unwrap();
    let taker_volume_value = market
        .get_taker_buy_sell_volume("BTCUSDT", Some("5m"))
        .await
        .unwrap();

    assert_eq!(time_value["serverTime"], "1688008631614");
    assert_eq!(long_short_value[0]["longShortRatio"], "1.2");
    assert_eq!(account_ratio_value[0]["longShortAccountRatio"], "1.1");
    assert_eq!(oi_limit_value[0]["symbol"], "BTCUSDT");
    assert_eq!(tier_value[0]["level"], "1");
    assert_eq!(taker_volume_value[0]["buyVolume"], "10");
    time.assert_async().await;
    long_short.assert_async().await;
    account_long_short.assert_async().await;
    oi_limit.assert_async().await;
    position_tier.assert_async().await;
    taker_volume.assert_async().await;
}

#[tokio::test]
async fn account_get_accounts_sends_bitget_signature_headers() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock(
            "GET",
            "/api/v2/mix/account/accounts?productType=USDT-FUTURES",
        )
        .match_header("ACCESS-KEY", "test-key")
        .match_header("ACCESS-PASSPHRASE", "test-pass")
        .match_header("ACCESS-TIMESTAMP", "1684814440729")
        .match_header("ACCESS-SIGN", Matcher::Regex(".+".to_string()))
        .match_header("locale", "en-US")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"00000",
                "msg":"success",
                "requestTime":1695794095685,
                "data":[{
                    "marginCoin":"USDT",
                    "locked":"1.2",
                    "available":"105.5",
                    "accountEquity":"106.7",
                    "usdtEquity":"106.7"
                }]
            }"#,
        )
        .create_async()
        .await;

    let account = BitgetAccount::new(signed_client(server.url()));
    let accounts = account.get_accounts("USDT-FUTURES").await.unwrap();

    assert_eq!(accounts[0].margin_coin, "USDT");
    assert_eq!(accounts[0].available, "105.5");
    assert_eq!(accounts[0].account_equity, "106.7");
    mock.assert_async().await;
}

#[tokio::test]
async fn account_positions_and_leverage_use_signed_v2_paths() {
    let mut server = Server::new_async().await;
    let positions = server
        .mock(
            "GET",
            "/api/v2/mix/position/all-position?marginCoin=USDT&productType=USDT-FUTURES",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT"}]}"#)
        .create_async()
        .await;
    let leverage = server
        .mock("POST", "/api/v2/mix/account/set-leverage")
        .match_header("ACCESS-KEY", "test-key")
        .match_header("ACCESS-TIMESTAMP", "1684814440729")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","leverage":"20"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"leverage":"20"}}"#)
        .create_async()
        .await;

    let account = BitgetAccount::new(signed_client(server.url()));

    let position_value = account
        .get_all_positions("USDT-FUTURES", Some("USDT"))
        .await
        .unwrap();
    let leverage_value = account
        .set_leverage("BTCUSDT", "USDT-FUTURES", "USDT", "20")
        .await
        .unwrap();

    assert_eq!(position_value[0]["symbol"], "BTCUSDT");
    assert_eq!(leverage_value["leverage"], "20");
    positions.assert_async().await;
    leverage.assert_async().await;
}

#[tokio::test]
async fn account_bills_margin_and_asset_mode_use_signed_v2_paths() {
    let mut server = Server::new_async().await;
    let bills = server
        .mock(
            "GET",
            "/api/v2/mix/account/bill?businessType=contract_settle_fee&coin=USDT&endTime=200&limit=50&onlyFunding=yes&productType=USDT-FUTURES&startTime=100",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"bills":[],"endId":"2"}}"#)
        .create_async()
        .await;
    let margin = server
        .mock("POST", "/api/v2/mix/account/set-margin")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","amount":"20","holdSide":"long"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":"success"}"#)
        .create_async()
        .await;
    let asset_mode = server
        .mock("POST", "/api/v2/mix/account/set-asset-mode")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"productType":"USDT-FUTURES","assetMode":"union"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":"success"}"#)
        .create_async()
        .await;
    let trade_rate = server
        .mock(
            "GET",
            "/api/v2/common/trade-rate?businessType=mix&symbol=BTCUSDT",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"makerFeeRate":"0.0002"}}"#)
        .create_async()
        .await;

    let account = BitgetAccount::new(signed_client(server.url()));

    let bills_value = account
        .get_account_bills(
            AccountBillRequest::new("USDT-FUTURES")
                .with_coin("USDT")
                .with_business_type("contract_settle_fee")
                .with_only_funding("yes")
                .with_start_time(100)
                .with_end_time(200)
                .with_limit(50),
        )
        .await
        .unwrap();
    let margin_value = account
        .set_position_margin("BTCUSDT", "USDT-FUTURES", "USDT", "20", "long")
        .await
        .unwrap();
    let asset_mode_value = account
        .set_asset_mode("USDT-FUTURES", "union")
        .await
        .unwrap();
    let trade_rate_value = account.get_trade_rate("BTCUSDT", "mix").await.unwrap();

    assert_eq!(bills_value["endId"], "2");
    assert_eq!(margin_value, "success");
    assert_eq!(asset_mode_value, "success");
    assert_eq!(trade_rate_value["makerFeeRate"], "0.0002");
    bills.assert_async().await;
    margin.assert_async().await;
    asset_mode.assert_async().await;
    trade_rate.assert_async().await;
}

#[tokio::test]
async fn announcements_query_uses_bitget_public_notice_endpoint() {
    let mut server = Server::new_async().await;
    let notices = server
        .mock(
            "GET",
            "/api/v2/public/annoucements?annType=api_trading&language=en_US&limit=10",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"announcements":[]}}"#)
        .create_async()
        .await;

    let announcements = BitgetAnnouncements::new(public_client(server.url()));
    let value = announcements
        .get_announcements(
            AnnouncementListRequest::new("en_US")
                .with_ann_type("api_trading")
                .with_limit(10),
        )
        .await
        .unwrap();

    assert!(value["announcements"].is_array());
    notices.assert_async().await;
}

#[tokio::test]
async fn trade_place_and_cancel_order_sign_json_body() {
    let mut server = Server::new_async().await;
    let place = server
        .mock("POST", "/api/v2/mix/order/place-order")
        .match_header("ACCESS-KEY", "test-key")
        .match_header("ACCESS-SIGN", Matcher::Regex(".+".to_string()))
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginMode":"crossed","marginCoin":"USDT","size":"0.001","side":"buy","orderType":"limit","price":"10000","tradeSide":"open","force":"post_only"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"orderId":"1","clientOid":"c1"}}"#,
        )
        .create_async()
        .await;
    let cancel = server
        .mock("POST", "/api/v2/mix/order/cancel-order")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","orderId":"1"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"orderId":"1"}}"#)
        .create_async()
        .await;

    let trade = BitgetTrade::new(signed_client(server.url()));

    let placed = trade
        .place_order(
            NewOrderRequest::limit(
                "BTCUSDT",
                "USDT-FUTURES",
                "crossed",
                "USDT",
                "0.001",
                "buy",
                "10000",
            )
            .with_trade_side("open")
            .with_force("post_only"),
        )
        .await
        .unwrap();
    let canceled = trade
        .cancel_order(
            CancelOrderRequest::new("BTCUSDT", "USDT-FUTURES")
                .with_margin_coin("USDT")
                .with_order_id("1"),
        )
        .await
        .unwrap();

    assert_eq!(placed["orderId"], "1");
    assert_eq!(canceled["orderId"], "1");
    place.assert_async().await;
    cancel.assert_async().await;
}

#[tokio::test]
async fn trade_query_modify_cancel_all_and_close_use_v2_paths() {
    let mut server = Server::new_async().await;
    let pending = server
        .mock(
            "GET",
            "/api/v2/mix/order/orders-pending?endTime=200&idLessThan=9&limit=20&productType=USDT-FUTURES&startTime=100&status=live&symbol=BTCUSDT",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"entrustedList":[],"endId":"9"}}"#)
        .create_async()
        .await;
    let modify = server
        .mock("POST", "/api/v2/mix/order/modify-order")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","orderId":"1","newClientOid":"new-1","newSize":"0.002","newPrice":"11000"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"orderId":"2"}}"#)
        .create_async()
        .await;
    let cancel_all = server
        .mock("POST", "/api/v2/mix/order/cancel-all-orders")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"productType":"USDT-FUTURES","marginCoin":"USDT"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"successList":[]}}"#)
        .create_async()
        .await;
    let close = server
        .mock("POST", "/api/v2/mix/order/close-positions")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","holdSide":"long"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"orderIdList":["1"]}}"#)
        .create_async()
        .await;

    let trade = BitgetTrade::new(signed_client(server.url()));

    let pending_value = trade
        .get_pending_orders_with(
            OrderQueryRequest::new("USDT-FUTURES")
                .with_symbol("BTCUSDT")
                .with_status("live")
                .with_id_less_than("9")
                .with_start_time(100)
                .with_end_time(200)
                .with_limit(20),
        )
        .await
        .unwrap();
    let modify_value = trade
        .modify_order(
            ModifyOrderRequest::new("BTCUSDT", "USDT-FUTURES", "new-1")
                .with_margin_coin("USDT")
                .with_order_id("1")
                .with_new_size("0.002")
                .with_new_price("11000"),
        )
        .await
        .unwrap();
    let cancel_all_value = trade
        .cancel_all_orders(CancelAllOrdersRequest::new("USDT-FUTURES").with_margin_coin("USDT"))
        .await
        .unwrap();
    let close_value = trade
        .close_positions(
            ClosePositionsRequest::new("USDT-FUTURES")
                .with_symbol("BTCUSDT")
                .with_hold_side("long"),
        )
        .await
        .unwrap();

    assert_eq!(pending_value["endId"], "9");
    assert_eq!(modify_value["orderId"], "2");
    assert!(cancel_all_value["successList"].is_array());
    assert_eq!(close_value["orderIdList"][0], "1");
    pending.assert_async().await;
    modify.assert_async().await;
    cancel_all.assert_async().await;
    close.assert_async().await;
}

#[tokio::test]
async fn asset_wallet_methods_use_bitget_v2_paths() {
    let mut server = Server::new_async().await;
    let coins = server
        .mock("GET", "/api/v2/spot/public/coins?coin=USDT")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"coin":"USDT"}]}"#)
        .create_async()
        .await;
    let deposit_address = server
        .mock(
            "GET",
            "/api/v2/spot/wallet/deposit-address?chain=trc20&coin=USDT",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"coin":"USDT","chain":"trc20"}}"#)
        .create_async()
        .await;
    let transfer = server
        .mock("POST", "/api/v2/spot/wallet/transfer")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"fromType":"spot","toType":"usdt_futures","amount":"10","coin":"USDT"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"transferId":"t1"}}"#)
        .create_async()
        .await;

    let asset = BitgetAsset::new(signed_client(server.url()));

    let coins_value = asset.get_coins(Some("USDT")).await.unwrap();
    let address_value = asset
        .get_deposit_address(DepositAddressRequest::new("USDT", "trc20"))
        .await
        .unwrap();
    let transfer_value = asset
        .transfer(TransferRequest::new("spot", "usdt_futures", "10", "USDT"))
        .await
        .unwrap();

    assert_eq!(coins_value[0]["coin"], "USDT");
    assert_eq!(address_value["chain"], "trc20");
    assert_eq!(transfer_value["transferId"], "t1");
    coins.assert_async().await;
    deposit_address.assert_async().await;
    transfer.assert_async().await;
}

#[tokio::test]
async fn asset_history_transferable_coins_and_withdraw_use_v2_paths() {
    let mut server = Server::new_async().await;
    let deposit_history = server
        .mock(
            "GET",
            "/api/v2/spot/wallet/deposit-records?clientOid=c1&coin=USDT&endTime=200&idLessThan=9&limit=20&startTime=100",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"orderId":"9"}]}"#)
        .create_async()
        .await;
    let transferable = server
        .mock(
            "GET",
            "/api/v2/spot/wallet/transfer-coin-info?fromType=spot&toType=usdt_futures",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":["USDT"]}"#)
        .create_async()
        .await;
    let withdrawal = server
        .mock("POST", "/api/v2/spot/wallet/withdrawal")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"coin":"USDT","transferType":"on_chain","address":"TXYZ","size":"10","chain":"trc20","clientOid":"w1"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"orderId":"w1"}}"#)
        .create_async()
        .await;

    let asset = BitgetAsset::new(signed_client(server.url()));

    let history_value = asset
        .get_deposit_records(
            WalletHistoryRequest::new(100, 200)
                .with_coin("USDT")
                .with_client_oid("c1")
                .with_id_less_than("9")
                .with_limit(20),
        )
        .await
        .unwrap();
    let transferable_value = asset
        .get_transferable_coins("spot", "usdt_futures")
        .await
        .unwrap();
    let withdrawal_value = asset
        .withdraw(
            WithdrawRequest::on_chain("USDT", "TXYZ", "10")
                .with_chain("trc20")
                .with_client_oid("w1"),
        )
        .await
        .unwrap();

    assert_eq!(history_value[0]["orderId"], "9");
    assert_eq!(transferable_value[0], "USDT");
    assert_eq!(withdrawal_value["orderId"], "w1");
    deposit_history.assert_async().await;
    transferable.assert_async().await;
    withdrawal.assert_async().await;
}
