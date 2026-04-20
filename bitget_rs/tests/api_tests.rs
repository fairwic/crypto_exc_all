use bitget_rs::api::account::{AccountBillRequest, BitgetAccount};
use bitget_rs::api::announcements::{AnnouncementListRequest, BitgetAnnouncements};
use bitget_rs::api::asset::{
    BitgetAsset, DepositAddressRequest, TransferRequest, WalletHistoryRequest, WithdrawRequest,
};
use bitget_rs::api::market::{BitgetMarket, TickerRequest};
use bitget_rs::api::trade::{
    BitgetTrade, CancelAllOrdersRequest, CancelOrderRequest, ClosePositionsRequest,
    ModifyOrderRequest, NewOrderRequest, OrderQueryRequest,
};
use bitget_rs::client::BitgetClient;
use bitget_rs::config::{Config, Credentials};
use mockito::{Matcher, Server};

fn public_client(server_url: String) -> BitgetClient {
    BitgetClient::with_config(None, Config::default().with_api_url_for_test(server_url)).unwrap()
}

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
async fn market_get_ticker_maps_bitget_v2_endpoint() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock(
            "GET",
            "/api/v2/mix/market/ticker?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"00000",
                "msg":"success",
                "requestTime":1695794095685,
                "data":[{
                    "symbol":"BTCUSDT",
                    "lastPr":"70000.1",
                    "askPr":"70000.2",
                    "bidPr":"69999.9",
                    "baseVolume":"12.3",
                    "quoteVolume":"861000",
                    "ts":"1695794098184"
                }]
            }"#,
        )
        .create_async()
        .await;

    let market = BitgetMarket::new(public_client(server.url()));
    let tickers = market
        .get_ticker(TickerRequest::new("BTCUSDT", "USDT-FUTURES"))
        .await
        .unwrap();

    assert_eq!(tickers[0].symbol, "BTCUSDT");
    assert_eq!(tickers[0].last_price, "70000.1");
    assert_eq!(tickers[0].bid_price, "69999.9");
    mock.assert_async().await;
}

#[tokio::test]
async fn market_contracts_orderbook_and_candles_use_bitget_v2_paths() {
    let mut server = Server::new_async().await;
    let contracts = server
        .mock(
            "GET",
            "/api/v2/mix/market/contracts?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT"}]}"#)
        .create_async()
        .await;
    let orderbook = server
        .mock(
            "GET",
            "/api/v2/mix/market/orderbook?limit=5&productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"asks":[],"bids":[]}}"#)
        .create_async()
        .await;
    let candles = server
        .mock(
            "GET",
            "/api/v2/mix/market/candles?granularity=5m&limit=100&productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[]}"#)
        .create_async()
        .await;

    let market = BitgetMarket::new(public_client(server.url()));

    let contracts_value = market
        .get_contracts("USDT-FUTURES", Some("BTCUSDT"))
        .await
        .unwrap();
    let orderbook_value = market
        .get_orderbook("BTCUSDT", "USDT-FUTURES", Some("5"))
        .await
        .unwrap();
    let candles_value = market
        .get_candles("BTCUSDT", "USDT-FUTURES", "5m", Some(100))
        .await
        .unwrap();

    assert_eq!(contracts_value[0]["symbol"], "BTCUSDT");
    assert!(orderbook_value["asks"].is_array());
    assert!(candles_value.is_array());
    contracts.assert_async().await;
    orderbook.assert_async().await;
    candles.assert_async().await;
}

#[tokio::test]
async fn market_public_helpers_cover_time_ratios_and_oi_paths() {
    let mut server = Server::new_async().await;
    let time = server
        .mock("GET", "/api/v2/public/time")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"serverTime":"1688008631614"}}"#)
        .create_async()
        .await;
    let long_short = server
        .mock(
            "GET",
            "/api/v2/mix/market/long-short?period=5m&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"longShortRatio":"1.2"}]}"#)
        .create_async()
        .await;
    let account_long_short = server
        .mock(
            "GET",
            "/api/v2/mix/market/account-long-short?period=5m&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"longShortAccountRatio":"1.1"}]}"#)
        .create_async()
        .await;
    let oi_limit = server
        .mock(
            "GET",
            "/api/v2/mix/market/oi-limit?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT"}]}"#)
        .create_async()
        .await;
    let position_tier = server
        .mock(
            "GET",
            "/api/v2/mix/market/query-position-lever?productType=USDT-FUTURES&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"level":"1"}]}"#)
        .create_async()
        .await;
    let taker_volume = server
        .mock(
            "GET",
            "/api/v2/mix/market/taker-buy-sell?period=5m&symbol=BTCUSDT",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"buyVolume":"10"}]}"#)
        .create_async()
        .await;

    let market = BitgetMarket::new(public_client(server.url()));

    let time_value = market.get_server_time().await.unwrap();
    let long_short_value = market
        .get_long_short_ratio("BTCUSDT", Some("5m"))
        .await
        .unwrap();
    let account_ratio_value = market
        .get_account_long_short_ratio("BTCUSDT", Some("5m"))
        .await
        .unwrap();
    let oi_limit_value = market
        .get_open_interest_limit("USDT-FUTURES", Some("BTCUSDT"))
        .await
        .unwrap();
    let tier_value = market
        .get_position_tier("BTCUSDT", "USDT-FUTURES")
        .await
        .unwrap();
    let taker_volume_value = market
        .get_taker_buy_sell_volume("BTCUSDT", Some("5m"))
        .await
        .unwrap();

    assert_eq!(time_value["serverTime"], "1688008631614");
    assert_eq!(long_short_value[0]["longShortRatio"], "1.2");
    assert_eq!(account_ratio_value[0]["longShortAccountRatio"], "1.1");
    assert_eq!(oi_limit_value[0]["symbol"], "BTCUSDT");
    assert_eq!(tier_value[0]["level"], "1");
    assert_eq!(taker_volume_value[0]["buyVolume"], "10");
    time.assert_async().await;
    long_short.assert_async().await;
    account_long_short.assert_async().await;
    oi_limit.assert_async().await;
    position_tier.assert_async().await;
    taker_volume.assert_async().await;
}

#[tokio::test]
async fn account_get_accounts_sends_bitget_signature_headers() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock(
            "GET",
            "/api/v2/mix/account/accounts?productType=USDT-FUTURES",
        )
        .match_header("ACCESS-KEY", "test-key")
        .match_header("ACCESS-PASSPHRASE", "test-pass")
        .match_header("ACCESS-TIMESTAMP", "1684814440729")
        .match_header("ACCESS-SIGN", Matcher::Regex(".+".to_string()))
        .match_header("locale", "en-US")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "code":"00000",
                "msg":"success",
                "requestTime":1695794095685,
                "data":[{
                    "marginCoin":"USDT",
                    "locked":"1.2",
                    "available":"105.5",
                    "accountEquity":"106.7",
                    "usdtEquity":"106.7"
                }]
            }"#,
        )
        .create_async()
        .await;

    let account = BitgetAccount::new(signed_client(server.url()));
    let accounts = account.get_accounts("USDT-FUTURES").await.unwrap();

    assert_eq!(accounts[0].margin_coin, "USDT");
    assert_eq!(accounts[0].available, "105.5");
    assert_eq!(accounts[0].account_equity, "106.7");
    mock.assert_async().await;
}

#[tokio::test]
async fn account_positions_and_leverage_use_signed_v2_paths() {
    let mut server = Server::new_async().await;
    let positions = server
        .mock(
            "GET",
            "/api/v2/mix/position/all-position?marginCoin=USDT&productType=USDT-FUTURES",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":[{"symbol":"BTCUSDT"}]}"#)
        .create_async()
        .await;
    let leverage = server
        .mock("POST", "/api/v2/mix/account/set-leverage")
        .match_header("ACCESS-KEY", "test-key")
        .match_header("ACCESS-TIMESTAMP", "1684814440729")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","leverage":"20"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"leverage":"20"}}"#)
        .create_async()
        .await;

    let account = BitgetAccount::new(signed_client(server.url()));

    let position_value = account
        .get_all_positions("USDT-FUTURES", Some("USDT"))
        .await
        .unwrap();
    let leverage_value = account
        .set_leverage("BTCUSDT", "USDT-FUTURES", "USDT", "20")
        .await
        .unwrap();

    assert_eq!(position_value[0]["symbol"], "BTCUSDT");
    assert_eq!(leverage_value["leverage"], "20");
    positions.assert_async().await;
    leverage.assert_async().await;
}

#[tokio::test]
async fn account_bills_margin_and_asset_mode_use_signed_v2_paths() {
    let mut server = Server::new_async().await;
    let bills = server
        .mock(
            "GET",
            "/api/v2/mix/account/bill?businessType=contract_settle_fee&coin=USDT&endTime=200&limit=50&onlyFunding=yes&productType=USDT-FUTURES&startTime=100",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"bills":[],"endId":"2"}}"#)
        .create_async()
        .await;
    let margin = server
        .mock("POST", "/api/v2/mix/account/set-margin")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","amount":"20","holdSide":"long"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":"success"}"#)
        .create_async()
        .await;
    let asset_mode = server
        .mock("POST", "/api/v2/mix/account/set-asset-mode")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"productType":"USDT-FUTURES","assetMode":"union"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":"success"}"#)
        .create_async()
        .await;
    let trade_rate = server
        .mock(
            "GET",
            "/api/v2/common/trade-rate?businessType=mix&symbol=BTCUSDT",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"makerFeeRate":"0.0002"}}"#)
        .create_async()
        .await;

    let account = BitgetAccount::new(signed_client(server.url()));

    let bills_value = account
        .get_account_bills(
            AccountBillRequest::new("USDT-FUTURES")
                .with_coin("USDT")
                .with_business_type("contract_settle_fee")
                .with_only_funding("yes")
                .with_start_time(100)
                .with_end_time(200)
                .with_limit(50),
        )
        .await
        .unwrap();
    let margin_value = account
        .set_position_margin("BTCUSDT", "USDT-FUTURES", "USDT", "20", "long")
        .await
        .unwrap();
    let asset_mode_value = account
        .set_asset_mode("USDT-FUTURES", "union")
        .await
        .unwrap();
    let trade_rate_value = account.get_trade_rate("BTCUSDT", "mix").await.unwrap();

    assert_eq!(bills_value["endId"], "2");
    assert_eq!(margin_value, "success");
    assert_eq!(asset_mode_value, "success");
    assert_eq!(trade_rate_value["makerFeeRate"], "0.0002");
    bills.assert_async().await;
    margin.assert_async().await;
    asset_mode.assert_async().await;
    trade_rate.assert_async().await;
}

#[tokio::test]
async fn announcements_query_uses_bitget_public_notice_endpoint() {
    let mut server = Server::new_async().await;
    let notices = server
        .mock(
            "GET",
            "/api/v2/public/annoucements?annType=api_trading&language=en_US&limit=10",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"announcements":[]}}"#)
        .create_async()
        .await;

    let announcements = BitgetAnnouncements::new(public_client(server.url()));
    let value = announcements
        .get_announcements(
            AnnouncementListRequest::new("en_US")
                .with_ann_type("api_trading")
                .with_limit(10),
        )
        .await
        .unwrap();

    assert!(value["announcements"].is_array());
    notices.assert_async().await;
}

#[tokio::test]
async fn trade_place_and_cancel_order_sign_json_body() {
    let mut server = Server::new_async().await;
    let place = server
        .mock("POST", "/api/v2/mix/order/place-order")
        .match_header("ACCESS-KEY", "test-key")
        .match_header("ACCESS-SIGN", Matcher::Regex(".+".to_string()))
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginMode":"crossed","marginCoin":"USDT","size":"0.001","side":"buy","orderType":"limit","price":"10000","tradeSide":"open","force":"post_only"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"code":"00000","msg":"success","data":{"orderId":"1","clientOid":"c1"}}"#,
        )
        .create_async()
        .await;
    let cancel = server
        .mock("POST", "/api/v2/mix/order/cancel-order")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","orderId":"1"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"orderId":"1"}}"#)
        .create_async()
        .await;

    let trade = BitgetTrade::new(signed_client(server.url()));

    let placed = trade
        .place_order(
            NewOrderRequest::limit(
                "BTCUSDT",
                "USDT-FUTURES",
                "crossed",
                "USDT",
                "0.001",
                "buy",
                "10000",
            )
            .with_trade_side("open")
            .with_force("post_only"),
        )
        .await
        .unwrap();
    let canceled = trade
        .cancel_order(
            CancelOrderRequest::new("BTCUSDT", "USDT-FUTURES")
                .with_margin_coin("USDT")
                .with_order_id("1"),
        )
        .await
        .unwrap();

    assert_eq!(placed["orderId"], "1");
    assert_eq!(canceled["orderId"], "1");
    place.assert_async().await;
    cancel.assert_async().await;
}

#[tokio::test]
async fn trade_query_modify_cancel_all_and_close_use_v2_paths() {
    let mut server = Server::new_async().await;
    let pending = server
        .mock(
            "GET",
            "/api/v2/mix/order/orders-pending?endTime=200&idLessThan=9&limit=20&productType=USDT-FUTURES&startTime=100&status=live&symbol=BTCUSDT",
        )
        .match_header("ACCESS-KEY", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"entrustedList":[],"endId":"9"}}"#)
        .create_async()
        .await;
    let modify = server
        .mock("POST", "/api/v2/mix/order/modify-order")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","marginCoin":"USDT","orderId":"1","newClientOid":"new-1","newSize":"0.002","newPrice":"11000"}"#
                .to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"orderId":"2"}}"#)
        .create_async()
        .await;
    let cancel_all = server
        .mock("POST", "/api/v2/mix/order/cancel-all-orders")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"productType":"USDT-FUTURES","marginCoin":"USDT"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"successList":[]}}"#)
        .create_async()
        .await;
    let close = server
        .mock("POST", "/api/v2/mix/order/close-positions")
        .match_header("ACCESS-KEY", "test-key")
        .match_body(Matcher::JsonString(
            r#"{"symbol":"BTCUSDT","productType":"USDT-FUTURES","holdSide":"long"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"00000","msg":"success","data":{"orderIdList":["1"]}}"#)
        .create_async()
        .await;

    let trade = BitgetTrade::new(signed_client(server.url()));

    let pending_value = trade
        .get_pending_orders_with(
            OrderQueryRequest::new("USDT-FUTURES")
                .with_symbol("BTCUSDT")
                .with_status("live")
                .with_id_less_than("9")
                .with_start_time(100)
                .with_end_time(200)
                .with_limit(20),
        )
        .await
        .unwrap();
    let modify_value = trade
        .modify_order(
            ModifyOrderRequest::new("BTCUSDT", "USDT-FUTURES", "new-1")
                .with_margin_coin("USDT")
                .with_order_id("1")
                .with_new_size("0.002")
                .with_new_price("11000"),
        )
        .await
        .unwrap();
    let cancel_all_value = trade
        .cancel_all_orders(CancelAllOrdersRequest::new("USDT-FUTURES").with_margin_coin("USDT"))
        .await
        .unwrap();
    let close_value = trade
        .close_positions(
            ClosePositionsRequest::new("USDT-FUTURES")
                .with_symbol("BTCUSDT")
                .with_hold_side("long"),
        )
        .await
        .unwrap();

    assert_eq!(pending_value["endId"], "9");
    assert_eq!(modify_value["orderId"], "2");
    assert!(cancel_all_value["successList"].is_array());
    assert_eq!(close_value["orderIdList"][0], "1");
    pending.assert_async().await;
    modify.assert_async().await;
    cancel_all.assert_async().await;
    close.assert_async().await;
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
