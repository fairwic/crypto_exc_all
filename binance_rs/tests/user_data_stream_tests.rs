use binance_rs::api::websocket::{BinanceUserDataStreamSession, BinanceWebsocket};
use binance_rs::client::BinanceClient;
use binance_rs::config::Credentials;
use futures_util::{SinkExt, StreamExt};
use mockito::Server;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

#[tokio::test]
async fn user_data_session_owns_listen_key_and_uses_official_private_path() {
    let mut rest = Server::new_async().await;
    let start = listen_key_mock(&mut rest, "POST", r#"{"listenKey":"listen-key"}"#).await;
    let keepalive = listen_key_mock(&mut rest, "PUT", r#"{"listenKey":"listen-key"}"#).await;
    let close = listen_key_mock(&mut rest, "DELETE", "").await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let stream_base_url = format!("ws://{}", listener.local_addr().unwrap());
    let (path_tx, path_rx) = oneshot::channel();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut path_tx = Some(path_tx);
        let mut socket = accept_hdr_async(stream, move |request: &Request, response: Response| {
            path_tx
                .take()
                .unwrap()
                .send(request.uri().path().to_string())
                .unwrap();
            Ok(response)
        })
        .await
        .unwrap();
        socket
            .send(Message::Text(
                json!({"e":"ACCOUNT_UPDATE","E":10,"T":9,"a":{"B":[],"P":[]}})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        assert!(matches!(socket.next().await, Some(Ok(Message::Close(_)))));
    });

    let mut client = BinanceClient::new(Credentials::new("test-key", "test-secret")).unwrap();
    client.set_base_url(rest.url());
    let websocket = BinanceWebsocket::with_stream_config(client, stream_base_url, None);
    let mut session = BinanceUserDataStreamSession::open(websocket).await.unwrap();

    assert_eq!(path_rx.await.unwrap(), "/private/ws/listen-key");
    assert_eq!(
        timeout(Duration::from_secs(1), session.recv_json())
            .await
            .unwrap()
            .unwrap(),
        Some(json!({"e":"ACCOUNT_UPDATE","E":10,"T":9,"a":{"B":[],"P":[]}}))
    );
    session.keepalive().await.unwrap();
    session.close().await.unwrap();

    start.assert_async().await;
    keepalive.assert_async().await;
    close.assert_async().await;
}

#[tokio::test]
async fn keepalive_rejects_a_different_active_listen_key() {
    let mut rest = Server::new_async().await;
    let start = listen_key_mock(&mut rest, "POST", r#"{"listenKey":"listen-key"}"#).await;
    let keepalive = listen_key_mock(&mut rest, "PUT", r#"{"listenKey":"different-key"}"#).await;
    let close = listen_key_mock(&mut rest, "DELETE", "").await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let stream_base_url = format!("ws://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        assert!(matches!(socket.next().await, Some(Ok(Message::Close(_)))));
    });

    let mut client = BinanceClient::new(Credentials::new("test-key", "test-secret")).unwrap();
    client.set_base_url(rest.url());
    let websocket = BinanceWebsocket::with_stream_config(client, stream_base_url, None);
    let session = BinanceUserDataStreamSession::open(websocket).await.unwrap();

    assert!(session.keepalive().await.is_err());
    session.close().await.unwrap();

    start.assert_async().await;
    keepalive.assert_async().await;
    close.assert_async().await;
}

#[tokio::test]
async fn malformed_start_response_does_not_close_an_unowned_user_stream() {
    let mut rest = Server::new_async().await;
    let start = listen_key_mock(&mut rest, "POST", r#"{}"#).await;
    let close = rest
        .mock("DELETE", "/fapi/v1/listenKey")
        .match_header("x-mbx-apikey", "test-key")
        .expect(0)
        .create_async()
        .await;
    let mut client = BinanceClient::new(Credentials::new("test-key", "test-secret")).unwrap();
    client.set_base_url(rest.url());
    let websocket = BinanceWebsocket::with_stream_config(client, "ws://127.0.0.1:1", None);

    assert!(BinanceUserDataStreamSession::open(websocket).await.is_err());
    start.assert_async().await;
    close.assert_async().await;
}

#[tokio::test]
async fn websocket_connect_failure_does_not_expose_the_listen_key() {
    let mut rest = Server::new_async().await;
    let start = listen_key_mock(&mut rest, "POST", r#"{"listenKey":"sensitive-listen-key"}"#).await;
    let close = listen_key_mock(&mut rest, "DELETE", "").await;
    let mut client = BinanceClient::new(Credentials::new("test-key", "test-secret")).unwrap();
    client.set_base_url(rest.url());
    let websocket = BinanceWebsocket::with_stream_config(client, "not-a-websocket-url", None);

    let error = match BinanceUserDataStreamSession::open(websocket).await {
        Ok(_) => panic!("invalid websocket URL must fail"),
        Err(error) => error,
    };
    assert_eq!(error.to_string(), "WebSocket错误: 连接 Binance 用户流失败");
    assert!(!error.to_string().contains("sensitive-listen-key"));

    start.assert_async().await;
    close.assert_async().await;
}

async fn listen_key_mock(server: &mut Server, method: &str, body: &str) -> mockito::Mock {
    server
        .mock(method, "/fapi/v1/listenKey")
        .match_header("x-mbx-apikey", "test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await
}

#[test]
fn private_url_rejects_unsafe_listen_key_path_segments() {
    let websocket = BinanceWebsocket::new_public_with_stream_base_url("wss://example.test");

    assert!(websocket.private_ws_url("../escape").is_err());
    assert!(websocket.private_ws_url("key?query").is_err());
}
