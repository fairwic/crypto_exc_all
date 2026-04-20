use binance_rs::api::asset::{
    BinanceAsset, DepositAddressRequest, DepositHistoryRequest, FundingWalletRequest,
    UniversalTransferHistoryRequest, UniversalTransferRequest, UserAssetRequest,
    WithdrawHistoryRequest, WithdrawRequest,
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

#[tokio::test]
async fn asset_wrappers_map_wallet_and_capital_endpoints() {
    let mut server = Server::new_async().await;
    let coins = server
        .mock("GET", "/sapi/v1/capital/config/getall")
        .match_header("x-mbx-apikey", "test-key")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"coin":"USDT","free":"10"}]"#)
        .create_async()
        .await;
    let wallet_balance = server
        .mock("GET", "/sapi/v1/asset/wallet/balance")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("quoteAsset".into(), "USDT".into()),
            Matcher::UrlEncoded("timestamp".into(), "1591702613943".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"walletName":"USDⓈ-M Futures","balance":"1"}]"#)
        .create_async()
        .await;
    let user_assets = server
        .mock("POST", "/sapi/v3/asset/getUserAsset")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("asset".into(), "USDT".into()),
            Matcher::UrlEncoded("needBtcValuation".into(), "true".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"asset":"USDT","free":"1"}]"#)
        .create_async()
        .await;
    let funding_wallet = server
        .mock("POST", "/sapi/v1/asset/get-funding-asset")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("asset".into(), "USDT".into()),
            Matcher::UrlEncoded("needBtcValuation".into(), "false".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"asset":"USDT","free":"1"}]"#)
        .create_async()
        .await;
    let transfer = server
        .mock("POST", "/sapi/v1/asset/transfer")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("type".into(), "MAIN_UMFUTURE".into()),
            Matcher::UrlEncoded("asset".into(), "USDT".into()),
            Matcher::UrlEncoded("amount".into(), "10".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"tranId":123}"#)
        .create_async()
        .await;
    let transfer_history = server
        .mock("GET", "/sapi/v1/asset/transfer")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("type".into(), "MAIN_UMFUTURE".into()),
            Matcher::UrlEncoded("current".into(), "1".into()),
            Matcher::UrlEncoded("size".into(), "10".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"total":1,"rows":[{"asset":"USDT","status":"CONFIRMED"}]}"#)
        .create_async()
        .await;
    let deposit_address = server
        .mock("GET", "/sapi/v1/capital/deposit/address")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("coin".into(), "USDT".into()),
            Matcher::UrlEncoded("network".into(), "TRX".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"coin":"USDT","address":"Txxx"}"#)
        .create_async()
        .await;
    let deposit_history = server
        .mock("GET", "/sapi/v1/capital/deposit/hisrec")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("coin".into(), "USDT".into()),
            Matcher::UrlEncoded("limit".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"coin":"USDT","status":1}]"#)
        .create_async()
        .await;
    let withdraw_history = server
        .mock("GET", "/sapi/v1/capital/withdraw/history")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("coin".into(), "USDT".into()),
            Matcher::UrlEncoded("status".into(), "6".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"coin":"USDT","status":6}]"#)
        .create_async()
        .await;
    let withdraw = server
        .mock("POST", "/sapi/v1/capital/withdraw/apply")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("coin".into(), "USDT".into()),
            Matcher::UrlEncoded("network".into(), "TRX".into()),
            Matcher::UrlEncoded("address".into(), "Txxx".into()),
            Matcher::UrlEncoded("amount".into(), "10".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"withdraw-id"}"#)
        .create_async()
        .await;

    let asset = BinanceAsset::new(signed_client(server.url()));

    assert_eq!(asset.get_all_coins().await.unwrap()[0]["coin"], "USDT");
    assert_eq!(
        asset.get_wallet_balance(Some("USDT")).await.unwrap()[0]["walletName"],
        "USDⓈ-M Futures"
    );
    assert_eq!(
        asset
            .get_user_assets(
                UserAssetRequest::new()
                    .with_asset("USDT")
                    .with_btc_valuation(true)
            )
            .await
            .unwrap()[0]["asset"],
        "USDT"
    );
    assert_eq!(
        asset
            .get_funding_wallet(
                FundingWalletRequest::new()
                    .with_asset("USDT")
                    .with_btc_valuation(false)
            )
            .await
            .unwrap()[0]["asset"],
        "USDT"
    );
    assert_eq!(
        asset
            .transfer(UniversalTransferRequest::new("MAIN_UMFUTURE", "USDT", "10"))
            .await
            .unwrap()["tranId"],
        123
    );
    assert_eq!(
        asset
            .get_transfer_history(
                UniversalTransferHistoryRequest::new("MAIN_UMFUTURE")
                    .with_current(1)
                    .with_size(10)
            )
            .await
            .unwrap()["rows"][0]["status"],
        "CONFIRMED"
    );
    assert_eq!(
        asset
            .get_deposit_address(DepositAddressRequest::new("USDT").with_network("TRX"))
            .await
            .unwrap()["address"],
        "Txxx"
    );
    assert_eq!(
        asset
            .get_deposit_history(
                DepositHistoryRequest::new()
                    .with_coin("USDT")
                    .with_limit(100)
            )
            .await
            .unwrap()[0]["status"],
        1
    );
    assert_eq!(
        asset
            .get_withdraw_history(
                WithdrawHistoryRequest::new()
                    .with_coin("USDT")
                    .with_status(6)
            )
            .await
            .unwrap()[0]["status"],
        6
    );
    assert_eq!(
        asset
            .withdraw(WithdrawRequest::new("USDT", "Txxx", "10").with_network("TRX"))
            .await
            .unwrap()["id"],
        "withdraw-id"
    );

    coins.assert_async().await;
    wallet_balance.assert_async().await;
    user_assets.assert_async().await;
    funding_wallet.assert_async().await;
    transfer.assert_async().await;
    transfer_history.assert_async().await;
    deposit_address.assert_async().await;
    deposit_history.assert_async().await;
    withdraw_history.assert_async().await;
    withdraw.assert_async().await;
}
