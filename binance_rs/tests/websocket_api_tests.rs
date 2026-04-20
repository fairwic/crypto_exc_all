use binance_rs::api::websocket::{
    BinanceStreamRoute, BinanceWebsocket, BinanceWebsocketEvent, BinanceWebsocketHub,
    BinanceWebsocketManager, ConnectionState, ReconnectConfig, StreamSubscription,
};
use binance_rs::client::BinanceClient;
use binance_rs::config::Credentials;
use futures_util::{SinkExt, StreamExt};
use mockito::Server;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

fn api_key_client(server_url: String) -> BinanceClient {
    let mut client = BinanceClient::new(Credentials::new("test-key", "test-secret")).unwrap();
    client.set_base_url(server_url);
    client
}

#[tokio::test]
async fn websocket_user_stream_maps_listen_key_endpoints() {
    let mut server = Server::new_async().await;
    let start = server
        .mock("POST", "/fapi/v1/listenKey")
        .match_header("x-mbx-apikey", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"listenKey":"listen-key"}"#)
        .create_async()
        .await;
    let keepalive = server
        .mock("PUT", "/fapi/v1/listenKey")
        .match_header("x-mbx-apikey", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"listenKey":"listen-key"}"#)
        .create_async()
        .await;
    let close = server
        .mock("DELETE", "/fapi/v1/listenKey")
        .match_header("x-mbx-apikey", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{}"#)
        .create_async()
        .await;

    let websocket = BinanceWebsocket::new(api_key_client(server.url()));

    assert_eq!(
        websocket.start_user_data_stream().await.unwrap()["listenKey"],
        "listen-key"
    );
    assert_eq!(
        websocket.keepalive_user_data_stream().await.unwrap()["listenKey"],
        "listen-key"
    );
    assert_eq!(
        websocket.close_user_data_stream().await.unwrap(),
        serde_json::json!({})
    );

    start.assert_async().await;
    keepalive.assert_async().await;
    close.assert_async().await;
}

#[test]
fn websocket_url_builders_use_split_binance_futures_routes() {
    let websocket = BinanceWebsocket::new_public_with_stream_base_url("wss://fstream.binance.com");

    assert_eq!(
        websocket.public_ws_url(&["btcusdt@depth", "ethusdt@depth"]),
        "wss://fstream.binance.com/public/ws/btcusdt@depth/ethusdt@depth"
    );
    assert_eq!(
        websocket.market_stream_url(&["btcusdt@aggTrade", "ethusdt@markPrice"]),
        "wss://fstream.binance.com/market/stream?streams=btcusdt@aggTrade/ethusdt@markPrice"
    );
    assert_eq!(
        websocket.private_ws_url("listen-key", &["ORDER_TRADE_UPDATE", "ACCOUNT_UPDATE"]),
        "wss://fstream.binance.com/private/ws?listenKey=listen-key&events=ORDER_TRADE_UPDATE/ACCOUNT_UPDATE"
    );
    assert_eq!(
        websocket.public_route_ws_url(),
        "wss://fstream.binance.com/public/ws"
    );
    assert_eq!(
        websocket.market_route_ws_url(),
        "wss://fstream.binance.com/market/ws"
    );
    assert_eq!(
        websocket.private_route_ws_url(),
        "wss://fstream.binance.com/private/ws"
    );
}

#[tokio::test]
async fn websocket_session_receives_json_and_sends_subscription_ops() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (sent_tx, mut sent_rx) = mpsc::channel::<Value>(4);

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(
                r#"{"stream":"btcusdt@aggTrade","data":{"e":"aggTrade"}}"#.into(),
            ))
            .await
            .unwrap();

        for _ in 0..2 {
            if let Some(Ok(Message::Text(text))) = socket.next().await {
                sent_tx
                    .send(serde_json::from_str::<Value>(&text).unwrap())
                    .await
                    .unwrap();
            }
        }
    });

    let websocket = BinanceWebsocket::new_public_with_stream_base_url("wss://fstream.binance.com");
    let mut session = websocket.connect_url(&url).await.unwrap();

    let received = timeout(Duration::from_secs(1), session.recv_json())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(received["data"]["e"], "aggTrade");

    session.subscribe(&["btcusdt@aggTrade"]).await.unwrap();
    session.unsubscribe(&["btcusdt@aggTrade"]).await.unwrap();

    let subscribe = timeout(Duration::from_secs(1), sent_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(subscribe["method"], "SUBSCRIBE");
    assert_eq!(subscribe["params"][0], "btcusdt@aggTrade");

    let unsubscribe = timeout(Duration::from_secs(1), sent_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unsubscribe["method"], "UNSUBSCRIBE");
    assert_eq!(unsubscribe["params"][0], "btcusdt@aggTrade");
}

#[tokio::test]
async fn websocket_manager_reconnects_and_replays_subscriptions() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (subscription_tx, mut subscription_rx) = mpsc::channel::<Value>(4);

    tokio::spawn(async move {
        for connection in 1..=2 {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();

            if let Some(Ok(Message::Text(text))) = socket.next().await {
                subscription_tx
                    .send(serde_json::from_str::<Value>(&text).unwrap())
                    .await
                    .unwrap();
            }

            socket
                .send(Message::Text(
                    format!(r#"{{"connection":{connection}}}"#).into(),
                ))
                .await
                .unwrap();
            socket.close(None).await.unwrap();
        }
    });

    let mut manager =
        BinanceWebsocketManager::new(url, ReconnectConfig::new(Duration::from_millis(10), 2));
    manager.add_subscription("btcusdt@aggTrade");
    let mut receiver = manager.start().await.unwrap();

    let first = timeout(Duration::from_secs(2), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    let second = timeout(Duration::from_secs(2), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first["connection"], 1);
    assert_eq!(second["connection"], 2);

    let first_subscribe = timeout(Duration::from_secs(1), subscription_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let second_subscribe = timeout(Duration::from_secs(1), subscription_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first_subscribe["method"], "SUBSCRIBE");
    assert_eq!(second_subscribe["method"], "SUBSCRIBE");

    manager.stop().await;
}

#[tokio::test]
async fn websocket_session_can_connect_through_socks5h_proxy() {
    let ws_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let ws_url = format!("ws://{}", ws_listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (stream, _) = ws_listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(r#"{"proxied":true}"#.into()))
            .await
            .unwrap();
    });

    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_url = format!("socks5h://{}", proxy_listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (mut inbound, _) = proxy_listener.accept().await.unwrap();

        let mut greeting = [0_u8; 3];
        inbound.read_exact(&mut greeting).await.unwrap();
        assert_eq!(greeting, [0x05, 0x01, 0x00]);
        inbound.write_all(&[0x05, 0x00]).await.unwrap();

        let mut header = [0_u8; 4];
        inbound.read_exact(&mut header).await.unwrap();
        assert_eq!(header, [0x05, 0x01, 0x00, 0x03]);

        let mut host_len = [0_u8; 1];
        inbound.read_exact(&mut host_len).await.unwrap();
        let mut host = vec![0_u8; usize::from(host_len[0])];
        inbound.read_exact(&mut host).await.unwrap();
        let host = String::from_utf8(host).unwrap();

        let mut port = [0_u8; 2];
        inbound.read_exact(&mut port).await.unwrap();
        let port = u16::from_be_bytes(port);

        inbound
            .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
            .await
            .unwrap();

        let mut outbound = tokio::net::TcpStream::connect((host.as_str(), port))
            .await
            .unwrap();
        let _ = copy_bidirectional(&mut inbound, &mut outbound).await;
    });

    let websocket = BinanceWebsocket::new_public_with_stream_base_url("wss://fstream.binance.com")
        .with_proxy_url(proxy_url);
    let mut session = websocket.connect_url(&ws_url).await.unwrap();
    let received = timeout(Duration::from_secs(1), session.recv_json())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(received["proxied"], true);
}

#[test]
fn websocket_event_parser_maps_order_and_account_updates() {
    let order = serde_json::json!({
        "e": "ORDER_TRADE_UPDATE",
        "E": 1568879465651_u64,
        "T": 1568879465650_u64,
        "o": {
            "s": "BTCUSDT",
            "c": "TEST",
            "S": "SELL",
            "o": "LIMIT",
            "x": "NEW",
            "X": "NEW",
            "i": 8886774_u64,
            "q": "0.001",
            "p": "10000"
        }
    });

    let parsed = BinanceWebsocketEvent::parse(order.clone()).unwrap();
    match parsed {
        BinanceWebsocketEvent::OrderTradeUpdate(update) => {
            assert_eq!(update.event_time, 1_568_879_465_651);
            assert_eq!(update.order.symbol, "BTCUSDT");
            assert_eq!(update.order.order_id, 8_886_774);
            assert_eq!(update.order.execution_type, "NEW");
            assert_eq!(update.order.status, "NEW");
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let account = serde_json::json!({
        "e": "ACCOUNT_UPDATE",
        "E": 1564745798939_u64,
        "T": 1564745798938_u64,
        "a": {
            "m": "ORDER",
            "B": [{"a":"USDT","wb":"122624.12345678","cw":"100.12345678","bc":"50.12345678"}],
            "P": [{"s":"BTCUSDT","pa":"0","ep":"0.00000","bep":"0","cr":"200","up":"0","mt":"isolated","iw":"0.00000000","ps":"BOTH"}]
        }
    });

    let parsed = BinanceWebsocketEvent::parse(account.clone()).unwrap();
    match parsed {
        BinanceWebsocketEvent::AccountUpdate(update) => {
            assert_eq!(update.event_time, 1_564_745_798_939);
            assert_eq!(update.data.reason, "ORDER");
            assert_eq!(update.data.balances[0].asset, "USDT");
            assert_eq!(update.data.positions[0].symbol, "BTCUSDT");
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let parsed = BinanceWebsocketEvent::parse(serde_json::json!({
        "stream": "btcusdt@aggTrade",
        "data": {"e":"aggTrade","s":"BTCUSDT"}
    }))
    .unwrap();
    match parsed {
        BinanceWebsocketEvent::Raw(value) => assert_eq!(value["data"]["e"], "aggTrade"),
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn websocket_event_parser_maps_remaining_user_data_events() {
    let parsed = BinanceWebsocketEvent::parse(serde_json::json!({
        "stream": "user-data",
        "data": {
            "e": "listenKeyExpired",
            "E": "1736996475556",
            "listenKey": "listen-key"
        }
    }))
    .unwrap();
    match parsed {
        BinanceWebsocketEvent::ListenKeyExpired(update) => {
            assert_eq!(update.event_time, 1_736_996_475_556);
            assert_eq!(update.listen_key, "listen-key");
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let parsed = BinanceWebsocketEvent::parse(serde_json::json!({
        "e": "MARGIN_CALL",
        "E": 1587727187525_u64,
        "cw": "3.16812045",
        "p": [{
            "s": "ETHUSDT",
            "ps": "LONG",
            "pa": "1.327",
            "mt": "CROSSED",
            "iw": "0",
            "mp": "187.17127",
            "up": "-1.166074",
            "mm": "1.614445"
        }]
    }))
    .unwrap();
    match parsed {
        BinanceWebsocketEvent::MarginCall(update) => {
            assert_eq!(update.cross_wallet_balance.as_deref(), Some("3.16812045"));
            assert_eq!(update.positions[0].symbol, "ETHUSDT");
            assert_eq!(update.positions[0].maintenance_margin_required, "1.614445");
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let parsed = BinanceWebsocketEvent::parse(serde_json::json!({
        "e": "TRADE_LITE",
        "E": 1721895408092_u64,
        "T": 1721895408214_u64,
        "s": "BTCUSDT",
        "q": "0.001",
        "p": "0",
        "m": false,
        "c": "client-id",
        "S": "BUY",
        "L": "64089.20",
        "l": "0.040",
        "t": 109100866_u64,
        "i": 8886774_u64
    }))
    .unwrap();
    match parsed {
        BinanceWebsocketEvent::TradeLite(update) => {
            assert_eq!(update.symbol, "BTCUSDT");
            assert_eq!(update.client_order_id, "client-id");
            assert_eq!(update.trade_id, 109_100_866);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let parsed = BinanceWebsocketEvent::parse(serde_json::json!({
        "e": "ACCOUNT_CONFIG_UPDATE",
        "E": 1611646737479_u64,
        "T": 1611646737476_u64,
        "ac": {"s": "BTCUSDT", "l": 25}
    }))
    .unwrap();
    match parsed {
        BinanceWebsocketEvent::AccountConfigUpdate(update) => {
            let symbol_config = update.symbol_config.expect("symbol config");
            assert_eq!(symbol_config.symbol, "BTCUSDT");
            assert_eq!(symbol_config.leverage, 25);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let parsed = BinanceWebsocketEvent::parse(serde_json::json!({
        "e": "ACCOUNT_CONFIG_UPDATE",
        "E": 1611646737479_u64,
        "T": 1611646737476_u64,
        "ai": {"j": true}
    }))
    .unwrap();
    match parsed {
        BinanceWebsocketEvent::AccountConfigUpdate(update) => {
            assert!(
                update
                    .user_config
                    .expect("user config")
                    .multi_assets_margin_mode
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let parsed = BinanceWebsocketEvent::parse(serde_json::json!({
        "e": "STRATEGY_UPDATE",
        "T": 1669261797627_u64,
        "E": 1669261797628_u64,
        "su": {
            "si": 176054594_u64,
            "st": "GRID",
            "ss": "NEW",
            "s": "BTCUSDT",
            "ut": 1669261797627_u64,
            "c": 8007
        }
    }))
    .unwrap();
    match parsed {
        BinanceWebsocketEvent::StrategyUpdate(update) => {
            assert_eq!(update.update.strategy_id, 176_054_594);
            assert_eq!(update.update.opcode, 8007);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let parsed = BinanceWebsocketEvent::parse(serde_json::json!({
        "e": "GRID_UPDATE",
        "T": 1669262908216_u64,
        "E": 1669262908218_u64,
        "gu": {
            "si": 176057039_u64,
            "st": "GRID",
            "ss": "WORKING",
            "s": "BTCUSDT",
            "r": "-0.00300716",
            "up": "16720",
            "uq": "-0.001",
            "uf": "-0.00300716",
            "mp": "0.0",
            "ut": 1669262908197_u64
        }
    }))
    .unwrap();
    match parsed {
        BinanceWebsocketEvent::GridUpdate(update) => {
            assert_eq!(update.update.strategy_status, "WORKING");
            assert_eq!(update.update.unmatched_quantity, "-0.001");
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let parsed = BinanceWebsocketEvent::parse(serde_json::json!({
        "e": "CONDITIONAL_ORDER_TRIGGER_REJECT",
        "E": 1685517224945_u64,
        "T": 1685517224955_u64,
        "or": {
            "s": "ETHUSDT",
            "i": 155618472834_u64,
            "r": "FOK order has been rejected"
        }
    }))
    .unwrap();
    match parsed {
        BinanceWebsocketEvent::ConditionalOrderTriggerReject(update) => {
            assert_eq!(update.order.symbol, "ETHUSDT");
            assert_eq!(update.order.order_id, 155_618_472_834);
            assert_eq!(update.order.reject_reason, "FOK order has been rejected");
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let parsed = BinanceWebsocketEvent::parse(serde_json::json!({
        "e": "ALGO_UPDATE",
        "T": 1750515742297_u64,
        "E": 1750515742303_u64,
        "o": {
            "caid": "client-algo-id",
            "aid": 2148719_u64,
            "at": "CONDITIONAL",
            "o": "TAKE_PROFIT",
            "s": "BNBUSDT",
            "S": "SELL",
            "ps": "BOTH",
            "f": "GTC",
            "q": "0.01",
            "X": "CANCELED",
            "ai": "",
            "ap": "0.00000",
            "aq": "0.00000",
            "act": "0",
            "tp": "750",
            "p": "750",
            "V": "EXPIRE_MAKER",
            "wt": "CONTRACT_PRICE",
            "pm": "NONE",
            "cp": false,
            "pP": false,
            "R": false,
            "tt": 0,
            "gtd": 0,
            "rm": "Reduce Only reject"
        }
    }))
    .unwrap();
    match parsed {
        BinanceWebsocketEvent::AlgoUpdate(update) => {
            assert_eq!(update.order.algo_id, 2_148_719);
            assert_eq!(update.order.algo_status, "CANCELED");
            assert_eq!(
                update.order.reject_reason.as_deref(),
                Some("Reduce Only reject")
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn websocket_manager_exposes_state_and_health_metrics() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(r#"{"connection":1}"#.into()))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
    });

    let mut manager =
        BinanceWebsocketManager::new(url, ReconnectConfig::new(Duration::from_millis(10), 2));
    assert_eq!(manager.connection_state(), ConnectionState::Disconnected);
    assert!(!manager.is_healthy(Duration::from_secs(1)));

    let mut receiver = manager.start().await.unwrap();
    let first = timeout(Duration::from_secs(2), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first["connection"], 1);

    let metrics = manager.metrics();
    assert_eq!(manager.connection_state(), ConnectionState::Connected);
    assert!(manager.is_healthy(Duration::from_secs(1)));
    assert_eq!(metrics.messages_received, 1);
    assert_eq!(metrics.reconnects, 0);
    assert!(metrics.connected_at.is_some());
    assert!(metrics.last_message_at.is_some());

    manager.stop().await;
    assert_eq!(manager.connection_state(), ConnectionState::Stopped);
}

#[tokio::test]
async fn websocket_hub_routes_subscriptions_to_split_public_market_private_urls() {
    let public_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let market_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let private_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let public_url = format!("ws://{}", public_listener.local_addr().unwrap());
    let market_url = format!("ws://{}", market_listener.local_addr().unwrap());
    let private_url = format!("ws://{}", private_listener.local_addr().unwrap());

    let (route_tx, mut route_rx) = mpsc::channel::<String>(3);
    spawn_route_server(public_listener, "public", route_tx.clone());
    spawn_route_server(market_listener, "market", route_tx.clone());
    spawn_route_server(private_listener, "private", route_tx);

    let hub = BinanceWebsocketHub::new()
        .with_route_url(BinanceStreamRoute::Public, public_url)
        .with_route_url(BinanceStreamRoute::Market, market_url)
        .with_route_url(BinanceStreamRoute::Private, private_url);
    let mut receiver = hub
        .start(vec![
            StreamSubscription::public("btcusdt@depth"),
            StreamSubscription::market("btcusdt@aggTrade"),
            StreamSubscription::private("listen-key", &["ORDER_TRADE_UPDATE"]),
        ])
        .await
        .unwrap();

    let mut routes = Vec::new();
    for _ in 0..3 {
        let message = timeout(Duration::from_secs(2), receiver.recv())
            .await
            .unwrap()
            .unwrap();
        routes.push(message["route"].as_str().unwrap().to_string());
    }
    routes.sort();
    assert_eq!(routes, vec!["market", "private", "public"]);

    let mut connected_routes = Vec::new();
    for _ in 0..3 {
        connected_routes.push(
            timeout(Duration::from_secs(1), route_rx.recv())
                .await
                .unwrap()
                .unwrap(),
        );
    }
    connected_routes.sort();
    assert_eq!(connected_routes, vec!["market", "private", "public"]);
}

#[tokio::test]
async fn websocket_hub_does_not_resubscribe_when_route_url_embeds_streams() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "ws://{}/stream?streams=btcusdt@aggTrade",
        listener.local_addr().unwrap()
    );
    let (subscription_tx, mut subscription_rx) = mpsc::channel::<Value>(1);

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(r#"{"stream":"btcusdt@aggTrade"}"#.into()))
            .await
            .unwrap();

        if let Ok(Some(Ok(Message::Text(text)))) =
            timeout(Duration::from_millis(100), socket.next()).await
        {
            subscription_tx
                .send(serde_json::from_str::<Value>(&text).unwrap())
                .await
                .unwrap();
        }
    });

    let hub = BinanceWebsocketHub::new().with_route_url(BinanceStreamRoute::Market, url);
    let mut receiver = hub
        .start(vec![StreamSubscription::market("btcusdt@aggTrade")])
        .await
        .unwrap();

    let message = timeout(Duration::from_secs(1), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(message["stream"], "btcusdt@aggTrade");

    let maybe_subscription = timeout(Duration::from_millis(200), subscription_rx.recv()).await;
    assert!(
        !matches!(maybe_subscription, Ok(Some(_))),
        "unexpected subscription payload: {maybe_subscription:?}"
    );
}

fn spawn_route_server(listener: TcpListener, route: &'static str, route_tx: mpsc::Sender<String>) {
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        route_tx.send(route.to_string()).await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        socket
            .send(Message::Text(format!(r#"{{"route":"{route}"}}"#).into()))
            .await
            .unwrap();
        let _ = socket.next().await;
    });
}
