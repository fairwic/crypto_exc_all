use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, client_async_tls_with_config, connect_async,
};

use crate::error::Error;

use super::{
    BitgetWebsocket, BitgetWebsocketChannel, BitgetWebsocketEvent, WEBSOCKET_CHANNEL_SIZE,
};

pub struct BitgetWebsocketSession {
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<BitgetWebsocketEvent>,
}

impl BitgetWebsocketSession {
    pub async fn connect(url: &str) -> Result<Self, Error> {
        Self::connect_with_proxy(url, None).await
    }

    pub async fn connect_with_proxy(url: &str, proxy_url: Option<&str>) -> Result<Self, Error> {
        let (stream, _) = connect_websocket(url, proxy_url).await?;
        let (mut write, mut read) = stream.split();
        let (tx_in, mut rx_in) = mpsc::channel::<Message>(WEBSOCKET_CHANNEL_SIZE);
        let (tx_out, rx_out) = mpsc::channel::<BitgetWebsocketEvent>(WEBSOCKET_CHANNEL_SIZE);
        let ping_tx = tx_in.clone();

        tokio::spawn(async move {
            while let Some(message) = rx_in.recv().await {
                if write.send(message).await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        let forward_failed = forward_event(text.as_str(), &tx_out).await.is_err();
                        if forward_failed {
                            break;
                        }
                    }
                    Ok(Message::Binary(bytes)) => {
                        let Ok(text) = std::str::from_utf8(&bytes) else {
                            continue;
                        };
                        if forward_event(text, &tx_out).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Ping(payload)) => {
                        let send_failed = ping_tx.send(Message::Pong(payload)).await.is_err();
                        if send_failed {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
        });

        Ok(Self {
            tx: tx_in,
            rx: rx_out,
        })
    }

    pub async fn recv_event(&mut self) -> Option<BitgetWebsocketEvent> {
        self.rx.recv().await
    }

    pub async fn ping(&self) -> Result<(), Error> {
        self.tx
            .send(Message::Text("ping".into()))
            .await
            .map_err(|err| Error::WebSocketError(format!("发送 ping 失败: {err}")))
    }

    pub async fn login(&self, request: Value) -> Result<(), Error> {
        self.send_json(request).await
    }

    pub async fn subscribe(&self, channels: &[BitgetWebsocketChannel]) -> Result<(), Error> {
        self.send_text(BitgetWebsocket::subscribe_request(channels))
            .await
    }

    pub async fn unsubscribe(&self, channels: &[BitgetWebsocketChannel]) -> Result<(), Error> {
        self.send_text(BitgetWebsocket::unsubscribe_request(channels))
            .await
    }

    pub async fn close(&self) -> Result<(), Error> {
        self.tx
            .send(Message::Close(None))
            .await
            .map_err(|err| Error::WebSocketError(format!("发送关闭消息失败: {err}")))
    }

    async fn send_json(&self, value: Value) -> Result<(), Error> {
        self.send_text(value.to_string()).await
    }

    async fn send_text(&self, text: String) -> Result<(), Error> {
        self.tx
            .send(Message::Text(text.into()))
            .await
            .map_err(|err| Error::WebSocketError(format!("发送 WebSocket 消息失败: {err}")))
    }
}

pub(super) async fn connect_websocket(
    url: &str,
    proxy_url: Option<&str>,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), Error> {
    if let Some(proxy_addr) = proxy_url.and_then(socks5_proxy_addr) {
        return connect_websocket_via_socks5(url, &proxy_addr).await;
    }

    connect_async(url)
        .await
        .map_err(|err| Error::WebSocketError(format!("连接失败: {err}")))
}

async fn connect_websocket_via_socks5(
    url: &str,
    proxy_addr: &str,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), Error> {
    let parsed = url::Url::parse(url)
        .map_err(|err| Error::WebSocketError(format!("WebSocket URL 无效: {err}")))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| Error::WebSocketError("WebSocket URL 缺少 host".to_string()))?;
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| Error::WebSocketError("WebSocket URL 缺少 port".to_string()))?;

    let mut stream = TcpStream::connect(proxy_addr)
        .await
        .map_err(|err| Error::WebSocketError(format!("连接 SOCKS5 代理失败: {err}")))?;
    socks5_connect(&mut stream, host, port).await?;

    client_async_tls_with_config(url, stream, None, None)
        .await
        .map_err(|err| Error::WebSocketError(format!("代理 WebSocket 握手失败: {err}")))
}

async fn socks5_connect(stream: &mut TcpStream, host: &str, port: u16) -> Result<(), Error> {
    let host_bytes = host.as_bytes();
    if host_bytes.len() > u8::MAX as usize {
        return Err(Error::WebSocketError(
            "SOCKS5 目标 host 长度超过 255 字节".to_string(),
        ));
    }

    stream
        .write_all(&[0x05, 0x01, 0x00])
        .await
        .map_err(|err| Error::WebSocketError(format!("发送 SOCKS5 greeting 失败: {err}")))?;
    let mut greeting = [0_u8; 2];
    stream
        .read_exact(&mut greeting)
        .await
        .map_err(|err| Error::WebSocketError(format!("读取 SOCKS5 greeting 失败: {err}")))?;
    if greeting != [0x05, 0x00] {
        return Err(Error::WebSocketError(format!(
            "SOCKS5 代理不支持 no-auth: {greeting:?}"
        )));
    }

    let mut request = Vec::with_capacity(7 + host_bytes.len());
    request.extend_from_slice(&[0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8]);
    request.extend_from_slice(host_bytes);
    request.extend_from_slice(&port.to_be_bytes());
    stream
        .write_all(&request)
        .await
        .map_err(|err| Error::WebSocketError(format!("发送 SOCKS5 connect 请求失败: {err}")))?;

    let mut response = [0_u8; 4];
    stream
        .read_exact(&mut response)
        .await
        .map_err(|err| Error::WebSocketError(format!("读取 SOCKS5 响应失败: {err}")))?;
    if response[0] != 0x05 || response[1] != 0x00 {
        return Err(Error::WebSocketError(format!(
            "SOCKS5 connect 失败: {response:?}"
        )));
    }

    match response[3] {
        0x01 => read_exact_discard(stream, 4).await?,
        0x03 => {
            let mut len = [0_u8; 1];
            stream
                .read_exact(&mut len)
                .await
                .map_err(|err| Error::WebSocketError(format!("读取 SOCKS5 地址长度失败: {err}")))?;
            read_exact_discard(stream, usize::from(len[0])).await?;
        }
        0x04 => read_exact_discard(stream, 16).await?,
        other => {
            return Err(Error::WebSocketError(format!(
                "SOCKS5 响应地址类型不支持: {other}"
            )));
        }
    }
    read_exact_discard(stream, 2).await?;

    Ok(())
}

async fn read_exact_discard(stream: &mut TcpStream, len: usize) -> Result<(), Error> {
    let mut buffer = vec![0_u8; len];
    stream
        .read_exact(&mut buffer)
        .await
        .map_err(|err| Error::WebSocketError(format!("读取 SOCKS5 响应字段失败: {err}")))?;
    Ok(())
}

fn socks5_proxy_addr(proxy_url: &str) -> Option<String> {
    let trimmed = proxy_url.trim();
    let rest = trimmed
        .strip_prefix("socks5h://")
        .or_else(|| trimmed.strip_prefix("socks5://"))?;
    let authority = rest.split('/').next().unwrap_or(rest);
    if authority.is_empty() {
        None
    } else {
        Some(authority.to_string())
    }
}

pub(super) async fn forward_event(
    text: &str,
    tx: &mpsc::Sender<BitgetWebsocketEvent>,
) -> Result<(), ()> {
    match BitgetWebsocketEvent::parse(text) {
        Ok(event) => tx.send(event).await.map_err(|_| ()),
        Err(_) => Ok(()),
    }
}
