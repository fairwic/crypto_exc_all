#![cfg(feature = "public-market")]

use mockito::{Matcher, Server};
use okx::{OkxPublicLeadTraderPage, OkxPublicLeadTraders, OkxPublicResponse};

#[tokio::test]
async fn public_lead_trader_ranks_are_anonymous_and_lossless() {
    let mut server = Server::new_async().await;
    let request = server
        .mock("GET", "/api/v5/copytrading/public-lead-traders")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("instType".into(), "SWAP".into()),
            Matcher::UrlEncoded("sortType".into(), "overview".into()),
            Matcher::UrlEncoded("state".into(), "0".into()),
            Matcher::UrlEncoded("page".into(), "1".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .match_header("OK-ACCESS-KEY", Matcher::Missing)
        .match_header("OK-ACCESS-SIGN", Matcher::Missing)
        .match_header("OK-ACCESS-PASSPHRASE", Matcher::Missing)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_header("x-ratelimit-remaining", "4")
        .with_body(
            r#"{
              "code":"0",
              "msg":"",
              "data":[{
                "dataVer":"20260830140001",
                "totalPage":"88",
                "futureField":"preserved",
                "ranks":[{
                  "uniqueCode":"DA2B29551CBB2AE7",
                  "nickName":"Public trader",
                  "portLink":"https://static.example/avatar.png",
                  "copyState":"0",
                  "copyTraderNum":"12",
                  "maxCopyTraderNum":"100",
                  "accCopyTraderNum":"50",
                  "ccy":"USDT",
                  "pnl":"1234.56",
                  "pnlRatio":"0.125",
                  "aum":"9999.99",
                  "winRatio":"0.60",
                  "leadDays":"180",
                  "traderInsts":["BTC-USDT-SWAP"],
                  "pnlRatios":[{"beginTs":"1788000000000","pnlRatio":"0.1"}],
                  "futureRankField":{"value":"kept"}
                }]
              }]
            }"#,
        )
        .create_async()
        .await;
    let client = OkxPublicLeadTraders::with_base_url(server.url()).expect("public client");

    let response: OkxPublicResponse<Vec<OkxPublicLeadTraderPage>> =
        client.list_swap_overview(1, 2).await.expect("lead ranks");

    request.assert_async().await;
    assert_eq!(response.evidence.okx_code, "0");
    assert_eq!(
        response
            .evidence
            .rate_limit_headers
            .get("x-ratelimit-remaining")
            .map(String::as_str),
        Some("4")
    );
    let page = response.data.first().expect("one page");
    assert_eq!(page.data_version, "20260830140001");
    assert_eq!(page.extra["futureField"], "preserved");
    let rank = page.ranks.first().expect("one rank");
    assert_eq!(rank.unique_code, "DA2B29551CBB2AE7");
    assert_eq!(rank.pnl_ratio, "0.125");
    assert_eq!(rank.extra["futureRankField"]["value"], "kept");
}

#[tokio::test]
async fn invalid_page_size_fails_before_network_io() {
    let client = OkxPublicLeadTraders::new().expect("public client");
    let error = client
        .list_swap_overview(1, 21)
        .await
        .expect_err("limit above provider maximum must fail");
    assert!(error.to_string().contains("1..=20"));
}
