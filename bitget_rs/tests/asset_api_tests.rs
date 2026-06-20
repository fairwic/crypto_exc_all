use bitget_rs::api::asset::{
    BitgetAsset, DepositAddressRequest, TransferRequest, WalletHistoryRequest, WithdrawRequest,
};
use bitget_rs::client::BitgetClient;
use bitget_rs::config::{Config, Credentials};
use mockito::{Matcher, Server};

fn signed_client(server_url: String) -> BitgetClient {
    let mut client = BitgetClient::with_config(
        Some(Credentials::new("test-key", "test-secret", "test-pass")),
        Config::default().with_api_url_for_test(server_url),
    )
    .unwrap();
    client.set_timestamp_provider(|| 1_684_814_440_729);
    client
}

trait TestConfigExt {
    fn with_api_url_for_test(self, api_url: String) -> Self;
}

impl TestConfigExt for Config {
    fn with_api_url_for_test(mut self, api_url: String) -> Self {
        self.api_url = api_url;
        self
    }
}

#[tokio::test]
async fn asset_wallet_methods_use_bitget_v2_paths() {
    let mut server = Server::new_async().await;
    let coins = server
        .mock("GET", "/api/v2/spot/public/coins?coin=USDT")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"coin":"USDT"}]}"#)
        .create_async()
        .await;
    let deposit_address = server
        .mock(
            "GET",
            "/api/v2/spot/wallet/deposit-address?chain=trc20&coin=USDT",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"coin":"USDT","chain":"trc20"}}"#)
        .create_async()
        .await;
    let transfer = server
        .mock("POST", "/api/v2/spot/wallet/transfer")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"fromType":"spot","toType":"usdt_futures","amount":"10","coin":"USDT"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"transferId":"t1"}}"#)
        .create_async()
        .await;

    let asset = BitgetAsset::new(signed_client(server.url()));

    let coins_value = asset.get_coins(Some("USDT")).await.unwrap();
    let address_value = asset
        .get_deposit_address(DepositAddressRequest::new("USDT", "trc20"))
        .await
        .unwrap();
    let transfer_value = asset
        .transfer(TransferRequest::new("spot", "usdt_futures", "10", "USDT"))
        .await
        .unwrap();

    assert_eq!(coins_value[0]["coin"], "USDT");
    assert_eq!(address_value["chain"], "trc20");
    assert_eq!(transfer_value["transferId"], "t1");
    coins.assert_async().await;
    deposit_address.assert_async().await;
    transfer.assert_async().await;
}

#[tokio::test]
async fn asset_history_transferable_coins_and_withdraw_use_v2_paths() {
    let mut server = Server::new_async().await;
    let deposit_history = server
        .mock(
            "GET",
            "/api/v2/spot/wallet/deposit-records?clientOid=c1&coin=USDT&endTime=200&idLessThan=9&limit=20&startTime=100",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"orderId":"9"}]}"#)
        .create_async()
        .await;
    let transferable = server
        .mock(
            "GET",
            "/api/v2/spot/wallet/transfer-coin-info?fromType=spot&toType=usdt_futures",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":["USDT"]}"#)
        .create_async()
        .await;
    let withdrawal = server
        .mock("POST", "/api/v2/spot/wallet/withdrawal")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"coin":"USDT","transferType":"on_chain","address":"TXYZ","size":"10","chain":"trc20","clientOid":"w1"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"orderId":"w1"}}"#)
        .create_async()
        .await;

    let asset = BitgetAsset::new(signed_client(server.url()));

    let history_value = asset
        .get_deposit_records(
            WalletHistoryRequest::new(100, 200)
                .with_coin("USDT")
                .with_client_oid("c1")
                .with_id_less_than("9")
                .with_limit(20),
        )
        .await
        .unwrap();
    let transferable_value = asset
        .get_transferable_coins("spot", "usdt_futures")
        .await
        .unwrap();
    let withdrawal_value = asset
        .withdraw(
            WithdrawRequest::on_chain("USDT", "TXYZ", "10")
                .with_chain("trc20")
                .with_client_oid("w1"),
        )
        .await
        .unwrap();

    assert_eq!(history_value[0]["orderId"], "9");
    assert_eq!(transferable_value[0], "USDT");
    assert_eq!(withdrawal_value["orderId"], "w1");
    deposit_history.assert_async().await;
    transferable.assert_async().await;
    withdrawal.assert_async().await;
}
