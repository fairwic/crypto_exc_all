use futures::{SinkExt, StreamExt};
use okx::config::Credentials;
use okx::websocket::OkxPrivateAccountStreamClient;
use serde_json::{json, Value};
use std::collections::HashSet;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn private_account_stream_waits_for_login_and_all_channel_acks() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (requests_tx, requests_rx) = oneshot::channel();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let login = next_json(&mut socket).await;
        socket
            .send(Message::Text(
                r#"{"event":"login","code":"0","msg":""}"#.into(),
            ))
            .await
            .unwrap();
        let subscribe = next_json(&mut socket).await;
        socket
            .send(Message::Text(
                r#"{"event":"subscribe","arg":{"channel":"account"},"code":"0"}"#.into(),
            ))
            .await
            .unwrap();
        socket
            .send(Message::Text(
                r#"{"arg":{"channel":"orders"},"data":[{"ordId":"1","uTime":"7"}]}"#.into(),
            ))
            .await
            .unwrap();
        for channel in ["positions", "orders"] {
            socket
                .send(Message::Text(
                    json!({"event":"subscribe","arg":{"channel":channel},"code":"0"})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
        }
        requests_tx.send((login, subscribe)).unwrap();
        assert!(matches!(
            socket.next().await,
            Some(Ok(Message::Text(text))) if text.as_str() == "ping"
        ));
        socket
            .send(Message::Text(
                r#"{"arg":{"channel":"positions"},"data":[{"posId":"2","uTime":"8"}]}"#.into(),
            ))
            .await
            .unwrap();
        socket.send(Message::Text("pong".into())).await.unwrap();
        assert!(matches!(socket.next().await, Some(Ok(Message::Close(_)))));
    });

    let credentials = Credentials::new("key", "secret", "pass", "1");
    let client = OkxPrivateAccountStreamClient::new(credentials).with_url(url);
    let mut session = client.connect().await.unwrap();
    let (login, subscribe) = requests_rx.await.unwrap();

    assert_eq!(login["op"], "login");
    assert_eq!(subscribe["op"], "subscribe");
    let channels = subscribe["args"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|arg| arg["channel"].as_str())
        .collect::<HashSet<_>>();
    assert_eq!(channels, HashSet::from(["account", "positions", "orders"]));
    assert_eq!(
        timeout(Duration::from_secs(1), session.recv_json())
            .await
            .unwrap()
            .unwrap(),
        Some(json!({"arg":{"channel":"orders"},"data":[{"ordId":"1","uTime":"7"}]}))
    );
    session.heartbeat().await.unwrap();
    assert_eq!(
        session.recv_json().await.unwrap(),
        Some(json!({"arg":{"channel":"positions"},"data":[{"posId":"2","uTime":"8"}]}))
    );
    session.close().await.unwrap();
}

#[tokio::test]
async fn private_account_stream_rejects_login_ack_without_success_code() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let _ = next_json(&mut socket).await;
        socket
            .send(Message::Text(r#"{"event":"login","msg":""}"#.into()))
            .await
            .unwrap();
    });

    let credentials = Credentials::new("key", "secret", "pass", "1");
    let client = OkxPrivateAccountStreamClient::new(credentials).with_url(url);

    assert!(client.connect().await.is_err());
}

async fn next_json<S>(socket: &mut tokio_tungstenite::WebSocketStream<S>) -> Value
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    match socket.next().await.unwrap().unwrap() {
        Message::Text(text) => serde_json::from_str(text.as_str()).unwrap(),
        other => panic!("expected text frame, got {other:?}"),
    }
}
