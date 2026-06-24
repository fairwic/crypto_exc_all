use hyperliquid_rs::{Config, Error, HyperliquidClient};
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
async fn maps_plain_text_non_success_response_to_api_error_with_status() {
    let mut server = Server::new_async().await;
    let rate_limit = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(r#"{"type":"meta"}"#.to_string()))
        .with_status(429)
        .with_header("content-type", "text/plain")
        .with_body("rate limited")
        .create_async()
        .await;

    let error = client(server.url()).meta().await.unwrap_err();

    match error {
        Error::HyperliquidApiError {
            status,
            code,
            message,
        } => {
            assert_eq!(status, Some(429));
            assert_eq!(code, "429");
            assert_eq!(message, "rate limited");
        }
        other => panic!("expected HyperliquidApiError, got {other:?}"),
    }

    rate_limit.assert_async().await;
}

#[tokio::test]
async fn extracts_json_error_message_from_non_success_response() {
    let mut server = Server::new_async().await;
    let invalid_request = server
        .mock("POST", "/info")
        .match_body(Matcher::JsonString(r#"{"type":"meta"}"#.to_string()))
        .with_status(400)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error":"bad info request"}"#)
        .create_async()
        .await;

    let error = client(server.url()).meta().await.unwrap_err();

    match error {
        Error::HyperliquidApiError {
            status,
            code,
            message,
        } => {
            assert_eq!(status, Some(400));
            assert_eq!(code, "400");
            assert_eq!(message, "bad info request");
        }
        other => panic!("expected HyperliquidApiError, got {other:?}"),
    }

    invalid_request.assert_async().await;
}
