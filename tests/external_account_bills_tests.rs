use crypto_exc_all::{
    AccountBillQuery, BinanceExchangeConfig, BitgetExchangeConfig, BybitExchangeConfig, CryptoSdk,
    ExchangeId, GateExchangeConfig, SdkConfig,
};
use mockito::{Matcher, Server};

#[tokio::test]
async fn external_consumer_uses_root_crate_for_binance_account_bills() {
    let mut binance_server = Server::new_async().await;
    let deposit_history = binance_server
        .mock("GET", "/sapi/v1/capital/deposit/hisrec")
        .match_header("X-MBX-APIKEY", "binance-key")
        .match_query(signed_query(vec![
            Matcher::UrlEncoded("coin".into(), "USDT".into()),
            Matcher::UrlEncoded("startTime".into(), "1700000000000".into()),
            Matcher::UrlEncoded("endTime".into(), "1700007200000".into()),
            Matcher::UrlEncoded("limit".into(), "50".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "id":"deposit-binance-1",
                "coin":"USDT",
                "amount":"7.5",
                "status":1,
                "insertTime":1700000100000
            }]"#,
        )
        .create_async()
        .await;
    let withdraw_history = binance_server
        .mock("GET", "/sapi/v1/capital/withdraw/history")
        .match_header("X-MBX-APIKEY", "binance-key")
        .match_query(signed_query(vec![
            Matcher::UrlEncoded("coin".into(), "USDT".into()),
            Matcher::UrlEncoded("startTime".into(), "1700000000000".into()),
            Matcher::UrlEncoded("endTime".into(), "1700007200000".into()),
            Matcher::UrlEncoded("limit".into(), "50".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "id":"withdraw-binance-1",
                "coin":"USDT",
                "amount":"2.1",
                "transactionFee":"0.1",
                "status":6,
                "applyTime":1700000200000
            }]"#,
        )
        .create_async()
        .await;
    let transfer_history = binance_server
        .mock("GET", "/sapi/v1/asset/transfer")
        .match_header("X-MBX-APIKEY", "binance-key")
        .match_query(signed_query(vec![
            Matcher::UrlEncoded("type".into(), "MAIN_UMFUTURE".into()),
            Matcher::UrlEncoded("startTime".into(), "1700000000000".into()),
            Matcher::UrlEncoded("endTime".into(), "1700007200000".into()),
            Matcher::UrlEncoded("size".into(), "20".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "total":1,
                "rows":[{
                    "tranId":123456,
                    "asset":"USDT",
                    "amount":"5.4",
                    "type":"MAIN_UMFUTURE",
                    "status":"CONFIRMED",
                    "timestamp":1700000300000
                }]
            }"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
        binance: Some(BinanceExchangeConfig {
            api_key: "binance-key".to_string(),
            api_secret: "binance-secret".to_string(),
            api_url: Some("http://127.0.0.1:1".to_string()),
            sapi_api_url: Some(binance_server.url()),
            web_api_url: None,
            ws_stream_url: None,
            api_timeout_ms: Some(1_000),
            recv_window_ms: Some(5_000),
            proxy_url: None,
        }),
        ..SdkConfig::default()
    })
    .unwrap();

    let dnw_query = AccountBillQuery::new()
        .with_bill_type("dnw")
        .with_asset("USDT")
        .with_limit(50)
        .with_start_time(1_700_000_000_000)
        .with_end_time(1_700_007_200_000);
    let transfer_query = AccountBillQuery::new()
        .with_bill_type("MAIN_UMFUTURE")
        .with_limit(20)
        .with_start_time(1_700_000_000_000)
        .with_end_time(1_700_007_200_000);

    let dnw_bills = sdk
        .account(ExchangeId::Binance)
        .unwrap()
        .bills(dnw_query)
        .await
        .unwrap();
    let transfer_bills = sdk
        .account(ExchangeId::Binance)
        .unwrap()
        .bills(transfer_query)
        .await
        .unwrap();

    assert_eq!(dnw_bills.len(), 2);
    assert_eq!(dnw_bills[0].bill_id.as_deref(), Some("deposit-binance-1"));
    assert_eq!(dnw_bills[0].bill_type.as_deref(), Some("deposit"));
    assert_eq!(dnw_bills[0].asset.as_deref(), Some("USDT"));
    assert_eq!(dnw_bills[0].balance_change.as_deref(), Some("7.5"));
    assert_eq!(dnw_bills[0].timestamp, Some(1_700_000_100_000));
    assert_eq!(dnw_bills[1].bill_id.as_deref(), Some("withdraw-binance-1"));
    assert_eq!(dnw_bills[1].bill_type.as_deref(), Some("withdrawal"));
    assert_eq!(dnw_bills[1].fee.as_deref(), Some("0.1"));
    assert_eq!(dnw_bills[1].timestamp, Some(1_700_000_200_000));

    assert_eq!(transfer_bills.len(), 1);
    assert_eq!(transfer_bills[0].bill_id.as_deref(), Some("123456"));
    assert_eq!(
        transfer_bills[0].bill_type.as_deref(),
        Some("MAIN_UMFUTURE")
    );
    assert_eq!(transfer_bills[0].balance_change.as_deref(), Some("5.4"));
    assert_eq!(transfer_bills[0].timestamp, Some(1_700_000_300_000));

    deposit_history.assert_async().await;
    withdraw_history.assert_async().await;
    transfer_history.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_bitget_account_bills() {
    let mut bitget_server = Server::new_async().await;
    let account_bills = bitget_server
        .mock(
            "GET",
            "/api/v2/mix/account/bill?businessType=contract_settle_fee&coin=USDT&endTime=1700007200000&limit=50&productType=USDT-FUTURES&startTime=1700000000000",
        )
        .match_header("ACCESS-KEY", "bitget-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"00000",
                "msg":"success",
                "data":{"bills":[{
                    "id":"bitget-bill-1",
                    "coin":"USDT",
                    "amount":"-0.25",
                    "fee":"0.01",
                    "businessType":"contract_settle_fee",
                    "symbol":"BTCUSDT",
                    "uTime":"1700000400000"
                }],"endId":"2"}
            }"#,
        )
        .create_async()
        .await;

    let sdk = CryptoSdk::from_config(SdkConfig {
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

    let bills = sdk
        .account(ExchangeId::Bitget)
        .unwrap()
        .bills(
            AccountBillQuery::new()
                .with_bill_type("contract_settle_fee")
                .with_asset("USDT")
                .with_limit(50)
                .with_start_time(1_700_000_000_000)
                .with_end_time(1_700_007_200_000),
        )
        .await
        .unwrap();

    assert_eq!(bills.len(), 1);
    assert_eq!(bills[0].bill_id.as_deref(), Some("bitget-bill-1"));
    assert_eq!(bills[0].exchange_symbol.as_deref(), Some("BTCUSDT"));
    assert_eq!(bills[0].asset.as_deref(), Some("USDT"));
    assert_eq!(bills[0].balance_change.as_deref(), Some("-0.25"));
    assert_eq!(bills[0].fee.as_deref(), Some("0.01"));
    assert_eq!(bills[0].bill_type.as_deref(), Some("contract_settle_fee"));
    assert_eq!(bills[0].timestamp, Some(1_700_000_400_000));

    account_bills.assert_async().await;
}

#[tokio::test]
async fn external_consumer_uses_root_crate_for_bybit_and_gate_account_bills() {
    let mut bybit_server = Server::new_async().await;
    let bybit_transfers = bybit_server
        .mock(
            "GET",
            "/v5/asset/transfer/query-inter-transfer-list?endTime=1700007200000&limit=50&startTime=1700000000000",
        )
        .match_header("X-BAPI-API-KEY", "bybit-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{"list":[{
                    "transferId":"transfer-1",
                    "coin":"USDT",
                    "amount":"12.5",
                    "status":"SUCCESS",
                    "timestamp":"1700000100000"
                }]}
            }"#,
        )
        .create_async()
        .await;
    let bybit_deposits = bybit_server
        .mock(
            "GET",
            "/v5/asset/deposit/query-record?endTime=1700007200000&limit=50&startTime=1700000000000",
        )
        .match_header("X-BAPI-API-KEY", "bybit-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{"rows":[{
                    "id":"deposit-1",
                    "coin":"USDT",
                    "amount":"3.1",
                    "status":3,
                    "successAt":"1700000200000"
                }]}
            }"#,
        )
        .create_async()
        .await;
    let bybit_withdrawals = bybit_server
        .mock(
            "GET",
            "/v5/asset/withdraw/query-record?endTime=1700007200000&limit=50&startTime=1700000000000",
        )
        .match_header("X-BAPI-API-KEY", "bybit-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "retCode":0,
                "retMsg":"OK",
                "result":{"rows":[{
                    "withdrawID":"withdraw-1",
                    "coin":"USDT",
                    "amount":"1.2",
                    "status":"success",
                    "updatedTime":"1700000300000"
                }]}
            }"#,
        )
        .create_async()
        .await;

    let mut gate_server = Server::new_async().await;
    let gate_account_book = gate_server
        .mock(
            "GET",
            "/futures/usdt/account_book?from=1700000000&limit=50&to=1700007200&type=dnw",
        )
        .match_header("KEY", "gate-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{
                "time":1700000400,
                "change":"4.2",
                "balance":"104.2",
                "type":"dnw",
                "text":"transfer-gate-1"
            }]"#,
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
        gate: Some(GateExchangeConfig {
            api_key: "gate-key".to_string(),
            api_secret: "gate-secret".to_string(),
            api_url: Some(gate_server.url()),
            api_timeout_ms: Some(1_000),
            proxy_url: None,
            settle: Some("usdt".to_string()),
        }),
        ..SdkConfig::default()
    })
    .unwrap();

    let query = AccountBillQuery::new()
        .with_bill_type("dnw")
        .with_limit(50)
        .with_start_time(1_700_000_000_000)
        .with_end_time(1_700_007_200_000);

    let bybit_bills = sdk
        .account(ExchangeId::Bybit)
        .unwrap()
        .bills(query.clone())
        .await
        .unwrap();
    let gate_bills = sdk
        .account(ExchangeId::Gate)
        .unwrap()
        .bills(query)
        .await
        .unwrap();

    assert_eq!(bybit_bills.len(), 3);
    assert_eq!(bybit_bills[0].bill_id.as_deref(), Some("transfer-1"));
    assert_eq!(bybit_bills[0].asset.as_deref(), Some("USDT"));
    assert_eq!(bybit_bills[0].balance_change.as_deref(), Some("12.5"));
    assert_eq!(bybit_bills[0].bill_type.as_deref(), Some("transfer"));
    assert_eq!(bybit_bills[1].bill_type.as_deref(), Some("deposit"));
    assert_eq!(bybit_bills[2].bill_type.as_deref(), Some("withdrawal"));

    assert_eq!(gate_bills.len(), 1);
    assert_eq!(gate_bills[0].bill_id.as_deref(), Some("transfer-gate-1"));
    assert_eq!(gate_bills[0].balance_change.as_deref(), Some("4.2"));
    assert_eq!(gate_bills[0].balance_after.as_deref(), Some("104.2"));
    assert_eq!(gate_bills[0].bill_type.as_deref(), Some("dnw"));
    assert_eq!(gate_bills[0].timestamp, Some(1_700_000_400_000));

    bybit_transfers.assert_async().await;
    bybit_deposits.assert_async().await;
    bybit_withdrawals.assert_async().await;
    gate_account_book.assert_async().await;
}

fn signed_query(mut matchers: Vec<Matcher>) -> Matcher {
    matchers.push(Matcher::UrlEncoded("recvWindow".into(), "5000".into()));
    matchers.push(Matcher::Regex("(^|&)timestamp=".into()));
    matchers.push(Matcher::Regex("(^|&)signature=".into()));
    Matcher::AllOf(matchers)
}
