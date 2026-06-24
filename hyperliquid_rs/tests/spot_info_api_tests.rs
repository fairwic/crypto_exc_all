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
async fn sends_spot_deploy_and_outcome_info_requests_to_info_endpoint_without_signing() {
    let mut server = Server::new_async().await;
    let spot_deploy_state = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"spotDeployState","user":"0xabc"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"states":[{"token":150,"fullName":"Hyperliquid"}]}"#)
        .create_async()
        .await;
    let spot_pair_auction = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"spotPairDeployAuctionStatus"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"currentGas":"500.0"}"#)
        .create_async()
        .await;
    let token_details = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"tokenDetails","tokenId":"0x00000000000000000000000000000000"}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"name":"TEST","markPx":"3.2025"}"#)
        .create_async()
        .await;
    let outcome_meta = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(r#"{"type":"outcomeMeta"}"#.to_string()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"outcomes":[{"outcome":123,"name":"Recurring"}]}"#)
        .create_async()
        .await;
    let settled_outcome = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(
            r#"{"type":"settledOutcome","outcome":95}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"settleFraction":"0.0","details":"price:76876.9"}"#)
        .create_async()
        .await;

    let client = client(server.url());

    assert_eq!(
        client.spot_deploy_state("0xabc").await.unwrap()["states"][0]["token"],
        150
    );
    assert_eq!(
        client.spot_pair_deploy_auction_status().await.unwrap()["currentGas"],
        "500.0"
    );
    assert_eq!(
        client
            .token_details("0x00000000000000000000000000000000")
            .await
            .unwrap()["name"],
        "TEST"
    );
    assert_eq!(
        client.outcome_meta().await.unwrap()["outcomes"][0]["outcome"],
        123
    );
    assert_eq!(
        client.settled_outcome(95).await.unwrap()["details"],
        "price:76876.9"
    );

    spot_deploy_state.assert_async().await;
    spot_pair_auction.assert_async().await;
    token_details.assert_async().await;
    outcome_meta.assert_async().await;
    settled_outcome.assert_async().await;
}
