use gate_rs::{Config, Credentials, GateAccountBookRequest, GateClient};

#[tokio::test]
async fn sends_public_market_overview_queries_to_futures_paths() {
    let mut server = mockito::Server::new_async().await;
    let tickers = server
        .mock("GET", "/futures/usdt/tickers")
        .with_status(200)
        .with_body(r#"[{"contract":"BTC_USDT","total_size":"123","funding_rate":"0.0001"}]"#)
        .create_async()
        .await;
    let funding = server
        .mock(
            "GET",
            "/futures/usdt/funding_rate?contract=BTC_USDT&limit=20",
        )
        .with_status(200)
        .with_body(r#"[{"t":1543968000,"r":"0.000157"}]"#)
        .create_async()
        .await;
    let insurance = server
        .mock("GET", "/futures/usdt/insurance?limit=10")
        .with_status(200)
        .with_body(r#"[{"t":1543968000,"b":"83.0031"}]"#)
        .create_async()
        .await;

    let client = public_test_client(server.url());

    let tickers_result = client.tickers("usdt", None).await.unwrap();
    let funding_result = client
        .funding_rate_history("usdt", "BTC_USDT", Some(20))
        .await
        .unwrap();
    let insurance_result = client.insurance_history("usdt", Some(10)).await.unwrap();

    tickers.assert_async().await;
    funding.assert_async().await;
    insurance.assert_async().await;
    assert_eq!(tickers_result[0]["total_size"], "123");
    assert_eq!(funding_result[0]["r"], "0.000157");
    assert_eq!(insurance_result[0]["b"], "83.0031");
}

#[tokio::test]
async fn signed_account_book_query_sends_gate_auth_headers() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock(
            "GET",
            "/futures/usdt/account_book?from=1700000000&limit=50&to=1700007200&type=dnw",
        )
        .match_header("KEY", "api-key")
        .match_header("Timestamp", "1700000000")
        .with_status(200)
        .with_body(r#"[{"time":1547633726,"change":"0.1","balance":"4.5","type":"dnw"}]"#)
        .create_async()
        .await;

    let mut client = signed_test_client(server.url());
    client.set_timestamp_provider(|| 1_700_000_000);

    let result = client
        .account_book(
            "usdt",
            GateAccountBookRequest::new()
                .with_from(1_700_000_000)
                .with_to(1_700_007_200)
                .with_limit(50)
                .with_type("dnw"),
        )
        .await
        .unwrap();

    mock.assert_async().await;
    assert_eq!(result[0]["type"], "dnw");
}

fn public_test_client(api_url: String) -> GateClient {
    GateClient::with_config(
        None,
        Config {
            api_url,
            api_timeout_ms: 1_000,
            proxy_url: None,
        },
    )
    .unwrap()
}

fn signed_test_client(api_url: String) -> GateClient {
    GateClient::with_config(
        Some(Credentials::new("api-key", "secret")),
        Config {
            api_url,
            api_timeout_ms: 1_000,
            proxy_url: None,
        },
    )
    .unwrap()
}
