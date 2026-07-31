use binance_rs::Error;
use binance_rs::api::market::{BinanceMarket, KlineRequest};
use binance_rs::client::{BinanceClient, BinancePublicFailureKind, BinancePublicRequestFailure};
use binance_rs::dto::market::BinanceWireDecimal;
use mockito::{Matcher, Server};

/// 构造只持有匿名 transport 的 provider Market client。
fn public_market(server_url: String) -> BinanceMarket {
    let mut client = BinanceClient::new_public().expect("public client should build");
    client.set_base_url(server_url);
    BinanceMarket::new(client)
}

/// 从 provider SDK 错误中提取结构化 public failure，避免测试解析字符串。
fn public_failure(error: Error) -> BinancePublicRequestFailure {
    match error {
        Error::BinancePublicRequestFailed { failure } => *failure,
        other => panic!("unexpected error: {other:?}"),
    }
}

/// Typed K 线必须忠实传递全部查询参数，并保留标准列、扩展列与同次 HTTP 证据。
#[tokio::test]
async fn typed_klines_preserve_query_wire_fields_extensions_and_http_evidence() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/fapi/v1/klines")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "1m".into()),
            Matcher::UrlEncoded("startTime".into(), "1720000000000".into()),
            Matcher::UrlEncoded("endTime".into(), "1720000059999".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .match_header("x-mbx-apikey", Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_header("x-mbx-used-weight-1m", "2")
        .with_body(
            r#"[
                [
                    1720000000000,
                    "0.10000000000000000001",
                    1e-8,
                    "0.09",
                    "0.095",
                    "123.45",
                    1720000059999,
                    "12.34",
                    42,
                    "60",
                    "6",
                    "0",
                    {"futureColumn": true}
                ]
            ]"#,
        )
        .create_async()
        .await;

    let response = public_market(server.url())
        .get_klines_typed(
            KlineRequest::new("BTCUSDT", "1m")
                .with_start_time(1_720_000_000_000)
                .with_end_time(1_720_000_059_999)
                .with_limit(2),
        )
        .await
        .expect("typed Kline response should decode");

    request.assert_async().await;
    assert_eq!(response.evidence.http_status, 200);
    assert_eq!(
        response
            .evidence
            .used_weight_headers
            .get("x-mbx-used-weight-1m")
            .map(String::as_str),
        Some("2")
    );
    let row = &response.data[0];
    assert_eq!(row.open_time, 1_720_000_000_000);
    assert_eq!(row.close_time, 1_720_000_059_999);
    assert_eq!(row.trade_count, 42);
    assert_eq!(
        row.open,
        BinanceWireDecimal::Text("0.10000000000000000001".to_owned())
    );
    match &row.high {
        BinanceWireDecimal::Number(value) => assert_eq!(value.to_string(), "1e-8"),
        other => panic!("expected arbitrary-precision number, got {other:?}"),
    }
    assert_eq!(row.extra.len(), 1);
    assert_eq!(row.extra[0]["futureColumn"], true);
}

/// 标准 12 列缺任意一列都必须整批失败，不能由空字符串或 None 掩盖损坏行。
#[tokio::test]
async fn short_kline_row_fails_the_whole_typed_response() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/fapi/v1/klines")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "1m".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[[1720000000000,"1","2","0.5","1.5","10",1720000059999,"15",4,"6","9"]]"#)
        .create_async()
        .await;

    let failure = public_failure(
        public_market(server.url())
            .get_klines_typed(KlineRequest::new("BTCUSDT", "1m"))
            .await
            .expect_err("short row must fail closed"),
    );

    request.assert_async().await;
    assert_eq!(failure.kind, BinancePublicFailureKind::Decode);
    assert_eq!(
        failure
            .evidence
            .as_ref()
            .map(|evidence| evidence.http_status),
        Some(200)
    );
}

/// 明确非法的本地请求必须在网络前失败；不确定的 provider 最大页长不在 SDK 猜测。
#[tokio::test]
async fn zero_limit_is_rejected_before_network_io() {
    let error = public_market("http://127.0.0.1:9".to_owned())
        .get_klines_typed(KlineRequest::new("BTCUSDT", "1m").with_limit(0))
        .await
        .expect_err("zero limit must be rejected locally");

    assert!(matches!(error, Error::InvalidRequest(_)));
}

/// Symbol 与 interval 是 provider identity；空白输入不能靠 URL 编码后交给 provider 猜测。
#[tokio::test]
async fn blank_symbol_or_interval_is_rejected_before_network_io() {
    for request in [
        KlineRequest::new("BTC USDT", "1m"),
        KlineRequest::new("BTCUSDT", " "),
    ] {
        let error = public_market("http://127.0.0.1:9".to_owned())
            .get_klines_typed(request)
            .await
            .expect_err("blank identity must be rejected locally");

        assert!(matches!(error, Error::InvalidRequest(_)));
    }
}

/// 文档未冻结稳定最大值时，SDK 必须原样传递正数 limit，不能自行 clamp。
#[tokio::test]
async fn positive_limit_is_forwarded_without_an_sdk_maximum() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/fapi/v1/klines")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "1m".into()),
            Matcher::UrlEncoded("limit".into(), "1501".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create_async()
        .await;

    let response = public_market(server.url())
        .get_klines_typed(KlineRequest::new("BTCUSDT", "1m").with_limit(1501))
        .await
        .expect("positive provider limit should be forwarded");

    request.assert_async().await;
    assert!(response.data.is_empty());
}

/// 限频响应只请求一次，并保留 provider code、HTTP 状态与 Retry-After。
#[tokio::test]
async fn typed_klines_preserve_429_evidence_without_retrying() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/fapi/v1/klines")
        .expect(1)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("symbol".into(), "BTCUSDT".into()),
            Matcher::UrlEncoded("interval".into(), "1m".into()),
        ]))
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_header("x-mbx-used-weight-1m", "2400")
        .with_header("retry-after", "2")
        .with_body(r#"{"code":-1003,"msg":"Too many requests."}"#)
        .create_async()
        .await;

    let failure = public_failure(
        public_market(server.url())
            .get_klines_typed(KlineRequest::new("BTCUSDT", "1m"))
            .await
            .expect_err("429 must remain structured"),
    );

    request.assert_async().await;
    let evidence = failure.evidence.expect("HTTP response must carry evidence");
    assert_eq!(failure.kind, BinancePublicFailureKind::Provider);
    assert_eq!(failure.provider_code, Some(-1003));
    assert_eq!(evidence.http_status, 429);
    assert_eq!(evidence.retry_after.as_deref(), Some("2"));
}
