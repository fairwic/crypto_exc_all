use crate::config::{Credentials, DEFAULT_BUSINESS_WEBSOCKET_URL, DEFAULT_PRIVATE_WEBSOCKET_URL};
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
    private_url: String,
    business_url: String,
}

impl OkxPrivateAccountStreamClient {
    /// 使用官方私有 WebSocket 地址创建连接器。
    pub fn new(credentials: Credentials) -> Self {
        Self {
            credentials,
            private_url: DEFAULT_PRIVATE_WEBSOCKET_URL.to_string(),
            business_url: DEFAULT_BUSINESS_WEBSOCKET_URL.to_string(),
        }
    }

    /// 覆盖私有流地址，供显式环境配置与本地协议测试使用。
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.private_url = url.into();
        self
    }

    /// 覆盖业务私有流地址；`orders-algo` 按 OKX 合同只存在于该端点。
    pub fn with_business_url(mut self, url: impl Into<String>) -> Self {
        self.business_url = url.into();
        self
    }

    /// 建立 private/business 双连接并等待四个必需频道全部确认。
    ///
    /// ACK 期间到达的业务 frame 会先缓存，避免“订阅成功”确认与首批账户事实
    /// 交错时丢数据；重连与恢复由 Account owner 决定。
    pub async fn connect(&self) -> Result<OkxPrivateAccountStreamSession, Error> {
        let (mut private_socket, mut private_pending) =
            open_authenticated_socket(&self.private_url, &self.credentials, "private").await?;
        send_subscriptions(
            &mut private_socket,
            json!([
                {"channel": "account", "ccy": "USDT"},
                {"channel": "positions", "instType": "SWAP"},
                {"channel": "orders", "instType": "SWAP"}
            ]),
        )
        .await?;
        wait_for_subscriptions(
            &mut private_socket,
            &mut private_pending,
            &["account", "positions", "orders"],
            "private",
        )
        .await?;

        let (mut business_socket, mut business_pending) =
            open_authenticated_socket(&self.business_url, &self.credentials, "business").await?;
        send_subscriptions(
            &mut business_socket,
            json!([{"channel": "orders-algo", "instType": "ANY"}]),
        )
        .await?;
        wait_for_subscriptions(
            &mut business_socket,
            &mut business_pending,
            &["orders-algo"],
            "business",
        )
        .await?;

        Ok(OkxPrivateAccountStreamSession {
            private_socket,
            business_socket,
            private_pending,
            business_pending,
        })
    }
}

/// 已完成认证与 P0 频道确认的 OKX 私有账户双连接会话。
pub struct OkxPrivateAccountStreamSession {
    private_socket: OkxPrivateSocket,
    business_socket: OkxPrivateSocket,
    private_pending: VecDeque<Value>,
    business_pending: VecDeque<Value>,
}

impl OkxPrivateAccountStreamSession {
    /// 按 provider 顺序读取下一条 JSON，优先返回握手期间缓存的业务 frame。
    pub async fn recv_json(&mut self) -> Result<Option<Value>, Error> {
        if let Some(value) = self.private_pending.pop_front() {
            return Ok(Some(value));
        }
        if let Some(value) = self.business_pending.pop_front() {
            return Ok(Some(value));
        }
        tokio::select! {
            value = recv_json(&mut self.private_socket) => value,
            value = recv_json(&mut self.business_socket) => value,
        }
    }

    /// 对两个必需端点发送 `ping` 并等待 `pong`；任一失败都会使整条账户流失败。
    pub async fn heartbeat(&mut self) -> Result<(), Error> {
        self.private_socket
            .send(Message::Text("ping".into()))
            .await
            .map_err(|error| {
                Error::WebSocketError(format!("发送 OKX private heartbeat 失败: {error}"))
            })?;
        self.business_socket
            .send(Message::Text("ping".into()))
            .await
            .map_err(|error| {
                Error::WebSocketError(format!("发送 OKX business heartbeat 失败: {error}"))
            })?;
        timeout(HANDSHAKE_TIMEOUT, async {
            wait_for_pong(
                &mut self.private_socket,
                &mut self.private_pending,
                "private",
            )
            .await?;
            wait_for_pong(
                &mut self.business_socket,
                &mut self.business_pending,
                "business",
            )
            .await
        })
        .await
        .map_err(|_| Error::TimeoutError("等待 OKX heartbeat pong 超时".to_string()))?
    }

    /// 主动关闭两个连接；OKX 私有流没有 Binance listenKey 式 REST 关闭动作。
    pub async fn close(mut self) -> Result<(), Error> {
        let private_result = self
            .private_socket
            .send(Message::Close(None))
            .await
            .map_err(|error| Error::WebSocketError(format!("关闭 OKX private 流失败: {error}")));
        let business_result = self
            .business_socket
            .send(Message::Close(None))
            .await
            .map_err(|error| Error::WebSocketError(format!("关闭 OKX business 流失败: {error}")));
        private_result.and(business_result)
    }
}

async fn open_authenticated_socket(
    url: &str,
    credentials: &Credentials,
    endpoint: &str,
) -> Result<(OkxPrivateSocket, VecDeque<Value>), Error> {
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_PRIVATE_MESSAGE_BYTES))
        .max_frame_size(Some(MAX_PRIVATE_MESSAGE_BYTES));
    let (mut socket, _) = timeout(
        HANDSHAKE_TIMEOUT,
        connect_async_with_config(url, Some(config), false),
    )
    .await
    .map_err(|_| Error::TimeoutError(format!("连接 OKX {endpoint} 私有流超时")))?
    .map_err(|error| Error::WebSocketError(format!("连接 OKX {endpoint} 私有流失败: {error}")))?;
    send_login(&mut socket, credentials).await?;
    let mut pending = VecDeque::new();
    wait_for_login(&mut socket, &mut pending, endpoint).await?;
    Ok((socket, pending))
}

async fn wait_for_pong(
    socket: &mut OkxPrivateSocket,
    pending: &mut VecDeque<Value>,
    endpoint: &str,
) -> Result<(), Error> {
    loop {
        let Some(message) = socket.next().await else {
            return Err(Error::ConnectionError(format!(
                "OKX {endpoint} 在 heartbeat 期间关闭连接"
            )));
        };
        match message.map_err(|error| Error::WebSocketError(error.to_string()))? {
            Message::Text(text) if text.as_str() == "pong" => return Ok(()),
            Message::Text(text) => {
                let value = serde_json::from_str(text.as_str())?;
                buffer_provider_frame(pending, value)?;
            }
            Message::Binary(bytes) => {
                let text = std::str::from_utf8(&bytes).map_err(|error| {
                    Error::WebSocketError(format!("OKX binary frame 不是 UTF-8: {error}"))
                })?;
                let value = serde_json::from_str(text)?;
                buffer_provider_frame(pending, value)?;
            }
            Message::Ping(payload) => socket
                .send(Message::Pong(payload))
                .await
                .map_err(|error| Error::WebSocketError(error.to_string()))?,
            Message::Close(_) => {
                return Err(Error::ConnectionError(format!(
                    "OKX {endpoint} 在 heartbeat 期间关闭连接"
                )));
            }
            Message::Pong(_) | Message::Frame(_) => {}
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

async fn send_subscriptions(socket: &mut OkxPrivateSocket, args: Value) -> Result<(), Error> {
    send_json(
        socket,
        json!({
            "op": "subscribe",
            "args": args
        }),
    )
    .await
}

async fn wait_for_login(
    socket: &mut OkxPrivateSocket,
    pending: &mut VecDeque<Value>,
    endpoint: &str,
) -> Result<(), Error> {
    timeout(HANDSHAKE_TIMEOUT, wait_for_login_inner(socket, pending))
        .await
        .map_err(|_| Error::TimeoutError(format!("等待 OKX {endpoint} login ACK 超时")))?
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
        buffer_provider_frame(pending, value)?;
    }
}

async fn wait_for_subscriptions(
    socket: &mut OkxPrivateSocket,
    pending: &mut VecDeque<Value>,
    expected_channels: &[&str],
    endpoint: &str,
) -> Result<(), Error> {
    timeout(
        HANDSHAKE_TIMEOUT,
        wait_for_subscriptions_inner(socket, pending, expected_channels),
    )
    .await
    .map_err(|_| Error::TimeoutError(format!("等待 OKX {endpoint} subscribe ACK 超时")))?
}

async fn wait_for_subscriptions_inner(
    socket: &mut OkxPrivateSocket,
    pending: &mut VecDeque<Value>,
    expected_channels: &[&str],
) -> Result<(), Error> {
    let mut confirmed = HashSet::new();
    while confirmed.len() < expected_channels.len() {
        let value = handshake_frame(socket).await?;
        if value.get("event").and_then(Value::as_str) == Some("subscribe") {
            ensure_ack_success(&value, "subscribe", false)?;
            let channel = value
                .pointer("/arg/channel")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    Error::SubscriptionError("OKX subscribe ACK 缺少 arg.channel".to_string())
                })?;
            if !expected_channels.contains(&channel) {
                return Err(Error::SubscriptionError(format!(
                    "OKX 返回了未请求频道的 ACK: {channel}"
                )));
            }
            confirmed.insert(channel.to_string());
        } else {
            buffer_provider_frame(pending, value)?;
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

fn buffer_provider_frame(pending: &mut VecDeque<Value>, value: Value) -> Result<(), Error> {
    if value.get("event").and_then(Value::as_str) == Some("channel-conn-count-error") {
        return Err(Error::SubscriptionError(
            "OKX 私有频道连接数达到上限".to_string(),
        ));
    }
    if value.get("event").is_some()
        && value.get("event").and_then(Value::as_str) != Some("channel-conn-count")
    {
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
