use hyperliquid_rs::{Config, HyperliquidClient};
use mockito::{Matcher, Server};

fn client(url: String) -> HyperliquidClient {
    HyperliquidClient::with_config(Config {
        api_url: url,
        api_timeout_ms: 1_000,
        proxy_url: None,
    })
    .unwrap()
}

#[tokio::test]
async fn sends_public_market_info_requests_to_info_endpoint() {
    let mut server = Server::new_async().await;
    let meta = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(r#"{"type":"meta"}"#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"universe":[{"name":"BTC"}]}"#)
        .create_async()
        .await;
    let book = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"l2Book","coin":"BTC"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"coin":"BTC","levels":[[],[]]}"#)
        .create_async()
        .await;
    let aggregated_book = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"l2Book","coin":"BTC","nSigFigs":5,"mantissa":2}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"coin":"BTC","levels":[[{"px":"65000","sz":"1"}],[]]}"#)
        .create_async()
        .await;
    let all_mids_for_dex = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"allMids","dex":"testdex"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"testdex:BTC":"65000"}"#)
        .create_async()
        .await;
    let candles = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"candleSnapshot","req":{"coin":"BTC","interval":"1m","startTime":1700000000000,"endTime":1700000060000}}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create_async()
        .await;

    let client = client(server.url());

    assert_eq!(client.meta().await.unwrap()["universe"][0]["name"], "BTC");
    assert_eq!(client.l2_book("BTC").await.unwrap()["coin"], "BTC");
    assert_eq!(
        client
            .l2_book_with_precision("BTC", 5, Some(2))
            .await
            .unwrap()["levels"][0][0]["px"],
        "65000"
    );
    assert_eq!(
        client.all_mids_for_dex("testdex").await.unwrap()["testdex:BTC"],
        "65000"
    );
    assert!(
        client
            .candle_snapshot("BTC", "1m", 1_700_000_000_000, 1_700_000_060_000)
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );

    meta.assert_async().await;
    book.assert_async().await;
    aggregated_book.assert_async().await;
    all_mids_for_dex.assert_async().await;
    candles.assert_async().await;
}

#[tokio::test]
async fn sends_perpetuals_info_requests_to_info_endpoint_without_signing() {
    let mut server = Server::new_async().await;
    let perp_dexs = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(r#"{"type":"perpDexs"}"#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[null,{"name":"test","fullName":"test dex"}]"#)
        .create_async()
        .await;
    let meta_for_dex = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"meta","dex":"testdex"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"universe":[{"name":"testdex:BTC"}],"marginTables":[]}"#)
        .create_async()
        .await;
    let meta_and_asset_ctxs_for_dex = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"metaAndAssetCtxs","dex":"testdex"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"universe":[{"name":"testdex:BTC"}],"marginTables":[]},[{"markPx":"65000"}]]"#,
        )
        .create_async()
        .await;
    let clearinghouse_for_dex = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"clearinghouseState","user":"0xabc","dex":"testdex"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"assetPositions":[],"marginSummary":{"accountValue":"10"}}"#)
        .create_async()
        .await;
    let predicted_fundings = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"predictedFundings"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[["BTC",[["HlPerp",{"fundingRate":"0.0000125"}]]]]"#)
        .create_async()
        .await;
    let oi_caps = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"perpsAtOpenInterestCap","dex":"testdex"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"["testdex:BTC"]"#)
        .create_async()
        .await;
    let auction = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"perpDeployAuctionStatus"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"currentGas":"500.0"}"#)
        .create_async()
        .await;
    let active_asset_data = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"activeAssetData","user":"0xabc","coin":"testdex:BTC"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"coin":"testdex:BTC","markPx":"65000"}"#)
        .create_async()
        .await;
    let dex_limits = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"perpDexLimits","dex":"testdex"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"totalOiCap":"10000000.0"}"#)
        .create_async()
        .await;
    let dex_status = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"perpDexStatus","dex":"testdex"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"totalNetDeposit":"4103492112.4478230476"}"#)
        .create_async()
        .await;
    let all_perp_metas = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"allPerpMetas"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[[{"universe":[{"name":"BTC"}]},[]]]"#)
        .create_async()
        .await;
    let annotation = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"perpAnnotation","coin":"BTC"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"category":"other","description":"other perps"}"#)
        .create_async()
        .await;
    let categories = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"perpCategories"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[["BTC","majors"]]"#)
        .create_async()
        .await;
    let concise_annotations = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"perpConciseAnnotations"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[["BTC",{"category":"majors"}]]"#)
        .create_async()
        .await;

    let client = client(server.url());

    assert_eq!(client.perp_dexs().await.unwrap()[1]["name"], "test");
    assert_eq!(
        client.meta_for_dex("testdex").await.unwrap()["universe"][0]["name"],
        "testdex:BTC"
    );
    assert_eq!(
        client.meta_and_asset_ctxs_for_dex("testdex").await.unwrap()[1][0]["markPx"],
        "65000"
    );
    assert_eq!(
        client
            .clearinghouse_state_for_dex("0xabc", "testdex")
            .await
            .unwrap()["marginSummary"]["accountValue"],
        "10"
    );
    assert_eq!(client.predicted_fundings().await.unwrap()[0][0], "BTC");
    assert_eq!(
        client
            .perps_at_open_interest_cap(Some("testdex"))
            .await
            .unwrap()[0],
        "testdex:BTC"
    );
    assert_eq!(
        client.perp_deploy_auction_status().await.unwrap()["currentGas"],
        "500.0"
    );
    assert_eq!(
        client
            .active_asset_data("0xabc", "testdex:BTC")
            .await
            .unwrap()["markPx"],
        "65000"
    );
    assert_eq!(
        client.perp_dex_limits("testdex").await.unwrap()["totalOiCap"],
        "10000000.0"
    );
    assert_eq!(
        client.perp_dex_status("testdex").await.unwrap()["totalNetDeposit"],
        "4103492112.4478230476"
    );
    assert_eq!(
        client.all_perp_metas().await.unwrap()[0][0]["universe"][0]["name"],
        "BTC"
    );
    assert_eq!(
        client.perp_annotation("BTC").await.unwrap()["category"],
        "other"
    );
    assert_eq!(client.perp_categories().await.unwrap()[0][1], "majors");
    assert_eq!(
        client.perp_concise_annotations().await.unwrap()[0][1]["category"],
        "majors"
    );

    perp_dexs.assert_async().await;
    meta_for_dex.assert_async().await;
    meta_and_asset_ctxs_for_dex.assert_async().await;
    clearinghouse_for_dex.assert_async().await;
    predicted_fundings.assert_async().await;
    oi_caps.assert_async().await;
    auction.assert_async().await;
    active_asset_data.assert_async().await;
    dex_limits.assert_async().await;
    dex_status.assert_async().await;
    all_perp_metas.assert_async().await;
    annotation.assert_async().await;
    categories.assert_async().await;
    concise_annotations.assert_async().await;
}

#[tokio::test]
async fn sends_user_info_requests_to_info_endpoint_without_signing() {
    let mut server = Server::new_async().await;
    let clearinghouse = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"clearinghouseState","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"marginSummary":{"accountValue":"10"}}"#)
        .create_async()
        .await;
    let ledger = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userNonFundingLedgerUpdates","user":"0xabc","startTime":1700000000000,"endTime":1700007200000}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create_async()
        .await;
    let user_funding = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userFunding","user":"0xabc","startTime":1700000000000,"endTime":1700007200000}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"time":1700000000000,"hash":"0x2","delta":{"type":"funding","coin":"BTC","usdc":"-0.12"}}]"#,
        )
        .create_async()
        .await;

    let client = client(server.url());

    assert_eq!(
        client.clearinghouse_state("0xabc").await.unwrap()["marginSummary"]["accountValue"],
        "10"
    );
    assert!(
        client
            .user_non_funding_ledger_updates("0xabc", 1_700_000_000_000, Some(1_700_007_200_000))
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        client
            .user_funding("0xabc", 1_700_000_000_000, Some(1_700_007_200_000))
            .await
            .unwrap()[0]["delta"]["type"],
        "funding"
    );

    clearinghouse.assert_async().await;
    ledger.assert_async().await;
    user_funding.assert_async().await;
}

#[tokio::test]
async fn sends_user_readiness_info_requests_to_info_endpoint_without_signing() {
    let mut server = Server::new_async().await;
    let rate_limit = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userRateLimit","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"cumVlm":"100","nRequestsUsed":10,"nRequestsCap":1000}"#)
        .create_async()
        .await;
    let role = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userRole","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"role":"agent","data":{"user":"0xmaster"}}"#)
        .create_async()
        .await;
    let fees = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userFees","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"userCrossRate":"0.000315","userAddRate":"0.000105"}"#)
        .create_async()
        .await;
    let sub_accounts = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"subAccounts","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"name":"Trading","subAccountUser":"0xsub","master":"0xabc"}]"#)
        .create_async()
        .await;
    let portfolio = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"portfolio","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[["day",{"accountValueHistory":[[1700000000000,"1000"]]}]]"#)
        .create_async()
        .await;
    let vault_details = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"vaultDetails","vaultAddress":"0xvault","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name":"Test","vaultAddress":"0xvault","leader":"0xleader","followers":[]}"#)
        .create_async()
        .await;
    let vault_equities = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userVaultEquities","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"vaultAddress":"0xvault","equity":"742500.082809"}]"#)
        .create_async()
        .await;
    let account_state = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userAbstraction","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#""unifiedAccount""#)
        .create_async()
        .await;
    let dex_abstraction_state = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userDexAbstraction","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"true"#)
        .create_async()
        .await;
    let approved_builders = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"approvedBuilders","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"["0xbuilder"]"#)
        .create_async()
        .await;
    let max_builder_fee = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"maxBuilderFee","user":"0xabc","builder":"0xbuilder"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"1"#)
        .create_async()
        .await;

    let client = client(server.url());

    assert_eq!(
        client.user_rate_limit("0xabc").await.unwrap()["nRequestsCap"],
        1000
    );
    assert_eq!(client.user_role("0xabc").await.unwrap()["role"], "agent");
    assert_eq!(
        client.user_fees("0xabc").await.unwrap()["userCrossRate"],
        "0.000315"
    );
    assert_eq!(
        client.sub_accounts("0xabc").await.unwrap()[0]["subAccountUser"],
        "0xsub"
    );
    assert_eq!(client.portfolio("0xabc").await.unwrap()[0][0], "day");
    assert_eq!(
        client
            .vault_details("0xvault", Some("0xabc"))
            .await
            .unwrap()["name"],
        "Test"
    );
    assert_eq!(
        client.user_vault_equities("0xabc").await.unwrap()[0]["equity"],
        "742500.082809"
    );
    assert_eq!(
        client.user_abstraction("0xabc").await.unwrap(),
        "unifiedAccount"
    );
    assert_eq!(client.user_dex_abstraction("0xabc").await.unwrap(), true);
    assert_eq!(
        client.approved_builders("0xabc").await.unwrap()[0],
        "0xbuilder"
    );
    assert_eq!(
        client.max_builder_fee("0xabc", "0xbuilder").await.unwrap(),
        1
    );

    rate_limit.assert_async().await;
    role.assert_async().await;
    fees.assert_async().await;
    sub_accounts.assert_async().await;
    portfolio.assert_async().await;
    vault_details.assert_async().await;
    vault_equities.assert_async().await;
    account_state.assert_async().await;
    dex_abstraction_state.assert_async().await;
    approved_builders.assert_async().await;
    max_builder_fee.assert_async().await;
}

#[tokio::test]
async fn sends_spot_info_requests_to_info_endpoint_without_signing() {
    let mut server = Server::new_async().await;
    let spot_meta = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(r#"{"type":"spotMeta"}"#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"tokens":[],"universe":[]}"#)
        .create_async()
        .await;
    let spot_meta_and_asset_ctxs = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"spotMetaAndAssetCtxs"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"tokens":[],"universe":[]},[]]"#)
        .create_async()
        .await;
    let spot_clearinghouse = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"spotClearinghouseState","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"balances":[]}"#)
        .create_async()
        .await;

    let client = client(server.url());

    assert!(
        client.spot_meta().await.unwrap()["tokens"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        client.spot_meta_and_asset_ctxs().await.unwrap()[1]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        client.spot_clearinghouse_state("0xabc").await.unwrap()["balances"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    spot_meta.assert_async().await;
    spot_meta_and_asset_ctxs.assert_async().await;
    spot_clearinghouse.assert_async().await;
}

#[tokio::test]
async fn sends_borrow_lend_info_requests_to_info_endpoint_without_signing() {
    let mut server = Server::new_async().await;
    let user_state = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"borrowLendUserState","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"tokenToState":[],"health":"healthy","healthFactor":null}"#)
        .create_async()
        .await;
    let reserve_state = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"borrowLendReserveState","token":150}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"borrowYearlyRate":"0.05","totalBorrowed":"0.0"}"#)
        .create_async()
        .await;
    let all_reserve_states = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"allBorrowLendReserveStates"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[[150,{"borrowYearlyRate":"0.05","totalBorrowed":"0.0"}]]"#)
        .create_async()
        .await;

    let client = client(server.url());

    assert_eq!(
        client.borrow_lend_user_state("0xabc").await.unwrap()["health"],
        "healthy"
    );
    assert_eq!(
        client.borrow_lend_reserve_state(150).await.unwrap()["borrowYearlyRate"],
        "0.05"
    );
    assert_eq!(
        client.all_borrow_lend_reserve_states().await.unwrap()[0][0],
        150
    );

    user_state.assert_async().await;
    reserve_state.assert_async().await;
    all_reserve_states.assert_async().await;
}

#[tokio::test]
async fn sends_referral_and_staking_info_requests_to_info_endpoint_without_signing() {
    let mut server = Server::new_async().await;
    let referral = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"referral","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"referredBy":{"code":"TESTNET"},"unclaimedRewards":"11.047361"}"#)
        .create_async()
        .await;
    let delegations = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"delegations","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"validator":"0xvalidator","amount":"12060.16529862"}]"#)
        .create_async()
        .await;
    let delegator_summary = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"delegatorSummary","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"delegated":"12060.16529862","nPendingWithdrawals":0}"#)
        .create_async()
        .await;
    let delegator_history = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"delegatorHistory","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"time":1735380381353,"hash":"0xhash"}]"#)
        .create_async()
        .await;
    let delegator_rewards = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"delegatorRewards","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"source":"delegation","totalAmount":"0.73117184"}]"#)
        .create_async()
        .await;

    let client = client(server.url());

    assert_eq!(
        client.referral("0xabc").await.unwrap()["referredBy"]["code"],
        "TESTNET"
    );
    assert_eq!(
        client.delegations("0xabc").await.unwrap()[0]["validator"],
        "0xvalidator"
    );
    assert_eq!(
        client.delegator_summary("0xabc").await.unwrap()["delegated"],
        "12060.16529862"
    );
    assert_eq!(
        client.delegator_history("0xabc").await.unwrap()[0]["hash"],
        "0xhash"
    );
    assert_eq!(
        client.delegator_rewards("0xabc").await.unwrap()[0]["source"],
        "delegation"
    );

    referral.assert_async().await;
    delegations.assert_async().await;
    delegator_summary.assert_async().await;
    delegator_history.assert_async().await;
    delegator_rewards.assert_async().await;
}

#[tokio::test]
async fn sends_order_and_fill_read_requests_to_info_endpoint_without_signing() {
    let mut server = Server::new_async().await;
    let open_orders = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"openOrders","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create_async()
        .await;
    let open_orders_for_dex = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"openOrders","user":"0xabc","dex":"testdex"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"coin":"testdex:BTC","oid":12345}]"#)
        .create_async()
        .await;
    let frontend_open_orders = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"frontendOpenOrders","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create_async()
        .await;
    let frontend_open_orders_for_dex = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"frontendOpenOrders","user":"0xabc","dex":"testdex"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create_async()
        .await;
    let order_status = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"orderStatus","user":"0xabc","oid":12345}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status":"unknownOid"}"#)
        .create_async()
        .await;
    let historical_orders = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"historicalOrders","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create_async()
        .await;
    let twap_slice_fills = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userTwapSliceFills","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"twapId":3156,"fill":{"coin":"AVAX","px":"18.435","sz":"93.53","side":"B","time":1681222254710,"fee":"0.01","feeToken":"USDC","tid":118906512037719}}]"#,
        )
        .create_async()
        .await;
    let fills = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userFillsByTime","user":"0xabc","startTime":1700000000000,"endTime":1700007200000}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create_async()
        .await;
    let aggregated_fills = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userFills","user":"0xabc","aggregateByTime":true}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create_async()
        .await;
    let aggregated_fills_by_time = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"userFillsByTime","user":"0xabc","startTime":1700000000000,"aggregateByTime":true}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[]"#)
        .create_async()
        .await;

    let client = client(server.url());

    assert!(
        client
            .open_orders("0xabc")
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        client
            .open_orders_for_dex("0xabc", "testdex")
            .await
            .unwrap()[0]["coin"],
        "testdex:BTC"
    );
    assert!(
        client
            .frontend_open_orders("0xabc", None)
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        client
            .frontend_open_orders("0xabc", Some("testdex"))
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        client.order_status("0xabc", "12345").await.unwrap()["status"],
        "unknownOid"
    );
    assert!(
        client
            .historical_orders("0xabc")
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        client.user_twap_slice_fills("0xabc").await.unwrap()[0]["twapId"],
        3156
    );
    assert!(
        client
            .user_fills_by_time("0xabc", 1_700_000_000_000, Some(1_700_007_200_000))
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        client
            .user_fills_aggregated("0xabc", true)
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        client
            .user_fills_by_time_aggregated("0xabc", 1_700_000_000_000, None, true)
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );

    open_orders.assert_async().await;
    open_orders_for_dex.assert_async().await;
    frontend_open_orders.assert_async().await;
    frontend_open_orders_for_dex.assert_async().await;
    order_status.assert_async().await;
    historical_orders.assert_async().await;
    twap_slice_fills.assert_async().await;
    fills.assert_async().await;
    aggregated_fills.assert_async().await;
    aggregated_fills_by_time.assert_async().await;
}
