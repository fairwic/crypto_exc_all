use binance_rs::api::account::{BinanceAccount, IncomeHistoryRequest};
use binance_rs::api::market::{BinanceMarket, FundingRateHistoryRequest, KlineRequest};
use binance_rs::api::trade::{
    BinanceTrade, ChangeLeverageRequest, NewOrderRequest, OrderIdRequest, OrderListRequest,
};
use binance_rs::client::BinanceClient;
use binance_rs::config::Credentials;
use mockito::{Matcher, Server};

fn signed_client(server_url: String) -> BinanceClient {
    let mut client = BinanceClient::new(Credentials::new("test-key", "test-secret")).unwrap();
    client.set_base_url(server_url);
    client.set_timestamp_provider(|| 1_591_702_613_943);
    client
}

fn public_client(server_url: String) -> BinanceClient {
    let mut client = BinanceClient::new_public().unwrap();
    client.set_base_url(server_url);
    client
}

#[tokio::test]
async fn market_wrappers_map_core_public_endpoints() {
    let mut server = Server::new_async().await;
    let exchange_info = server
        .mock("GET", "/fapi/v1/exchangeInfo")
        .match_query(Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"timezone":"UTC","symbols":[]}"#)
        .create_async()
        .await;
    let depth = server
        .mock("GET", "/fapi/v1/depth")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("limit".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"lastUpdateId":1,"bids":[],"asks":[]}"#)
        .create_async()
        .await;
    let klines = server
        .mock("GET", "/fapi/v1/klines")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "1h".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[[1499040000000,"1","2","0.5","1.5","10",1499043599999,"15",4,"6","9","0"]]"#)
        .create_async()
        .await;
    let ticker = server
        .mock("GET", "/fapi/v1/ticker/24hr")
        .match_query(Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","lastPrice":"50000"}"#)
        .create_async()
        .await;
    let funding = server
        .mock("GET", "/fapi/v1/fundingRate")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","fundingRate":"0.0001","fundingTime":1570636800000}]"#)
        .create_async()
        .await;

    let market = BinanceMarket::new(public_client(server.url()));

    assert_eq!(market.get_exchange_info().await.unwrap()["timezone"], "UTC");
    assert_eq!(
        market.get_depth("BTCUSDT", Some(100)).await.unwrap()["lastUpdateId"],
        1
    );
    assert_eq!(
        market
            .get_klines(KlineRequest::new("BTCUSDT", "1h").with_limit(2))
            .await
            .unwrap()[0][4],
        "1.5"
    );
    assert_eq!(
        market.get_ticker_24hr(Some("BTCUSDT")).await.unwrap()["symbol"],
        "BTCUSDT"
    );
    assert_eq!(
        market
            .get_funding_rate_history(
                FundingRateHistoryRequest::new()
                    .with_symbol("BTCUSDT")
                    .with_limit(2)
            )
            .await
            .unwrap()[0]["fundingRate"],
        "0.0001"
    );

    exchange_info.assert_async().await;
    depth.assert_async().await;
    klines.assert_async().await;
    ticker.assert_async().await;
    funding.assert_async().await;
}

#[tokio::test]
async fn account_wrappers_map_signed_account_position_and_income_endpoints() {
    let mut server = Server::new_async().await;
    let account_info = server
        .mock("GET", "/fapi/v3/account")
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
        .with_body(r#"{"availableBalance":"100","assets":[],"positions":[]}"#)
        .create_async()
        .await;
    let positions = server
        .mock("GET", "/fapi/v3/positionRisk")
        .match_header("x-mbx-apikey", "test-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::UrlEncoded("timestamp".into(), "1591702613943".into()),
            Matcher::UrlEncoded(
                "signature".into(),
                "0772b44cc897fad6e309db9eb1e76172c8a9d16e896656ed429026aec229571b".into(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","positionAmt":"0"}]"#)
        .create_async()
        .await;
    let income = server
        .mock("GET", "/fapi/v1/income")
        .match_header("x-mbx-apikey", "test-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("incomeType".into(), "REALIZED_PNL".into()),
            Matcher::UrlEncoded("limit".into(), "10".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::UrlEncoded("timestamp".into(), "1591702613943".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","incomeType":"REALIZED_PNL","income":"1"}]"#)
        .create_async()
        .await;

    let account = BinanceAccount::new(signed_client(server.url()));

    assert_eq!(
        account.get_account_info().await.unwrap()["availableBalance"],
        "100"
    );
    assert_eq!(
        account.get_positions(Some("BTCUSDT")).await.unwrap()[0]["symbol"],
        "BTCUSDT"
    );
    assert_eq!(
        account
            .get_income_history(
                IncomeHistoryRequest::new()
                    .with_symbol("BTCUSDT")
                    .with_income_type("REALIZED_PNL")
                    .with_limit(10),
            )
            .await
            .unwrap()[0]["income"],
        "1"
    );

    account_info.assert_async().await;
    positions.assert_async().await;
    income.assert_async().await;
}

#[tokio::test]
async fn trade_wrappers_map_core_order_and_trade_endpoints() {
    let mut server = Server::new_async().await;
    let new_order = server
        .mock("POST", "/fapi/v1/order")
        .match_header("x-mbx-apikey", "test-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("side".into(), "BUY".into()),
            Matcher::UrlEncoded("type".into(), "LIMIT".into()),
            Matcher::UrlEncoded("timeInForce".into(), "GTC".into()),
            Matcher::UrlEncoded("quantity".into(), "0.001".into()),
            Matcher::UrlEncoded("price".into(), "9000".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::UrlEncoded("timestamp".into(), "1591702613943".into()),
            Matcher::UrlEncoded(
                "signature".into(),
                "52845e5979d1e02d8ae2bdc52d2fb2dab0c1a37f406adc57cdf1b59cba2570fc".into(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","orderId":12345,"status":"NEW"}"#)
        .create_async()
        .await;
    let test_order = server
        .mock("POST", "/fapi/v1/order/test")
        .match_header("x-mbx-apikey", "test-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("side".into(), "BUY".into()),
            Matcher::UrlEncoded("type".into(), "LIMIT".into()),
            Matcher::UrlEncoded("timeInForce".into(), "GTC".into()),
            Matcher::UrlEncoded("quantity".into(), "0.001".into()),
            Matcher::UrlEncoded("price".into(), "9000".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{}"#)
        .create_async()
        .await;
    let query_order = server
        .mock("GET", "/fapi/v1/order")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("orderId".into(), "12345".into()),
            Matcher::UrlEncoded(
                "signature".into(),
                "7438194fbeca64c52b9749d3962536dfbcc87d05a2ea09ae712daf334aead3a0".into(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","orderId":12345}"#)
        .create_async()
        .await;
    let cancel = server
        .mock("DELETE", "/fapi/v1/order")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("orderId".into(), "12345".into()),
            Matcher::UrlEncoded(
                "signature".into(),
                "7438194fbeca64c52b9749d3962536dfbcc87d05a2ea09ae712daf334aead3a0".into(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","orderId":12345,"status":"CANCELED"}"#)
        .create_async()
        .await;
    let open_orders = server
        .mock("GET", "/fapi/v1/openOrders")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded(
                "signature".into(),
                "0772b44cc897fad6e309db9eb1e76172c8a9d16e896656ed429026aec229571b".into(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","orderId":12345}]"#)
        .create_async()
        .await;
    let all_orders = server
        .mock("GET", "/fapi/v1/allOrders")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("limit".into(), "10".into()),
            Matcher::UrlEncoded(
                "signature".into(),
                "1a95a80a25a5485f6003cbd8c75780aeac2d16a68879fbbb97254683ab5de327".into(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","orderId":12345}]"#)
        .create_async()
        .await;
    let trades = server
        .mock("GET", "/fapi/v1/userTrades")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("limit".into(), "10".into()),
            Matcher::UrlEncoded(
                "signature".into(),
                "1a95a80a25a5485f6003cbd8c75780aeac2d16a68879fbbb97254683ab5de327".into(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","id":1}]"#)
        .create_async()
        .await;
    let leverage = server
        .mock("POST", "/fapi/v1/leverage")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("leverage".into(), "20".into()),
            Matcher::UrlEncoded(
                "signature".into(),
                "56e05955fd940fe4683b5dadc4266c359c1f0dff8fb827393bf8ddf29225fbbc".into(),
            ),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","leverage":20}"#)
        .create_async()
        .await;

    let trade = BinanceTrade::new(signed_client(server.url()));
    let order = NewOrderRequest::limit("BTCUSDT", "BUY", "0.001", "9000", "GTC");

    assert_eq!(
        trade.place_order(order.clone()).await.unwrap()["status"],
        "NEW"
    );
    assert_eq!(
        trade.test_order(order).await.unwrap(),
        serde_json::json!({})
    );
    assert_eq!(
        trade
            .get_order(OrderIdRequest::new("BTCUSDT").with_order_id(12345))
            .await
            .unwrap()["orderId"],
        12345
    );
    assert_eq!(
        trade
            .cancel_order(OrderIdRequest::new("BTCUSDT").with_order_id(12345))
            .await
            .unwrap()["status"],
        "CANCELED"
    );
    assert_eq!(
        trade.get_open_orders(Some("BTCUSDT")).await.unwrap()[0]["orderId"],
        12345
    );
    assert_eq!(
        trade
            .get_all_orders(OrderListRequest::new("BTCUSDT").with_limit(10))
            .await
            .unwrap()[0]["orderId"],
        12345
    );
    assert_eq!(
        trade
            .get_user_trades(OrderListRequest::new("BTCUSDT").with_limit(10))
            .await
            .unwrap()[0]["id"],
        1
    );
    assert_eq!(
        trade
            .change_leverage(ChangeLeverageRequest::new("BTCUSDT", 20))
            .await
            .unwrap()["leverage"],
        20
    );

    new_order.assert_async().await;
    test_order.assert_async().await;
    query_order.assert_async().await;
    cancel.assert_async().await;
    open_orders.assert_async().await;
    all_orders.assert_async().await;
    trades.assert_async().await;
    leverage.assert_async().await;
}
