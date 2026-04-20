use binance_rs::api::account::BinanceAccount;
use binance_rs::api::market::BinanceMarket;
use binance_rs::client::BinanceClient;
use binance_rs::config::Credentials;
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
