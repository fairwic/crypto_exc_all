use bybit_rs::{
    BybitClient, BybitDepositRecordRequest, BybitTransferRecordRequest,
    BybitWithdrawalRecordRequest, Config, Credentials,
};

#[tokio::test]
async fn sends_public_market_analytics_to_v5_market_paths() {
    let mut server = mockito::Server::new_async().await;
    let funding = server
        .mock(
            "GET",
            "/v5/market/funding/history?category=linear&endTime=1700007200000&limit=100&startTime=1700000000000&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"list":[{"fundingRate":"0.0001"}]}}"#)
        .create_async()
        .await;
    let open_interest = server
        .mock(
            "GET",
            "/v5/market/open-interest?category=linear&cursor=next&endTime=1700007200000&intervalTime=1h&limit=50&startTime=1700000000000&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"list":[{"openInterest":"123"}]}}"#)
        .create_async()
        .await;
    let long_short = server
        .mock(
            "GET",
            "/v5/market/account-ratio?category=linear&cursor=next&endTime=1700007200000&limit=50&period=1h&startTime=1700000000000&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"list":[{"buyRatio":"0.49"}]}}"#)
        .create_async()
        .await;

    let client = public_test_client(server.url());

    let funding_result = client
        .funding_rate_history(
            "linear",
            "BTCUSDT",
            Some(1_700_000_000_000),
            Some(1_700_007_200_000),
            Some(100),
        )
        .await
        .unwrap();
    let open_interest_result = client
        .open_interest(
            "linear",
            "BTCUSDT",
            "1h",
            Some(1_700_000_000_000),
            Some(1_700_007_200_000),
            Some(50),
            Some("next"),
        )
        .await
        .unwrap();
    let long_short_result = client
        .long_short_ratio(
            "linear",
            "BTCUSDT",
            "1h",
            Some(1_700_000_000_000),
            Some(1_700_007_200_000),
            Some(50),
            Some("next"),
        )
        .await
        .unwrap();

    funding.assert_async().await;
    open_interest.assert_async().await;
    long_short.assert_async().await;
    assert_eq!(funding_result["list"][0]["fundingRate"], "0.0001");
    assert_eq!(open_interest_result["list"][0]["openInterest"], "123");
    assert_eq!(long_short_result["list"][0]["buyRatio"], "0.49");
}

#[tokio::test]
async fn sends_public_platform_status_and_announcements_to_v5_paths() {
    let mut server = mockito::Server::new_async().await;
    let status = server
        .mock("GET", "/v5/system/status?id=maint-1&state=ongoing")
        .with_status(200)
        .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"list":[{"id":"maint-1"}]}}"#)
        .create_async()
        .await;
    let announcements = server
        .mock(
            "GET",
            "/v5/announcements/index?limit=1&locale=en-US&page=2&tag=Spot&type=new_crypto",
        )
        .with_status(200)
        .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"list":[{"title":"Listing"}]}}"#)
        .create_async()
        .await;

    let client = public_test_client(server.url());

    let status_result = client
        .system_status(Some("maint-1"), Some("ongoing"))
        .await
        .unwrap();
    let announcement_result = client
        .announcements("en-US", Some("new_crypto"), Some("Spot"), Some(2), Some(1))
        .await
        .unwrap();

    status.assert_async().await;
    announcements.assert_async().await;
    assert_eq!(status_result["list"][0]["id"], "maint-1");
    assert_eq!(announcement_result["list"][0]["title"], "Listing");
}

#[tokio::test]
async fn signed_asset_history_queries_send_bybit_auth_headers() {
    let mut server = mockito::Server::new_async().await;
    let transfers = server
        .mock(
            "GET",
            "/v5/asset/transfer/query-inter-transfer-list?coin=USDT&cursor=abc&endTime=1700007200000&limit=50&startTime=1700000000000&status=SUCCESS&transferId=tx-1",
        )
        .match_header("X-BAPI-API-KEY", "api-key")
        .match_header("X-BAPI-TIMESTAMP", "1700000000000")
        .match_header("X-BAPI-RECV-WINDOW", "5000")
        .with_status(200)
        .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"list":[{"transferId":"tx-1"}]}}"#)
        .create_async()
        .await;
    let deposits = server
        .mock(
            "GET",
            "/v5/asset/deposit/query-record?coin=USDT&cursor=abc&endTime=1700007200000&id=dep-1&limit=50&startTime=1700000000000&txID=hash-1",
        )
        .match_header("X-BAPI-API-KEY", "api-key")
        .match_header("X-BAPI-TIMESTAMP", "1700000000000")
        .match_header("X-BAPI-RECV-WINDOW", "5000")
        .with_status(200)
        .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"rows":[{"id":"dep-1"}]}}"#)
        .create_async()
        .await;
    let withdrawals = server
        .mock(
            "GET",
            "/v5/asset/withdraw/query-record?coin=USDT&cursor=abc&endTime=1700007200000&limit=50&startTime=1700000000000&txID=hash-1&withdrawID=wd-1&withdrawType=2",
        )
        .match_header("X-BAPI-API-KEY", "api-key")
        .match_header("X-BAPI-TIMESTAMP", "1700000000000")
        .match_header("X-BAPI-RECV-WINDOW", "5000")
        .with_status(200)
        .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"rows":[{"withdrawId":"wd-1"}]}}"#)
        .create_async()
        .await;

    let mut client = signed_test_client(server.url());
    client.set_timestamp_provider(|| 1_700_000_000_000);

    let transfer_result = client
        .internal_transfer_records(
            BybitTransferRecordRequest::new()
                .with_transfer_id("tx-1")
                .with_coin("USDT")
                .with_status("SUCCESS")
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_007_200_000)
                .with_limit(50)
                .with_cursor("abc"),
        )
        .await
        .unwrap();
    let deposit_result = client
        .deposit_records(
            BybitDepositRecordRequest::new()
                .with_id("dep-1")
                .with_tx_id("hash-1")
                .with_coin("USDT")
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_007_200_000)
                .with_limit(50)
                .with_cursor("abc"),
        )
        .await
        .unwrap();
    let withdrawal_result = client
        .withdrawal_records(
            BybitWithdrawalRecordRequest::new()
                .with_withdraw_id("wd-1")
                .with_tx_id("hash-1")
                .with_coin("USDT")
                .with_withdraw_type(2)
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_007_200_000)
                .with_limit(50)
                .with_cursor("abc"),
        )
        .await
        .unwrap();

    transfers.assert_async().await;
    deposits.assert_async().await;
    withdrawals.assert_async().await;
    assert_eq!(transfer_result["list"][0]["transferId"], "tx-1");
    assert_eq!(deposit_result["rows"][0]["id"], "dep-1");
    assert_eq!(withdrawal_result["rows"][0]["withdrawId"], "wd-1");
}

fn public_test_client(api_url: String) -> BybitClient {
    BybitClient::with_config(
        None,
        Config {
            api_url,
            api_timeout_ms: 1_000,
            recv_window_ms: 5_000,
            proxy_url: None,
        },
    )
    .unwrap()
}

fn signed_test_client(api_url: String) -> BybitClient {
    BybitClient::with_config(
        Some(Credentials::new("api-key", "secret")),
        Config {
            api_url,
            api_timeout_ms: 1_000,
            recv_window_ms: 5_000,
            proxy_url: None,
        },
    )
    .unwrap()
}
