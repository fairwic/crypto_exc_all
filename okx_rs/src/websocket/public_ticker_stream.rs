//! 单一公开 ticker 连接；批量订阅、帧大小与心跳有界，重连由调用方拥有。
use super::socket_transport::{connect_socket, OkxSocket};
use crate::Error;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::time::{timeout, Duration, Instant};
use tokio_tungstenite::tungstenite::{protocol::WebSocketConfig, Message};

pub struct OkxPublicTickerStream {
    socket: OkxSocket,
    last_frame: Instant,
}
impl OkxPublicTickerStream {
    pub async fn connect(symbols: &[String]) -> Result<Self, Error> {
        Self::connect_url("wss://ws.okx.com:8443/ws/v5/public", symbols).await
    }
    pub async fn connect_url(url: &str, symbols: &[String]) -> Result<Self, Error> {
        let config = WebSocketConfig::default()
            .max_message_size(Some(1024 * 1024))
            .max_frame_size(Some(1024 * 1024));
        let (socket, _) = timeout(Duration::from_secs(10), connect_socket(url, config))
            .await
            .map_err(|_| Error::TimeoutError("OKX ticker handshake timeout".into()))??;
        let mut stream = Self {
            socket,
            last_frame: Instant::now(),
        };
        stream.subscribe("subscribe", symbols).await?;
        Ok(stream)
    }
    pub async fn subscribe(&mut self, operation: &str, symbols: &[String]) -> Result<(), Error> {
        if !matches!(operation, "subscribe" | "unsubscribe")
            || symbols.len() > 2000
            || symbols.iter().any(|s| {
                s.is_empty()
                    || s.len() > 64
                    || !s
                        .bytes()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'-')
            })
        {
            return Err(Error::ParameterError("invalid ticker subscription".into()));
        }
        for batch in symbols.chunks(100) {
            let args: Vec<_> = batch
                .iter()
                .map(|s| json!({"channel":"tickers","instId":s}))
                .collect();
            self.socket
                .send(Message::Text(
                    json!({"op":operation,"args":args}).to_string().into(),
                ))
                .await
                .map_err(|e| Error::WebSocketError(e.to_string()))?;
        }
        Ok(())
    }
    pub async fn recv_json(&mut self) -> Result<Option<Value>, Error> {
        loop {
            let Some(frame) = self.socket.next().await else {
                return Ok(None);
            };
            self.last_frame = Instant::now();
            match frame.map_err(|e| Error::WebSocketError(e.to_string()))? {
                Message::Text(text) if text.as_str() == "pong" => {}
                Message::Text(text) => {
                    return serde_json::from_str(&text).map(Some).map_err(Error::from)
                }
                Message::Ping(value) => self
                    .socket
                    .send(Message::Pong(value))
                    .await
                    .map_err(|e| Error::WebSocketError(e.to_string()))?,
                Message::Close(_) => return Ok(None),
                _ => {}
            }
        }
    }
    pub async fn heartbeat(&mut self) -> Result<(), Error> {
        if self.last_frame.elapsed() > Duration::from_secs(30) {
            return Err(Error::TimeoutError("OKX ticker heartbeat timeout".into()));
        }
        if self.last_frame.elapsed() > Duration::from_secs(10) {
            self.socket
                .send(Message::Text("ping".into()))
                .await
                .map_err(|e| Error::WebSocketError(e.to_string()))?;
        }
        Ok(())
    }
    pub async fn close(&mut self) {
        let _ = self.socket.close(None).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    #[tokio::test]
    async fn public_tickers_batch_subscribe_read_and_unsubscribe_without_login() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
            let request: Value =
                serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(request["op"], "subscribe");
            assert_eq!(request["args"].as_array().unwrap().len(), 2);
            ws.send(Message::Text(
                json!({"arg":{"channel":"tickers"},"data":[]})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            let request: Value =
                serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(request["op"], "unsubscribe");
        });
        let mut session = OkxPublicTickerStream::connect_url(
            &format!("ws://{addr}"),
            &["BTC-USDT-SWAP".into(), "ETH-USDT-SWAP".into()],
        )
        .await
        .unwrap();
        assert_eq!(
            session.recv_json().await.unwrap().unwrap()["arg"]["channel"],
            "tickers"
        );
        session
            .subscribe("unsubscribe", &["BTC-USDT-SWAP".into()])
            .await
            .unwrap();
        server.await.unwrap();
        session.close().await;
    }
}
