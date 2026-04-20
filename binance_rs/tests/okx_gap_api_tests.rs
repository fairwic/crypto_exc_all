use binance_rs::api::account::BinanceAccount;
use binance_rs::api::market::{BinanceMarket, FuturesDataRequest};
use binance_rs::api::trade::{
    BatchOrdersRequest, BinanceTrade, CancelMultipleOrdersRequest, ChangeMarginTypeRequest,
    ChangeMultiAssetsModeRequest, ChangePositionModeRequest, ModifyOrderRequest,
    ModifyPositionMarginRequest, NewOrderRequest, OrderIdRequest,
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
async fn trade_wrappers_cover_batch_modify_cancel_and_settings_endpoints() {
    let mut server = Server::new_async().await;
    let batch_orders = server
        .mock("POST", "/fapi/v1/batchOrders")
        .match_header("x-mbx-apikey", "test-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::Regex("(^|&)batchOrders=".into()),
            Matcher::Regex("BTCUSDT".into()),
            Matcher::Regex("quantity".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","orderId":1,"status":"NEW"}]"#)
        .create_async()
        .await;
    let modify = server
        .mock("PUT", "/fapi/v1/order")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("side".into(), "BUY".into()),
            Matcher::UrlEncoded("quantity".into(), "0.002".into()),
            Matcher::UrlEncoded("price".into(), "9001".into()),
            Matcher::UrlEncoded("orderId".into(), "1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","orderId":1,"status":"NEW"}"#)
        .create_async()
        .await;
    let batch_modify = server
        .mock("PUT", "/fapi/v1/batchOrders")
        .match_query(Matcher::AllOf(vec![
            Matcher::Regex("(^|&)batchOrders=".into()),
            Matcher::Regex("BTCUSDT".into()),
            Matcher::Regex("9002".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","orderId":1,"status":"NEW"}]"#)
        .create_async()
        .await;
    let batch_cancel = server
        .mock("DELETE", "/fapi/v1/batchOrders")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("orderIdList".into(), "[1,2]".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","orderId":1,"status":"CANCELED"}]"#)
        .create_async()
        .await;
    let cancel_all = server
        .mock("DELETE", "/fapi/v1/allOpenOrders")
        .match_query(Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":200,"msg":"done"}"#)
        .create_async()
        .await;
    let open_order = server
        .mock("GET", "/fapi/v1/openOrder")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("orderId".into(), "1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","orderId":1,"status":"NEW"}"#)
        .create_async()
        .await;
    let margin_type = server
        .mock("POST", "/fapi/v1/marginType")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("marginType".into(), "ISOLATED".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":200,"msg":"success"}"#)
        .create_async()
        .await;
    let position_mode = server
        .mock("POST", "/fapi/v1/positionSide/dual")
        .match_query(Matcher::UrlEncoded(
            "dualSidePosition".into(),
            "true".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":200,"msg":"success"}"#)
        .create_async()
        .await;
    let multi_assets = server
        .mock("POST", "/fapi/v1/multiAssetsMargin")
        .match_query(Matcher::UrlEncoded(
            "multiAssetsMargin".into(),
            "false".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":200,"msg":"success"}"#)
        .create_async()
        .await;
    let position_margin = server
        .mock("POST", "/fapi/v1/positionMargin")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("amount".into(), "10".into()),
            Matcher::UrlEncoded("type".into(), "1".into()),
            Matcher::UrlEncoded("positionSide".into(), "LONG".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"amount":10,"code":200,"msg":"ok","type":1}"#)
        .create_async()
        .await;

    let trade = BinanceTrade::new(signed_client(server.url()));
    let order = NewOrderRequest::limit("BTCUSDT", "BUY", "0.001", "9000", "GTC");
    let modify_request =
        ModifyOrderRequest::new("BTCUSDT", "BUY", "0.002", "9001").with_order_id(1);

    assert_eq!(
        trade
            .place_multiple_orders(BatchOrdersRequest::new(vec![order]))
            .await
            .unwrap()[0]["status"],
        "NEW"
    );
    assert_eq!(
        trade.modify_order(modify_request).await.unwrap()["orderId"],
        1
    );
    assert_eq!(
        trade
            .modify_multiple_orders(BatchOrdersRequest::new(vec![
                ModifyOrderRequest::new("BTCUSDT", "BUY", "0.002", "9002").with_order_id(1)
            ]))
            .await
            .unwrap()[0]["status"],
        "NEW"
    );
    assert_eq!(
        trade
            .cancel_multiple_orders(
                CancelMultipleOrdersRequest::new("BTCUSDT").with_order_ids(vec![1, 2,])
            )
            .await
            .unwrap()[0]["status"],
        "CANCELED"
    );
    assert_eq!(
        trade.cancel_all_open_orders("BTCUSDT").await.unwrap()["code"],
        200
    );
    assert_eq!(
        trade
            .get_open_order(OrderIdRequest::new("BTCUSDT").with_order_id(1))
            .await
            .unwrap()["status"],
        "NEW"
    );
    assert_eq!(
        trade
            .change_margin_type(ChangeMarginTypeRequest::new("BTCUSDT", "ISOLATED"))
            .await
            .unwrap()["code"],
        200
    );
    assert_eq!(
        trade
            .change_position_mode(ChangePositionModeRequest::new(true))
            .await
            .unwrap()["code"],
        200
    );
    assert_eq!(
        trade
            .change_multi_assets_mode(ChangeMultiAssetsModeRequest::new(false))
            .await
            .unwrap()["code"],
        200
    );
    assert_eq!(
        trade
            .modify_position_margin(
                ModifyPositionMarginRequest::new("BTCUSDT", "10", 1).with_position_side("LONG")
            )
            .await
            .unwrap()["code"],
        200
    );

    batch_orders.assert_async().await;
    modify.assert_async().await;
    batch_modify.assert_async().await;
    batch_cancel.assert_async().await;
    cancel_all.assert_async().await;
    open_order.assert_async().await;
    margin_type.assert_async().await;
    position_mode.assert_async().await;
    multi_assets.assert_async().await;
    position_margin.assert_async().await;
}

#[tokio::test]
async fn account_wrappers_cover_configuration_limits_and_commission_endpoints() {
    let mut server = Server::new_async().await;
    let account_config = server
        .mock("GET", "/fapi/v1/accountConfig")
        .match_header("x-mbx-apikey", "test-key")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"canTrade":true,"dualSidePosition":true}"#)
        .create_async()
        .await;
    let symbol_config = server
        .mock("GET", "/fapi/v1/symbolConfig")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("timestamp".into(), "1591702613943".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","leverage":20}]"#)
        .create_async()
        .await;
    let order_rate_limit = server
        .mock("GET", "/fapi/v1/rateLimit/order")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"rateLimitType":"ORDERS","interval":"MINUTE","limit":1200}]"#)
        .create_async()
        .await;
    let leverage_bracket = server
        .mock("GET", "/fapi/v1/leverageBracket")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("timestamp".into(), "1591702613943".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","brackets":[]}"#)
        .create_async()
        .await;
    let position_mode = server
        .mock("GET", "/fapi/v1/positionSide/dual")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"dualSidePosition":true}"#)
        .create_async()
        .await;
    let multi_assets = server
        .mock("GET", "/fapi/v1/multiAssetsMargin")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"multiAssetsMargin":false}"#)
        .create_async()
        .await;
    let commission = server
        .mock("GET", "/fapi/v1/commissionRate")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("timestamp".into(), "1591702613943".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","makerCommissionRate":"0.0002"}"#)
        .create_async()
        .await;

    let account = BinanceAccount::new(signed_client(server.url()));

    assert_eq!(
        account.get_account_config().await.unwrap()["canTrade"],
        true
    );
    assert_eq!(
        account.get_symbol_config(Some("BTCUSDT")).await.unwrap()[0]["leverage"],
        20
    );
    assert_eq!(
        account.get_order_rate_limit().await.unwrap()[0]["rateLimitType"],
        "ORDERS"
    );
    assert_eq!(
        account
            .get_leverage_brackets(Some("BTCUSDT"))
            .await
            .unwrap()["symbol"],
        "BTCUSDT"
    );
    assert_eq!(
        account.get_position_mode().await.unwrap()["dualSidePosition"],
        true
    );
    assert_eq!(
        account.get_multi_assets_mode().await.unwrap()["multiAssetsMargin"],
        false
    );
    assert_eq!(
        account.get_commission_rate("BTCUSDT").await.unwrap()["symbol"],
        "BTCUSDT"
    );

    account_config.assert_async().await;
    symbol_config.assert_async().await;
    order_rate_limit.assert_async().await;
    leverage_bracket.assert_async().await;
    position_mode.assert_async().await;
    multi_assets.assert_async().await;
    commission.assert_async().await;
}

#[tokio::test]
async fn market_wrappers_cover_mark_price_open_interest_and_trader_data_endpoints() {
    let mut server = Server::new_async().await;
    let mark_price = server
        .mock("GET", "/fapi/v1/premiumIndex")
        .match_query(Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","markPrice":"100"}"#)
        .create_async()
        .await;
    let open_interest = server
        .mock("GET", "/fapi/v1/openInterest")
        .match_query(Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"symbol":"BTCUSDT","openInterest":"10"}"#)
        .create_async()
        .await;
    let oi_hist = server
        .mock("GET", "/futures/data/openInterestHist")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("period".into(), "5m".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","sumOpenInterest":"10"}]"#)
        .create_async()
        .await;
    let top_position = server
        .mock("GET", "/futures/data/topLongShortPositionRatio")
        .match_query(Matcher::UrlEncoded("period".into(), "5m".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","longShortRatio":"1.2"}]"#)
        .create_async()
        .await;
    let top_account = server
        .mock("GET", "/futures/data/topLongShortAccountRatio")
        .match_query(Matcher::UrlEncoded("period".into(), "5m".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","longShortRatio":"1.3"}]"#)
        .create_async()
        .await;
    let global_ratio = server
        .mock("GET", "/futures/data/globalLongShortAccountRatio")
        .match_query(Matcher::UrlEncoded("period".into(), "5m".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"symbol":"BTCUSDT","longShortRatio":"1.4"}]"#)
        .create_async()
        .await;
    let taker_volume = server
        .mock("GET", "/futures/data/takerlongshortRatio")
        .match_query(Matcher::UrlEncoded("period".into(), "5m".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"buySellRatio":"1.5","buyVol":"2"}]"#)
        .create_async()
        .await;

    let market = BinanceMarket::new(public_client(server.url()));
    let request = FuturesDataRequest::new("BTCUSDT", "5m").with_limit(2);

    assert_eq!(
        market.get_mark_price(Some("BTCUSDT")).await.unwrap()["symbol"],
        "BTCUSDT"
    );
    assert_eq!(
        market.get_open_interest("BTCUSDT").await.unwrap()["openInterest"],
        "10"
    );
    assert_eq!(
        market
            .get_open_interest_statistics(request.clone())
            .await
            .unwrap()[0]["symbol"],
        "BTCUSDT"
    );
    assert_eq!(
        market
            .get_top_long_short_position_ratio(request.clone())
            .await
            .unwrap()[0]["longShortRatio"],
        "1.2"
    );
    assert_eq!(
        market
            .get_top_long_short_account_ratio(request.clone())
            .await
            .unwrap()[0]["longShortRatio"],
        "1.3"
    );
    assert_eq!(
        market
            .get_global_long_short_account_ratio(request.clone())
            .await
            .unwrap()[0]["longShortRatio"],
        "1.4"
    );
    assert_eq!(
        market.get_taker_buy_sell_volume(request).await.unwrap()[0]["buySellRatio"],
        "1.5"
    );

    mark_price.assert_async().await;
    open_interest.assert_async().await;
    oi_hist.assert_async().await;
    top_position.assert_async().await;
    top_account.assert_async().await;
    global_ratio.assert_async().await;
    taker_volume.assert_async().await;
}
