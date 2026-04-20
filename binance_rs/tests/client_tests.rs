use binance_rs::Error;
use binance_rs::client::BinanceClient;
use binance_rs::config::Credentials;
use binance_rs::dto::account::AccountBalance;
use mockito::{Matcher, Server};
use reqwest::Method;

#[tokio::test]
async fn signed_get_sends_api_key_and_signature_query_params() {
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

    let balances: Vec<AccountBalance> = client
        .send_signed_request(Method::GET, "/fapi/v2/balance", &[])
        .await
        .unwrap();

    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].asset, "USDT");
    assert_eq!(balances[0].available_balance, "23.72469206");
    assert!(balances[0].margin_available);
    mock.assert_async().await;
}

#[tokio::test]
async fn signed_get_maps_binance_error_payload() {
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
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":-2015,"msg":"Invalid API-key, IP, or permissions for action."}"#)
        .create_async()
        .await;

    let mut client = BinanceClient::new(Credentials::new("test-key", "test-secret")).unwrap();
    client.set_base_url(server.url());
    client.set_timestamp_provider(|| 1_591_702_613_943);

    let err = client
        .send_signed_request::<serde_json::Value>(Method::GET, "/fapi/v2/balance", &[])
        .await
        .unwrap_err();

    match err {
        Error::BinanceApiError {
            status,
            code,
            message,
        } => {
            assert_eq!(status, Some(401));
            assert_eq!(code, -2015);
            assert_eq!(message, "Invalid API-key, IP, or permissions for action.");
        }
        other => panic!("unexpected error: {other:?}"),
    }
    mock.assert_async().await;
}
