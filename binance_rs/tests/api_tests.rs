use binance_rs::api::account::BinanceAccount;
use binance_rs::api::market::BinanceMarket;
use binance_rs::api::trade::{BinanceTrade, NewOrderRequest};
use binance_rs::client::BinanceClient;
use binance_rs::config::Credentials;
use binance_rs::utils::generate_signature;
use mockito::{Matcher, Server};

#[tokio::test]
async fn account_get_balance_uses_signed_futures_balance_endpoint() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/fapi/v2/balance")
        .match_header("x-mbx-apikey", "test-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::UrlEncoded("timestamp".into(), "1591702613943".into()),
            Matcher::UrlEncoded(
                "signature".into(),
                "3694879045b8071b7b94882ec6c5c4332da0a384d10f80c157f495f3055770d3".into(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "accountAlias": "SgsR",
                "asset": "USDT",
                "balance": "122607.35137903",
                "crossWalletBalance": "23.72469206",
                "crossUnPnl": "0.00000000",
                "availableBalance": "23.72469206",
                "maxWithdrawAmount": "23.72469206",
                "marginAvailable": true,
                "updateTime": 1617939110373
            }]"#,
        )
        .create_async()
        .await;

    let mut client = BinanceClient::new(Credentials::new("test-key", "test-secret")).unwrap();
    client.set_base_url(server.url());
    client.set_timestamp_provider(|| 1_591_702_613_943);
    let account = BinanceAccount::new(client);

    let balances = account.get_balance().await.unwrap();

    assert_eq!(balances[0].asset, "USDT");
    mock.assert_async().await;
}

#[tokio::test]
async fn trade_get_open_algo_orders_uses_signed_account_level_endpoint() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/fapi/v1/openAlgoOrders")
        .match_header("x-mbx-apikey", "test-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::UrlEncoded("timestamp".into(), "1591702613943".into()),
            Matcher::UrlEncoded(
                "signature".into(),
                "3694879045b8071b7b94882ec6c5c4332da0a384d10f80c157f495f3055770d3".into(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create_async()
        .await;

    let mut client = BinanceClient::new(Credentials::new("test-key", "test-secret")).unwrap();
    client.set_base_url(server.url());
    client.set_timestamp_provider(|| 1_591_702_613_943);
    let trade = BinanceTrade::new(client);

    let orders = trade.get_open_algo_orders(None).await.unwrap();

    assert_eq!(orders, serde_json::json!([]));
    mock.assert_async().await;
}

#[tokio::test]
async fn trade_get_open_algo_orders_preserves_optional_symbol_filter() {
    let expected_signature = generate_signature(
        "test-secret",
        "symbol=BTCUSDT&recvWindow=5000&timestamp=1591702613943",
    )
    .unwrap();
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/fapi/v1/openAlgoOrders")
        .match_header("x-mbx-apikey", "test-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::UrlEncoded("timestamp".into(), "1591702613943".into()),
            Matcher::UrlEncoded("signature".into(), expected_signature),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create_async()
        .await;

    let mut client = BinanceClient::new(Credentials::new("test-key", "test-secret")).unwrap();
    client.set_base_url(server.url());
    client.set_timestamp_provider(|| 1_591_702_613_943);
    let trade = BinanceTrade::new(client);

    trade.get_open_algo_orders(Some("BTCUSDT")).await.unwrap();

    mock.assert_async().await;
}

#[tokio::test]
async fn market_get_server_time_uses_public_time_endpoint() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/fapi/v1/time")
        .match_query(Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"serverTime":1499827319559}"#)
        .create_async()
        .await;

    let mut client = BinanceClient::new_public().unwrap();
    client.set_base_url(server.url());
    let market = BinanceMarket::new(client);

    let time = market.get_server_time().await.unwrap();

    assert_eq!(time.server_time, 1_499_827_319_559);
    mock.assert_async().await;
}

#[test]
fn market_open_request_does_not_attach_conditional_fields_by_default() {
    let params = NewOrderRequest::market("ETHUSDT", "BUY", "0.01")
        .with_position_side("LONG")
        .with_new_client_order_id("open-1")
        .to_params();

    for conditional_field in [
        "algoType",
        "triggerPrice",
        "stopPrice",
        "closePosition",
        "workingType",
        "priceProtect",
        "clientAlgoId",
    ] {
        assert!(!params.iter().any(|(key, _)| *key == conditional_field));
    }
}

#[test]
fn new_order_request_preserves_standalone_conditional_order_fields() {
    let params = NewOrderRequest::stop_market("ETHUSDT", "SELL", "2200")
        .with_position_side("LONG")
        .with_close_position(true)
        .with_working_type("MARK_PRICE")
        .with_price_protect(true)
        .with_new_client_order_id("stop-1")
        .to_params();

    assert!(params.contains(&("type", "STOP_MARKET".to_string())));
    assert!(params.contains(&("stopPrice", "2200".to_string())));
    assert!(params.contains(&("closePosition", "true".to_string())));
    assert!(params.contains(&("workingType", "MARK_PRICE".to_string())));
    assert!(params.contains(&("priceProtect", "true".to_string())));
    assert!(params.contains(&("newClientOrderId", "stop-1".to_string())));
}
