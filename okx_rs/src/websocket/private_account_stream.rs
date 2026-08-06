use crate::config::{Credentials, DEFAULT_PRIVATE_WEBSOCKET_URL};
use crate::error::Error;
use crate::utils::{generate_signature, generate_timestamp_websocket};
use futures::{SinkExt, StreamExt};
use reqwest::Method;
use serde_json::{json, Value};
use std::collections::{HashSet, VecDeque};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async_with_config, MaybeTlsStream, WebSocketStream};

const MAX_PRIVATE_MESSAGE_BYTES: usize = 1024 * 1024;
const MAX_HANDSHAKE_BUFFERED_FRAMES: usize = 64;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

type OkxPrivateSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// OKX 账户私有流连接器，只负责 provider 登录与固定 P0 频道订阅。
#[derive(Clone)]
pub struct OkxPrivateAccountStreamClient {
    credentials: Credentials,
    url: String,
}

impl OkxPrivateAccountStreamClient {
    /// 使用官方私有 WebSocket 地址创建连接器。
    pub fn new(credentials: Credentials) -> Self {
        Self {
            credentials,
            url: DEFAULT_PRIVATE_WEBSOCKET_URL.to_string(),
        }
    }

    /// 覆盖私有流地址，供显式环境配置与本地协议测试使用。
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// 建立单连接并等待 login、account、positions、orders 全部确认。
    ///
    /// ACK 期间到达的业务 frame 会先缓存，避免“订阅成功”确认与首批账户事实
    /// 交错时丢数据；重连与恢复由 Account owner 决定。
    pub async fn connect(&self) -> Result<OkxPrivateAccountStreamSession, Error> {
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_PRIVATE_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_PRIVATE_MESSAGE_BYTES));
        let (mut socket, _) = timeout(
            HANDSHAKE_TIMEOUT,
            connect_async_with_config(&self.url, Some(config), false),
        )
        .await
        .map_err(|_| Error::TimeoutError("连接 OKX 私有流超时".to_string()))?
        .map_err(|error| Error::WebSocketError(format!("连接 OKX 私有流失败: {error}")))?;

        send_login(&mut socket, &self.credentials).await?;
        let mut pending = VecDeque::new();
        wait_for_login(&mut socket, &mut pending).await?;
        send_subscriptions(&mut socket).await?;
        wait_for_subscriptions(&mut socket, &mut pending).await?;

        Ok(OkxPrivateAccountStreamSession { socket, pending })
    }
}

/// 已完成认证与 P0 频道确认的 OKX 私有账户流会话。
pub struct OkxPrivateAccountStreamSession {
    socket: OkxPrivateSocket,
    pending: VecDeque<Value>,
}

impl OkxPrivateAccountStreamSession {
    /// 按 provider 顺序读取下一条 JSON，优先返回握手期间缓存的业务 frame。
    pub async fn recv_json(&mut self) -> Result<Option<Value>, Error> {
        if let Some(value) = self.pending.pop_front() {
            return Ok(Some(value));
        }
        recv_json(&mut self.socket).await
    }

    /// 发送 OKX 应用层 `ping` 并等待 `pong`；等待期间的业务 frame 按原顺序缓存。
    pub async fn heartbeat(&mut self) -> Result<(), Error> {
        self.socket
            .send(Message::Text("ping".into()))
            .await
            .map_err(|error| Error::WebSocketError(format!("发送 OKX heartbeat 失败: {error}")))?;
        timeout(HANDSHAKE_TIMEOUT, self.wait_for_pong())
            .await
            .map_err(|_| Error::TimeoutError("等待 OKX heartbeat pong 超时".to_string()))?
    }

    /// 主动关闭连接；OKX 私有流没有 Binance listenKey 式 REST 关闭动作。
    pub async fn close(mut self) -> Result<(), Error> {
        self.socket
            .send(Message::Close(None))
            .await
            .map_err(|error| Error::WebSocketError(format!("关闭 OKX 私有流失败: {error}")))
    }

    async fn wait_for_pong(&mut self) -> Result<(), Error> {
        loop {
            let Some(message) = self.socket.next().await else {
                return Err(Error::ConnectionError(
                    "OKX 在 heartbeat 期间关闭连接".to_string(),
                ));
            };
            match message.map_err(|error| Error::WebSocketError(error.to_string()))? {
                Message::Text(text) if text.as_str() == "pong" => return Ok(()),
                Message::Text(text) => {
                    let value = serde_json::from_str(text.as_str())?;
                    buffer_business_frame(&mut self.pending, value)?;
                }
                Message::Binary(bytes) => {
                    let text = std::str::from_utf8(&bytes).map_err(|error| {
                        Error::WebSocketError(format!("OKX binary frame 不是 UTF-8: {error}"))
                    })?;
                    let value = serde_json::from_str(text)?;
                    buffer_business_frame(&mut self.pending, value)?;
                }
                Message::Ping(payload) => self
                    .socket
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|error| Error::WebSocketError(error.to_string()))?,
                Message::Close(_) => {
                    return Err(Error::ConnectionError(
                        "OKX 在 heartbeat 期间关闭连接".to_string(),
                    ));
                }
                Message::Pong(_) | Message::Frame(_) => {}
            }
        }
    }
}

async fn send_login(socket: &mut OkxPrivateSocket, credentials: &Credentials) -> Result<(), Error> {
    let timestamp = generate_timestamp_websocket();
    let sign = generate_signature(
        &credentials.api_secret,
        &timestamp,
        &Method::GET,
        "/users/self/verify",
        "",
    )?;
    send_json(
        socket,
        json!({
            "op": "login",
            "args": [{
                "apiKey": credentials.api_key,
                "passphrase": credentials.passphrase,
                "timestamp": timestamp,
                "sign": sign
            }]
        }),
    )
    .await
}

async fn send_subscriptions(socket: &mut OkxPrivateSocket) -> Result<(), Error> {
    send_json(
        socket,
        json!({
            "op": "subscribe",
            "args": [
                {"channel": "account", "ccy": "USDT"},
                {"channel": "positions", "instType": "SWAP"},
                {"channel": "orders", "instType": "SWAP"}
            ]
        }),
    )
    .await
}

async fn wait_for_login(
    socket: &mut OkxPrivateSocket,
    pending: &mut VecDeque<Value>,
) -> Result<(), Error> {
    timeout(HANDSHAKE_TIMEOUT, wait_for_login_inner(socket, pending))
        .await
        .map_err(|_| Error::TimeoutError("等待 OKX private login ACK 超时".to_string()))?
}

async fn wait_for_login_inner(
    socket: &mut OkxPrivateSocket,
    pending: &mut VecDeque<Value>,
) -> Result<(), Error> {
    loop {
        let value = handshake_frame(socket).await?;
        if value.get("event").and_then(Value::as_str) == Some("login") {
            return ensure_ack_success(&value, "login", true);
        }
        buffer_business_frame(pending, value)?;
    }
}

async fn wait_for_subscriptions(
    socket: &mut OkxPrivateSocket,
    pending: &mut VecDeque<Value>,
) -> Result<(), Error> {
    timeout(
        HANDSHAKE_TIMEOUT,
        wait_for_subscriptions_inner(socket, pending),
    )
    .await
    .map_err(|_| Error::TimeoutError("等待 OKX private subscribe ACK 超时".to_string()))?
}

async fn wait_for_subscriptions_inner(
    socket: &mut OkxPrivateSocket,
    pending: &mut VecDeque<Value>,
) -> Result<(), Error> {
    let mut confirmed = HashSet::new();
    while confirmed.len() < 3 {
        let value = handshake_frame(socket).await?;
        if value.get("event").and_then(Value::as_str) == Some("subscribe") {
            ensure_ack_success(&value, "subscribe", false)?;
            let channel = value
                .pointer("/arg/channel")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    Error::SubscriptionError("OKX subscribe ACK 缺少 arg.channel".to_string())
                })?;
            if !matches!(channel, "account" | "positions" | "orders") {
                return Err(Error::SubscriptionError(format!(
                    "OKX 返回了未请求频道的 ACK: {channel}"
                )));
            }
            confirmed.insert(channel.to_string());
        } else {
            buffer_business_frame(pending, value)?;
        }
    }
    Ok(())
}

async fn handshake_frame(socket: &mut OkxPrivateSocket) -> Result<Value, Error> {
    recv_json(socket)
        .await?
        .ok_or_else(|| Error::ConnectionError("OKX 在握手期间关闭连接".to_string()))
}

fn ensure_ack_success(value: &Value, phase: &str, code_required: bool) -> Result<(), Error> {
    let code = value.get("code").and_then(Value::as_str);
    if code == Some("0") || (!code_required && code.is_none()) {
        return Ok(());
    }
    Err(Error::AuthenticationError(format!(
        "OKX {phase} 被拒绝: code={} msg={}",
        code.unwrap_or("missing"),
        value
            .get("msg")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    )))
}

fn buffer_business_frame(pending: &mut VecDeque<Value>, value: Value) -> Result<(), Error> {
    if value.get("event").is_some() {
        return Err(Error::SubscriptionError(format!(
            "OKX 握手收到意外控制消息: {value}"
        )));
    }
    if pending.len() >= MAX_HANDSHAKE_BUFFERED_FRAMES {
        return Err(Error::WebSocketError(
            "OKX 握手业务 frame 缓冲达到 64 条上限".to_string(),
        ));
    }
    pending.push_back(value);
    Ok(())
}

async fn send_json(socket: &mut OkxPrivateSocket, value: Value) -> Result<(), Error> {
    socket
        .send(Message::Text(value.to_string().into()))
        .await
        .map_err(|error| Error::WebSocketError(format!("发送 OKX 私有流消息失败: {error}")))
}

async fn recv_json(socket: &mut OkxPrivateSocket) -> Result<Option<Value>, Error> {
    loop {
        let Some(message) = socket.next().await else {
            return Ok(None);
        };
        match message.map_err(|error| Error::WebSocketError(error.to_string()))? {
            Message::Text(text) if text.as_str() == "pong" => continue,
            Message::Text(text) => {
                return serde_json::from_str(text.as_str())
                    .map(Some)
                    .map_err(Into::into)
            }
            Message::Binary(bytes) => {
                let text = std::str::from_utf8(&bytes).map_err(|error| {
                    Error::WebSocketError(format!("OKX binary frame 不是 UTF-8: {error}"))
                })?;
                return serde_json::from_str(text).map(Some).map_err(Into::into);
            }
            Message::Ping(payload) => socket
                .send(Message::Pong(payload))
                .await
                .map_err(|error| Error::WebSocketError(error.to_string()))?,
            Message::Close(_) => return Ok(None),
            Message::Pong(_) | Message::Frame(_) => {}
        }
    }
}
