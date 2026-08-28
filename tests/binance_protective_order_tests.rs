use crypto_exc_all::{
    BinanceExchangeConfig, CancelOrderRequest, CryptoSdk, Error, ExchangeId, Instrument, OrderSide,
    ProtectiveOrderQuery, ProtectiveOrderRequest, ProtectiveOrderWorkingType, SdkConfig,
};
use mockito::{Matcher, Server};

#[tokio::test]
async fn places_queries_and_directly_cancels_binance_protective_order() {
    let mut server = Server::new_async().await;
    let place = server
        .mock("POST", "/fapi/v1/algoOrder")
        .match_header("x-mbx-apikey", "binance-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("algoType".into(), "CONDITIONAL".into()),
            Matcher::UrlEncoded("symbol".into(), "ETHUSDT".into()),
            Matcher::UrlEncoded("side".into(), "SELL".into()),
            Matcher::UrlEncoded("type".into(), "STOP_MARKET".into()),
            Matcher::UrlEncoded("triggerPrice".into(), "2200".into()),
            Matcher::UrlEncoded("positionSide".into(), "LONG".into()),
            Matcher::UrlEncoded("closePosition".into(), "true".into()),
            Matcher::UrlEncoded("workingType".into(), "MARK_PRICE".into()),
            Matcher::UrlEncoded("priceProtect".into(), "true".into()),
            Matcher::UrlEncoded("clientAlgoId".into(), "sl-rqethopen3".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Any,
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(protective_order_body("NEW"))
        .create_async()
        .await;
    let query = server
        .mock("GET", "/fapi/v1/algoOrder")
        .match_header("x-mbx-apikey", "binance-key")
        .match_query(protective_order_id_query())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(protective_order_body("NEW"))
        .create_async()
        .await;
    let cancel = server
        .mock("DELETE", "/fapi/v1/algoOrder")
        .match_header("x-mbx-apikey", "binance-key")
        .match_query(protective_order_id_query())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "algoId": 2000000953242572,
                "clientAlgoId": "sl-rqethopen3",
                "code": "200",
                "msg": "success"
            }"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        okx: None,
        binance: Some(BinanceExchangeConfig {
            api_key: "binance-key".to_string(),
            api_secret: "binance-secret".to_string(),
            api_url: Some(server.url()),
            sapi_api_url: None,
            web_api_url: None,
            ws_stream_url: None,
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
        }),
        bitget: None,
        ..SdkConfig::default()
    })
    .unwrap();

    let instrument = Instrument::perp("ETH", "USDT");
    let ack = sdk
        .trade(ExchangeId::Binance)
        .unwrap()
        .place_protective_order(
            ProtectiveOrderRequest::stop_market(instrument.clone(), OrderSide::Sell, "2200")
                .with_position_side("LONG")
                .with_close_position(true)
                .with_working_type(ProtectiveOrderWorkingType::MarkPrice)
                .with_price_protect(true)
                .with_client_order_id("sl-rqethopen3"),
        )
        .await
        .unwrap();
    let order = sdk
        .orders(ExchangeId::Binance)
        .unwrap()
        .get_protective_order(ProtectiveOrderQuery::by_client_order_id(
            instrument.clone(),
            "sl-rqethopen3",
        ))
        .await
        .unwrap();
    let canceled = sdk
        .trade(ExchangeId::Binance)
        .unwrap()
        .cancel_protective_order(CancelOrderRequest::by_client_order_id(
            instrument,
            "sl-rqethopen3",
        ))
        .await
        .unwrap();

    assert_eq!(ack.order_id.as_deref(), Some("2000000953242572"));
    assert_eq!(ack.status.as_deref(), Some("NEW"));
    assert_eq!(order.status.as_deref(), Some("NEW"));
    assert_eq!(order.order_type.as_deref(), Some("STOP_MARKET"));
    assert_eq!(canceled.order_id.as_deref(), Some("2000000953242572"));
    assert_eq!(canceled.status.as_deref(), Some("200"));
    place.assert_async().await;
    query.assert_async().await;
    cancel.assert_async().await;
}

#[tokio::test]
async fn lists_all_account_level_binance_open_algo_orders_without_filtering() {
    let mut server = Server::new_async().await;
    let list = server
        .mock("GET", "/fapi/v1/openAlgoOrders")
        .match_header("x-mbx-apikey", "binance-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Any,
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[
                {
                    "algoId": 2000000953242572,
                    "clientAlgoId": "sl-rqethopen3",
                    "algoStatus": "NEW",
                    "orderType": "STOP_MARKET",
                    "symbol": "ETHUSDT",
                    "side": "SELL",
                    "positionSide": "LONG",
                    "triggerPrice": "2200",
                    "quantity": "0",
                    "closePosition": true,
                    "createTime": 1779023785699,
                    "updateTime": 1779023785700
                },
                {
                    "algoId": 2000000953242573,
                    "clientAlgoId": "provider-future-algo",
                    "algoStatus": "NEW",
                    "orderType": "PROVIDER_FUTURE_ALGO_TYPE",
                    "symbol": "BNBUSDT",
                    "side": "BUY",
                    "positionSide": "SHORT",
                    "triggerPrice": "500",
                    "quantity": "0.1",
                    "closePosition": false,
                    "createTime": 1779023785701,
                    "updateTime": 1779023785702
                }
            ]"#,
        )
        .create_async()
        .await;
    let sdk = CryptoSdk::from_config(SdkConfig {
        okx: None,
        binance: Some(BinanceExchangeConfig {
            api_key: "binance-key".to_string(),
            api_secret: "binance-secret".to_string(),
            api_url: Some(server.url()),
            sapi_api_url: None,
            web_api_url: None,
            ws_stream_url: None,
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
        }),
        bitget: None,
        ..SdkConfig::default()
    })
    .unwrap();

    let orders = sdk
        .orders(ExchangeId::Binance)
        .unwrap()
        .open_protective_orders()
        .await
        .unwrap();

    assert_eq!(orders.len(), 2);
    assert_eq!(orders[0].order_id.as_deref(), Some("2000000953242572"));
    assert_eq!(orders[0].client_order_id.as_deref(), Some("sl-rqethopen3"));
    assert_eq!(orders[0].status.as_deref(), Some("NEW"));
    assert_eq!(orders[0].order_type.as_deref(), Some("STOP_MARKET"));
    assert_eq!(orders[0].exchange_symbol, "ETHUSDT");
    assert_eq!(orders[0].side.as_deref(), Some("SELL"));
    assert_eq!(orders[0].price.as_deref(), Some("2200"));
    assert_eq!(orders[0].size.as_deref(), Some("0"));
    assert_eq!(orders[0].created_at, Some(1_779_023_785_699));
    assert_eq!(orders[0].updated_at, Some(1_779_023_785_700));
    assert_eq!(orders[0].raw["positionSide"], "LONG");
    assert_eq!(orders[0].raw["closePosition"], true);
    assert_eq!(
        orders[1].order_type.as_deref(),
        Some("PROVIDER_FUTURE_ALGO_TYPE")
    );
    list.assert_async().await;
}

#[tokio::test]
async fn places_fixed_size_hedge_protective_order_via_algo_endpoint() {
    let mut server = Server::new_async().await;
    let forbidden_close_position = server
        .mock("POST", "/fapi/v1/algoOrder")
        .match_query(Matcher::UrlEncoded("closePosition".into(), "true".into()))
        .expect(0)
        .with_status(500)
        .create_async()
        .await;
    let forbidden_disabled_close_position = server
        .mock("POST", "/fapi/v1/algoOrder")
        .match_query(Matcher::UrlEncoded("closePosition".into(), "false".into()))
        .expect(0)
        .with_status(500)
        .create_async()
        .await;
    let place = server
        .mock("POST", "/fapi/v1/algoOrder")
        .match_header("x-mbx-apikey", "binance-key")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("algoType".into(), "CONDITIONAL".into()),
            Matcher::UrlEncoded("symbol".into(), "ETHUSDT".into()),
            Matcher::UrlEncoded("side".into(), "SELL".into()),
            Matcher::UrlEncoded("type".into(), "STOP_MARKET".into()),
            Matcher::UrlEncoded("triggerPrice".into(), "2200".into()),
            Matcher::UrlEncoded("quantity".into(), "0.012".into()),
            Matcher::UrlEncoded("positionSide".into(), "LONG".into()),
            Matcher::UrlEncoded("clientAlgoId".into(), "sl-rqethfixed1".into()),
            Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
            Matcher::Any,
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "algoId": 2000000953242573,
                "clientAlgoId": "sl-rqethfixed1",
                "algoType": "CONDITIONAL",
                "orderType": "STOP_MARKET",
                "symbol": "ETHUSDT",
                "side": "SELL",
                "positionSide": "LONG",
                "quantity": "0.012",
                "algoStatus": "NEW",
                "triggerPrice": "2200",
                "closePosition": false,
                "createTime": 1779023785699,
                "updateTime": 1779023785699
            }"#,
        )
        .create_async()
        .await;
    let sdk = CryptoSdk::from_config(SdkConfig {
        okx: None,
        binance: Some(BinanceExchangeConfig {
            api_key: "binance-key".to_string(),
            api_secret: "binance-secret".to_string(),
            api_url: Some(server.url()),
            sapi_api_url: None,
            web_api_url: None,
            ws_stream_url: None,
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
        }),
        bitget: None,
        ..SdkConfig::default()
    })
    .unwrap();

    let ack = sdk
        .trade(ExchangeId::Binance)
        .unwrap()
        .place_protective_order(
            ProtectiveOrderRequest::stop_market(
                Instrument::perp("ETH", "USDT"),
                OrderSide::Sell,
                "2200",
            )
            .with_position_side("LONG")
            .with_quantity("0.012")
            .with_client_order_id("sl-rqethfixed1"),
        )
        .await
        .unwrap();

    assert_eq!(ack.order_id.as_deref(), Some("2000000953242573"));
    assert_eq!(ack.client_order_id.as_deref(), Some("sl-rqethfixed1"));
    assert_eq!(ack.status.as_deref(), Some("NEW"));
    assert_eq!(ack.raw["quantity"], "0.012");
    assert_eq!(ack.raw["positionSide"], "LONG");
    place.assert_async().await;
    forbidden_close_position.assert_async().await;
    forbidden_disabled_close_position.assert_async().await;
}

#[tokio::test]
async fn rejects_invalid_binance_protective_order_shapes_before_provider_submission() {
    let mut server = Server::new_async().await;
    let provider = server
        .mock("POST", "/fapi/v1/algoOrder")
        .expect(0)
        .with_status(200)
        .create_async()
        .await;
    let sdk = CryptoSdk::from_config(SdkConfig {
        okx: None,
        binance: Some(BinanceExchangeConfig {
            api_key: "binance-key".to_string(),
            api_secret: "binance-secret".to_string(),
            api_url: Some(server.url()),
            sapi_api_url: None,
            web_api_url: None,
            ws_stream_url: None,
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
        }),
        bitget: None,
        ..SdkConfig::default()
    })
    .unwrap();
    let trade = sdk.trade(ExchangeId::Binance).unwrap();
    let instrument = Instrument::perp("ETH", "USDT");
    let invalid_requests = [
        ProtectiveOrderRequest::stop_market(instrument.clone(), OrderSide::Sell, "2200")
            .with_position_side("LONG")
            .with_quantity("0.012")
            .with_close_position(true),
        ProtectiveOrderRequest::stop_market(instrument.clone(), OrderSide::Sell, "2200")
            .with_position_side("LONG"),
        ProtectiveOrderRequest::stop_market(instrument.clone(), OrderSide::Sell, "2200")
            .with_position_side("LONG")
            .with_close_position(false),
        ProtectiveOrderRequest::stop_market(instrument.clone(), OrderSide::Sell, "2200")
            .with_position_side("LONG")
            .with_quantity("0.012")
            .with_reduce_only(true),
        ProtectiveOrderRequest::stop_market(instrument, OrderSide::Buy, "2200")
            .with_position_side("short")
            .with_close_position(true)
            .with_reduce_only(false),
        ProtectiveOrderRequest::stop_market(
            Instrument::perp("ETH", "USDT"),
            OrderSide::Sell,
            "2200",
        )
        .with_position_side("BOTH")
        .with_close_position(true)
        .with_reduce_only(false),
        ProtectiveOrderRequest::stop_market(
            Instrument::perp("ETH", "USDT"),
            OrderSide::Buy,
            "2200",
        )
        .with_position_side("LONG")
        .with_close_position(true),
        ProtectiveOrderRequest::stop_market(
            Instrument::perp("ETH", "USDT"),
            OrderSide::Sell,
            "2200",
        )
        .with_position_side("INVALID")
        .with_quantity("0.012"),
        ProtectiveOrderRequest::stop_market(
            Instrument::perp("ETH", "USDT"),
            OrderSide::Buy,
            "2200",
        )
        .with_position_side("LONG")
        .with_quantity("0.012"),
        ProtectiveOrderRequest::stop_market(
            Instrument::perp("ETH", "USDT"),
            OrderSide::Sell,
            "2200",
        )
        .with_position_side("SHORT")
        .with_quantity("0.012"),
        ProtectiveOrderRequest::stop_market(
            Instrument::perp("ETH", "USDT"),
            OrderSide::Sell,
            "2200",
        )
        .with_position_side("BOTH")
        .with_quantity("0.012"),
    ];

    for request in invalid_requests {
        let error = trade
            .place_protective_order(request)
            .await
            .expect_err("invalid protective request must fail before provider submission");
        assert!(matches!(
            error,
            Error::Adapter {
                exchange: ExchangeId::Binance,
                ..
            }
        ));
    }
    provider.assert_async().await;
}

fn protective_order_id_query() -> Matcher {
    Matcher::AllOf(vec![
        Matcher::UrlEncoded("clientAlgoId".into(), "sl-rqethopen3".into()),
        Matcher::UrlEncoded("recvWindow".into(), "5000".into()),
        Matcher::Any,
    ])
}

fn protective_order_body(status: &str) -> String {
    format!(
        r#"{{
            "algoId": 2000000953242572,
            "clientAlgoId": "sl-rqethopen3",
            "algoType": "CONDITIONAL",
            "orderType": "STOP_MARKET",
            "symbol": "ETHUSDT",
            "side": "SELL",
            "positionSide": "LONG",
            "algoStatus": "{status}",
            "triggerPrice": "2200",
            "closePosition": true,
            "priceProtect": true,
            "createTime": 1779023785699,
            "updateTime": 1779023785699
        }}"#
    )
}
