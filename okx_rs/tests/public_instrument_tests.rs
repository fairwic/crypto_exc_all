#![cfg(feature = "public-market")]

use mockito::{Matcher, Server};
use okx::{
    Error, OkxPublicFailureKind, OkxPublicInstrument, OkxPublicInstruments, OkxPublicResponse,
};

const COMPLETE_SWAP_RESPONSE: &str = r#"{
    "code": "0",
    "msg": "",
    "data": [{
        "instType": "SWAP",
        "instId": "BTC-USDT-SWAP",
        "uly": "BTC-USDT",
        "instFamily": "BTC-USDT",
        "baseCcy": "",
        "quoteCcy": "",
        "settleCcy": "USDT",
        "ctVal": "0.01",
        "ctMult": "1",
        "ctValCcy": "BTC",
        "optType": "",
        "stk": "",
        "listTime": "1597026383085",
        "auctionEndTime": "",
        "expTime": "",
        "lever": "100",
        "tickSz": "1e-8",
        "lotSz": "1",
        "minSz": "1",
        "ctType": "linear",
        "alias": "",
        "state": "live",
        "ruleType": "normal",
        "instCategory": "1",
        "maxLmtSz": "100000000",
        "maxMktSz": "10000",
        "maxTwapSz": "100000000",
        "maxIcebergSz": "100000000",
        "maxTriggerSz": "100000000",
        "maxStopSz": "100000000",
        "futureRule": {"priceBand": "0.05"},
        "arbitraryPrecisionNumber": 12345678901234567890.12345678901234567890
    }]
}"#;

/// 固定 SWAP endpoint 必须匿名调用，并无损返回规则、未知字段和 quota 证据。
#[tokio::test]
async fn swap_instruments_are_anonymous_scoped_and_lossless() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/api/v5/public/instruments?instType=SWAP")
        .match_header("OK-ACCESS-KEY", Matcher::Missing)
        .match_header("OK-ACCESS-SIGN", Matcher::Missing)
        .match_header("OK-ACCESS-TIMESTAMP", Matcher::Missing)
        .match_header("OK-ACCESS-PASSPHRASE", Matcher::Missing)
        .match_header("x-simulated-trading", Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_header("x-ratelimit-limit", "20")
        .with_header("x-ratelimit-remaining", "19")
        .with_body(COMPLETE_SWAP_RESPONSE)
        .create_async()
        .await;
    let client = OkxPublicInstruments::with_base_url(server.url()).expect("public client");

    let response: OkxPublicResponse<Vec<OkxPublicInstrument>> =
        client.list_swap().await.expect("public instruments");

    request.assert_async().await;
    assert_eq!(response.evidence.http_status, 200);
    assert_eq!(response.evidence.okx_code, "0");
    assert_eq!(
        response
            .evidence
            .rate_limit_headers
            .get("x-ratelimit-remaining")
            .map(String::as_str),
        Some("19")
    );
    let instrument = response.data.first().expect("one instrument");
    assert_eq!(instrument.instrument_type, "SWAP");
    assert_eq!(instrument.instrument_id, "BTC-USDT-SWAP");
    assert_eq!(instrument.instrument_family, "BTC-USDT");
    assert_eq!(instrument.settlement_currency, "USDT");
    assert_eq!(instrument.contract_value, "0.01");
    assert_eq!(instrument.contract_value_currency, "BTC");
    assert_eq!(instrument.contract_type, "linear");
    assert_eq!(instrument.tick_size, "1e-8");
    assert_eq!(instrument.list_time_ms, "1597026383085");
    assert_eq!(instrument.rule_type, "normal");
    assert_eq!(
        instrument.extra["arbitraryPrecisionNumber"].to_string(),
        "12345678901234567890.12345678901234567890"
    );
    assert_eq!(instrument.extra["futureRule"]["priceBand"], "0.05");
}

/// 空 `data` 是合法 provider 快照；业务完整性由 Market owner 判断，SDK 不得伪造失败。
#[tokio::test]
async fn empty_instrument_data_remains_a_successful_provider_response() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/api/v5/public/instruments?instType=SWAP")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[]}"#)
        .create_async()
        .await;
    let client = OkxPublicInstruments::with_base_url(server.url()).expect("public client");

    let response = client.list_swap().await.expect("empty provider snapshot");

    request.assert_async().await;
    assert!(response.data.is_empty());
    assert_eq!(response.evidence.okx_code, "0");
}

/// 已知 identity 字段类型错误必须在 typed DTO 边界失败，并保留成功 envelope 的证据。
#[tokio::test]
async fn malformed_instrument_data_fails_with_response_evidence() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/api/v5/public/instruments?instType=SWAP")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[{"instType":"SWAP","instId":42}]}"#)
        .create_async()
        .await;
    let client = OkxPublicInstruments::with_base_url(server.url()).expect("public client");

    let error = client
        .list_swap()
        .await
        .expect_err("malformed instrument must fail");

    request.assert_async().await;
    let Error::PublicApiError(evidence) = error else {
        panic!("expected public response evidence");
    };
    assert_eq!(evidence.kind, OkxPublicFailureKind::MalformedData);
    assert_eq!(evidence.http_status, 200);
    assert_eq!(evidence.okx_code.as_deref(), Some("0"));
    assert_eq!(evidence.okx_message.as_deref(), Some(""));
    assert!(evidence.detail.contains("provider DTO"));
}

/// HTTP 200 不代表 provider 成功；非零 OKX code 必须保留为独立失败层。
#[tokio::test]
async fn provider_business_error_preserves_code_message_and_quota() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/api/v5/public/instruments?instType=SWAP")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_header("retry-after", "2")
        .with_header("ratelimit-remaining", "0")
        .with_body(r#"{"code":"50011","msg":"Rate limit reached","data":[]}"#)
        .create_async()
        .await;
    let client = OkxPublicInstruments::with_base_url(server.url()).expect("public client");

    let error = client
        .list_swap()
        .await
        .expect_err("provider rejection must fail");

    request.assert_async().await;
    let Error::PublicApiError(evidence) = error else {
        panic!("expected public response evidence");
    };
    assert_eq!(evidence.kind, OkxPublicFailureKind::ProviderRejected);
    assert_eq!(evidence.okx_code.as_deref(), Some("50011"));
    assert_eq!(evidence.okx_message.as_deref(), Some("Rate limit reached"));
    assert_eq!(evidence.retry_after.as_deref(), Some("2"));
    assert_eq!(
        evidence
            .rate_limit_headers
            .get("ratelimit-remaining")
            .map(String::as_str),
        Some("0")
    );
}

/// 429 必须保留 status、provider envelope 与安全限频头，但不得泄漏非白名单响应头。
#[tokio::test]
async fn rate_limited_response_preserves_only_safe_header_evidence() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/api/v5/public/instruments?instType=SWAP")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_header("retry-after", "3")
        .with_header("x-rate-limit-reset", "1700000000")
        .with_header("set-cookie", "internal-session=must-not-leak")
        .with_body(r#"{"code":"50011","msg":"Too many requests","data":[]}"#)
        .create_async()
        .await;
    let client = OkxPublicInstruments::with_base_url(server.url()).expect("public client");

    let error = client
        .list_swap()
        .await
        .expect_err("429 must fail without SDK retry");

    request.assert_async().await;
    let Error::PublicApiError(evidence) = error else {
        panic!("expected public response evidence");
    };
    assert_eq!(evidence.kind, OkxPublicFailureKind::HttpStatus);
    assert_eq!(evidence.http_status, 429);
    assert_eq!(evidence.okx_code.as_deref(), Some("50011"));
    assert_eq!(evidence.retry_after.as_deref(), Some("3"));
    assert_eq!(
        evidence
            .rate_limit_headers
            .get("x-rate-limit-reset")
            .map(String::as_str),
        Some("1700000000")
    );
    assert!(!evidence.rate_limit_headers.contains_key("set-cookie"));
}

/// 5xx 的非 JSON body 仍应保留 HTTP 与 quota 层，不能被压平成普通 JSON 错误。
#[tokio::test]
async fn server_error_with_non_json_body_preserves_http_evidence() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/api/v5/public/instruments?instType=SWAP")
        .with_status(503)
        .with_header("retry-after", "10")
        .with_header("x-ratelimit-remaining", "0")
        .with_body("upstream unavailable")
        .create_async()
        .await;
    let client = OkxPublicInstruments::with_base_url(server.url()).expect("public client");

    let error = client
        .list_swap()
        .await
        .expect_err("503 must fail without SDK retry");

    request.assert_async().await;
    let Error::PublicApiError(evidence) = error else {
        panic!("expected public response evidence");
    };
    assert_eq!(evidence.kind, OkxPublicFailureKind::HttpStatus);
    assert_eq!(evidence.http_status, 503);
    assert_eq!(evidence.retry_after.as_deref(), Some("10"));
    assert_eq!(evidence.okx_code, None);
}

/// HTTP 200 空 body 不含任何可验证 envelope，必须 fail closed 并保留状态。
#[tokio::test]
async fn empty_success_body_is_rejected_as_an_empty_envelope() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/api/v5/public/instruments?instType=SWAP")
        .with_status(200)
        .with_body("")
        .create_async()
        .await;
    let client = OkxPublicInstruments::with_base_url(server.url()).expect("public client");

    let error = client.list_swap().await.expect_err("empty body must fail");

    request.assert_async().await;
    let Error::PublicApiError(evidence) = error else {
        panic!("expected public response evidence");
    };
    assert_eq!(evidence.kind, OkxPublicFailureKind::EmptyBody);
    assert_eq!(evidence.http_status, 200);
    assert!(evidence.detail.contains("empty response body"));
}
