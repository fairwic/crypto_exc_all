use crypto_exc_all::{
    BinanceExchangeConfig, BitgetExchangeConfig, BybitExchangeConfig, CryptoSdk, ExchangeId,
    GateExchangeConfig, OkxExchangeConfig, PlatformEventQuery, SdkConfig,
};
use mockito::Server;

#[tokio::test]
async fn external_consumer_uses_root_crate_for_bybit_platform_events() {
    let mut bybit_server = Server::new_async().await;
    let system_status = bybit_server
        .mock("GET", "/v5/system/status?id=maint-1&state=ongoing")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{"list":[{
                    "id":"maint-1",
                    "title":"Linear perpetual maintenance",
                    "state":"ongoing",
                    "begin":"1700000000000",
                    "end":"1700003600000"
                }]}
            }"#,
        )
        .create_async()
        .await;
    let announcements = bybit_server
        .mock(
            "GET",
            "/v5/announcements/index?limit=1&locale=en-US&page=2&tag=Derivatives&type=maintenance",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{"list":[{
                    "id":"ann-1",
                    "title":"Derivatives maintenance notice",
                    "url":"https://ann.example/bybit/ann-1",
                    "dateTimestamp":"1700000100000"
                }]}
            }"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        bybit: Some(BybitExchangeConfig {
            api_key: "bybit-key".to_string(),
            api_secret: "bybit-secret".to_string(),
            api_url: Some(bybit_server.url()),
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
            category: Some("linear".to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();

    let platform = sdk.platform(ExchangeId::Bybit).unwrap();
    let statuses = platform
        .system_status(
            PlatformEventQuery::new()
                .with_id("maint-1")
                .with_state("ongoing"),
        )
        .await
        .unwrap();
    let notices = platform
        .announcements(
            PlatformEventQuery::new()
                .with_locale("en-US")
                .with_event_type("maintenance")
                .with_tag("Derivatives")
                .with_page(2)
                .with_limit(1),
        )
        .await
        .unwrap();

    assert_eq!(statuses[0].event_id.as_deref(), Some("maint-1"));
    assert_eq!(statuses[0].event_type, "system_status");
    assert_eq!(
        statuses[0].title.as_deref(),
        Some("Linear perpetual maintenance")
    );
    assert_eq!(statuses[0].status.as_deref(), Some("ongoing"));
    assert_eq!(statuses[0].start_time, Some(1_700_000_000_000));
    assert_eq!(statuses[0].end_time, Some(1_700_003_600_000));

    assert_eq!(notices[0].event_id.as_deref(), Some("ann-1"));
    assert_eq!(notices[0].event_type, "announcement");
    assert_eq!(
        notices[0].title.as_deref(),
        Some("Derivatives maintenance notice")
    );
    assert_eq!(
        notices[0].url.as_deref(),
        Some("https://ann.example/bybit/ann-1")
    );
    assert_eq!(notices[0].published_at, Some(1_700_000_100_000));

    system_status.assert_async().await;
    announcements.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_okx_platform_events() {
    let mut okx_server = Server::new_async().await;
    let status = okx_server
        .mock("GET", "/api/v5/public/status")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "title":"Trading service maintenance",
                    "state":"ongoing",
                    "begin":"1700000000000",
                    "end":"1700003600000",
                    "href":"https://ann.example/okx/status",
                    "serviceType":"1",
                    "system":"okx-maint-1"
                }]
            }"#,
        )
        .create_async()
        .await;
    let announcements = okx_server
        .mock("GET", "/api/v5/support/announcements")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"0",
                "msg":"",
                "data":[{
                    "details":[{
                        "annType":"maintenance",
                        "pTime":"1700000100000",
                        "title":"OKX maintenance notice",
                        "url":"https://ann.example/okx/ann-1"
                    }],
                    "totalPage":"1"
                }]
            }"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        okx: Some(OkxExchangeConfig {
            api_key: "okx-key".to_string(),
            api_secret: "okx-secret".to_string(),
            passphrase: "okx-pass".to_string(),
            simulated: true,
            api_url: Some(okx_server.url()),
            request_expiration_ms: Some(1_000),
        }),
        ..SdkConfig::default()
    })
    .unwrap();

    let platform = sdk.platform(ExchangeId::Okx).unwrap();
    let statuses = platform
        .system_status(PlatformEventQuery::new())
        .await
        .unwrap();
    let notices = platform
        .announcements(
            PlatformEventQuery::new()
                .with_event_type("maintenance")
                .with_page(1),
        )
        .await
        .unwrap();

    assert_eq!(statuses[0].event_id.as_deref(), Some("okx-maint-1"));
    assert_eq!(statuses[0].event_type, "system_status");
    assert_eq!(
        statuses[0].title.as_deref(),
        Some("Trading service maintenance")
    );
    assert_eq!(statuses[0].status.as_deref(), Some("ongoing"));
    assert_eq!(
        statuses[0].url.as_deref(),
        Some("https://ann.example/okx/status")
    );
    assert_eq!(statuses[0].start_time, Some(1_700_000_000_000));

    assert_eq!(notices[0].event_type, "announcement");
    assert_eq!(notices[0].title.as_deref(), Some("OKX maintenance notice"));
    assert_eq!(
        notices[0].url.as_deref(),
        Some("https://ann.example/okx/ann-1")
    );
    assert_eq!(notices[0].published_at, Some(1_700_000_100_000));

    status.assert_async().await;
    announcements.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_binance_and_bitget_announcements() {
    let mut binance_server = Server::new_async().await;
    let binance = binance_server
        .mock(
            "GET",
            "/bapi/composite/v1/public/cms/article/list/query?type=1&catalogId=48&pageNo=2&pageSize=1",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"000000",
                "message":null,
                "data":{"articles":[{"id":1001,"title":"Binance maintenance notice","releaseDate":1700000200000,"url":"https://ann.example/binance/1001"}]}
            }"#,
        )
        .create_async()
        .await;

    let mut bitget_server = Server::new_async().await;
    let bitget = bitget_server
        .mock(
            "GET",
            "/api/v2/public/annoucements?annType=maintenance&language=en-US&limit=1",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"announcements":[{"annId":"bg-ann-1","annTitle":"Bitget maintenance notice","annUrl":"https://ann.example/bitget/bg-ann-1","cTime":"1700000300000","annType":"maintenance"}]}}"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        binance: Some(BinanceExchangeConfig {
            api_key: "binance-key".to_string(),
            api_secret: "binance-secret".to_string(),
            api_url: Some("http://127.0.0.1:1".to_string()),
            sapi_api_url: None,
            web_api_url: Some(binance_server.url()),
            ws_stream_url: None,
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
        }),
        bitget: Some(BitgetExchangeConfig {
            api_key: "bitget-key".to_string(),
            api_secret: "bitget-secret".to_string(),
            passphrase: "bitget-pass".to_string(),
            api_url: Some(bitget_server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            product_type: Some("USDT-FUTURES".to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();

    let binance_notices = sdk
        .platform(ExchangeId::Binance)
        .unwrap()
        .announcements(PlatformEventQuery::new().with_page(2).with_limit(1))
        .await
        .unwrap();
    let bitget_notices = sdk
        .platform(ExchangeId::Bitget)
        .unwrap()
        .announcements(
            PlatformEventQuery::new()
                .with_locale("en-US")
                .with_event_type("maintenance")
                .with_limit(1),
        )
        .await
        .unwrap();

    assert_eq!(binance_notices[0].event_id.as_deref(), Some("1001"));
    assert_eq!(
        binance_notices[0].title.as_deref(),
        Some("Binance maintenance notice")
    );
    assert_eq!(binance_notices[0].published_at, Some(1_700_000_200_000));

    assert_eq!(bitget_notices[0].event_id.as_deref(), Some("bg-ann-1"));
    assert_eq!(
        bitget_notices[0].title.as_deref(),
        Some("Bitget maintenance notice")
    );
    assert_eq!(bitget_notices[0].published_at, Some(1_700_000_300_000));

    binance.assert_async().await;
    bitget.assert_async().await;
}

#[tokio::test]
async fn unsupported_exchange_platform_events_return_explicit_unsupported() {
    let sdk = CryptoSdk::from_config(SdkConfig {
        gate: Some(GateExchangeConfig {
            api_key: "gate-key".to_string(),
            api_secret: "gate-secret".to_string(),
            api_url: Some("http://127.0.0.1:1".to_string()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            settle: Some("usdt".to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();

    let error = sdk
        .platform(ExchangeId::Gate)
        .unwrap()
        .announcements(PlatformEventQuery::new())
        .await
        .expect_err("Gate platform announcements should be explicit unsupported");

    assert!(error.to_string().contains("gate"));
    assert!(error.to_string().contains("platform announcements"));
}
