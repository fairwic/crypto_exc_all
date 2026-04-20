use crate::api::api_trait::BinanceApiTrait;
use crate::client::BinanceClient;
use crate::config::{Config, Credentials, DEFAULT_WS_STREAM_URL};
use crate::error::Error;
use futures_util::{SinkExt, StreamExt};
use reqwest::Method;
use serde::{Deserialize, Deserializer};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, client_async_tls_with_config, connect_async,
};

const LISTEN_KEY_PATH: &str = "/fapi/v1/listenKey";
const WEBSOCKET_CHANNEL_SIZE: usize = 100;

#[derive(Clone)]
pub struct BinanceWebsocket {
    client: BinanceClient,
    stream_base_url: String,
    proxy_url: Option<String>,
}

impl BinanceApiTrait for BinanceWebsocket {
    fn new(client: BinanceClient) -> Self {
        Self {
            client,
            stream_base_url: DEFAULT_WS_STREAM_URL.to_string(),
            proxy_url: None,
        }
    }

    fn from_env() -> Result<Self, Error> {
        let config = Config::from_env();
        let stream_base_url = config.ws_stream_url.clone();
        let proxy_url = config.proxy_url.clone();
        let client = BinanceClient::with_config(Some(Credentials::from_env()?), config)?;
        Ok(Self {
            client,
            stream_base_url,
            proxy_url,
        })
    }

    fn client(&self) -> &BinanceClient {
        &self.client
    }
}

impl BinanceWebsocket {
    pub fn new(client: BinanceClient) -> Self {
        <Self as BinanceApiTrait>::new(client)
    }

    pub fn from_env() -> Result<Self, Error> {
        <Self as BinanceApiTrait>::from_env()
    }

    pub fn new_public_with_stream_base_url(stream_base_url: impl Into<String>) -> Self {
        Self {
            client: BinanceClient::new_public()
                .expect("public Binance websocket URL builder should not require credentials"),
            stream_base_url: stream_base_url.into(),
            proxy_url: None,
        }
    }

    pub fn with_proxy_url(mut self, proxy_url: impl Into<String>) -> Self {
        self.proxy_url = Some(proxy_url.into());
        self
    }

    pub async fn start_user_data_stream(&self) -> Result<serde_json::Value, Error> {
        self.client
            .send_api_key_request(Method::POST, LISTEN_KEY_PATH, &[])
            .await
    }

    pub async fn keepalive_user_data_stream(&self) -> Result<serde_json::Value, Error> {
        self.client
            .send_api_key_request(Method::PUT, LISTEN_KEY_PATH, &[])
            .await
    }

    pub async fn close_user_data_stream(&self) -> Result<serde_json::Value, Error> {
        self.client
            .send_api_key_request(Method::DELETE, LISTEN_KEY_PATH, &[])
            .await
    }

    pub async fn connect_url(&self, url: &str) -> Result<BinanceWebsocketSession, Error> {
        BinanceWebsocketSession::connect_with_proxy(url, self.proxy_url.as_deref()).await
    }

    pub fn public_ws_url<S: AsRef<str>>(&self, streams: &[S]) -> String {
        self.ws_path_url("public", "ws", streams)
    }

    pub fn public_stream_url<S: AsRef<str>>(&self, streams: &[S]) -> String {
        self.stream_query_url("public", streams)
    }

    pub fn public_route_ws_url(&self) -> String {
        self.route_ws_url("public")
    }

    pub fn market_ws_url<S: AsRef<str>>(&self, streams: &[S]) -> String {
        self.ws_path_url("market", "ws", streams)
    }

    pub fn market_stream_url<S: AsRef<str>>(&self, streams: &[S]) -> String {
        self.stream_query_url("market", streams)
    }

    pub fn market_route_ws_url(&self) -> String {
        self.route_ws_url("market")
    }

    pub fn private_route_ws_url(&self) -> String {
        self.route_ws_url("private")
    }

    pub fn private_ws_url<S: AsRef<str>>(&self, listen_key: &str, events: &[S]) -> String {
        let mut url = format!(
            "{}/private/ws?listenKey={}",
            self.stream_base_url(),
            listen_key
        );
        let events = join_streams(events);
        if !events.is_empty() {
            url.push_str("&events=");
            url.push_str(&events);
        }
        url
    }

    pub fn private_stream_url<S: AsRef<str>>(&self, listen_key: &str, events: &[S]) -> String {
        let mut url = format!(
            "{}/private/stream?listenKey={}",
            self.stream_base_url(),
            listen_key
        );
        let events = join_streams(events);
        if !events.is_empty() {
            url.push_str("&events=");
            url.push_str(&events);
        }
        url
    }

    fn ws_path_url<S: AsRef<str>>(&self, route: &str, mode: &str, streams: &[S]) -> String {
        format!(
            "{}/{}/{}/{}",
            self.stream_base_url(),
            route,
            mode,
            join_streams(streams)
        )
    }

    fn stream_query_url<S: AsRef<str>>(&self, route: &str, streams: &[S]) -> String {
        format!(
            "{}/{}/stream?streams={}",
            self.stream_base_url(),
            route,
            join_streams(streams)
        )
    }

    fn route_ws_url(&self, route: &str) -> String {
        format!("{}/{route}/ws", self.stream_base_url())
    }

    fn stream_base_url(&self) -> &str {
        self.stream_base_url.trim_end_matches('/')
    }
}

fn join_streams<S: AsRef<str>>(streams: &[S]) -> String {
    streams
        .iter()
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join("/")
}

async fn connect_websocket(
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

pub struct BinanceWebsocketSession {
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<Value>,
    request_id: Arc<AtomicU64>,
}

impl BinanceWebsocketSession {
    pub async fn connect(url: &str) -> Result<Self, Error> {
        Self::connect_with_proxy(url, None).await
    }

    pub async fn connect_with_proxy(url: &str, proxy_url: Option<&str>) -> Result<Self, Error> {
        let (stream, _) = connect_websocket(url, proxy_url).await?;
        let (mut write, mut read) = stream.split();
        let (tx_in, mut rx_in) = mpsc::channel::<Message>(WEBSOCKET_CHANNEL_SIZE);
        let (tx_out, rx_out) = mpsc::channel::<Value>(WEBSOCKET_CHANNEL_SIZE);
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
                        if forward_json(text.as_str(), &tx_out).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Binary(bytes)) => {
                        let Ok(text) = std::str::from_utf8(&bytes) else {
                            continue;
                        };
                        if forward_json(text, &tx_out).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Ping(payload)) => {
                        if ping_tx.send(Message::Pong(payload)).await.is_err() {
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
            request_id: Arc::new(AtomicU64::new(1)),
        })
    }

    pub async fn recv_json(&mut self) -> Option<Value> {
        self.rx.recv().await
    }

    pub async fn subscribe<S: AsRef<str>>(&self, streams: &[S]) -> Result<(), Error> {
        self.send_operation("SUBSCRIBE", streams).await
    }

    pub async fn unsubscribe<S: AsRef<str>>(&self, streams: &[S]) -> Result<(), Error> {
        self.send_operation("UNSUBSCRIBE", streams).await
    }

    pub async fn close(&self) -> Result<(), Error> {
        self.tx
            .send(Message::Close(None))
            .await
            .map_err(|err| Error::WebSocketError(format!("发送关闭消息失败: {err}")))
    }

    async fn send_operation<S: AsRef<str>>(
        &self,
        method: &str,
        streams: &[S],
    ) -> Result<(), Error> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let payload = websocket_operation(method, streams, id);
        self.tx
            .send(Message::Text(payload.into()))
            .await
            .map_err(|err| Error::WebSocketError(format!("发送订阅消息失败: {err}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectConfig {
    pub reconnect_interval: Duration,
    pub max_reconnect_attempts: u32,
}

impl ReconnectConfig {
    pub fn new(reconnect_interval: Duration, max_reconnect_attempts: u32) -> Self {
        Self {
            reconnect_interval,
            max_reconnect_attempts,
        }
    }
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self::new(Duration::from_secs(5), 10)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Stopped,
}

#[derive(Debug, Clone, Default)]
pub struct WebsocketMetrics {
    pub connected_at: Option<Instant>,
    pub last_message_at: Option<Instant>,
    pub messages_received: u64,
    pub reconnects: u64,
    pub connection_attempts: u64,
    pub last_error: Option<String>,
}

pub struct BinanceWebsocketManager {
    url: String,
    config: ReconnectConfig,
    subscriptions: Vec<String>,
    proxy_url: Option<String>,
    state_tx: watch::Sender<ConnectionState>,
    metrics_tx: watch::Sender<WebsocketMetrics>,
    stop_tx: Option<watch::Sender<bool>>,
    task: Option<JoinHandle<()>>,
}

impl BinanceWebsocketManager {
    pub fn new(url: impl Into<String>, config: ReconnectConfig) -> Self {
        let (state_tx, _) = watch::channel(ConnectionState::Disconnected);
        let (metrics_tx, _) = watch::channel(WebsocketMetrics::default());
        Self {
            url: url.into(),
            config,
            subscriptions: Vec::new(),
            proxy_url: None,
            state_tx,
            metrics_tx,
            stop_tx: None,
            task: None,
        }
    }

    pub fn with_proxy_url(mut self, proxy_url: impl Into<String>) -> Self {
        self.proxy_url = Some(proxy_url.into());
        self
    }

    pub fn add_subscription(&mut self, stream: impl Into<String>) {
        let stream = stream.into();
        if !self.subscriptions.contains(&stream) {
            self.subscriptions.push(stream);
        }
    }

    pub fn subscriptions(&self) -> &[String] {
        &self.subscriptions
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.state_tx.borrow().clone()
    }

    pub fn metrics(&self) -> WebsocketMetrics {
        self.metrics_tx.borrow().clone()
    }

    pub fn is_healthy(&self, max_message_age: Duration) -> bool {
        if self.connection_state() != ConnectionState::Connected {
            return false;
        }

        self.metrics()
            .last_message_at
            .map(|last_message_at| last_message_at.elapsed() <= max_message_age)
            .unwrap_or(false)
    }

    pub async fn start(&mut self) -> Result<mpsc::Receiver<Value>, Error> {
        if self.task.is_some() {
            return Err(Error::WebSocketError(
                "WebSocket manager 已经启动".to_string(),
            ));
        }

        let (message_tx, message_rx) = mpsc::channel::<Value>(WEBSOCKET_CHANNEL_SIZE);
        let (stop_tx, stop_rx) = watch::channel(false);
        self.stop_tx = Some(stop_tx);

        let url = self.url.clone();
        let config = self.config.clone();
        let subscriptions = self.subscriptions.clone();
        let proxy_url = self.proxy_url.clone();
        let state_tx = self.state_tx.clone();
        let metrics_tx = self.metrics_tx.clone();
        self.task = Some(tokio::spawn(async move {
            let context = ReconnectLoopContext {
                url,
                config,
                subscriptions,
                proxy_url,
                message_tx,
                stop_rx,
                state_tx,
                metrics_tx,
            };
            run_reconnect_loop(context).await;
        }));

        Ok(message_rx)
    }

    pub async fn stop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(true);
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        self.state_tx.send_replace(ConnectionState::Stopped);
    }
}

struct ReconnectLoopContext {
    url: String,
    config: ReconnectConfig,
    subscriptions: Vec<String>,
    proxy_url: Option<String>,
    message_tx: mpsc::Sender<Value>,
    stop_rx: watch::Receiver<bool>,
    state_tx: watch::Sender<ConnectionState>,
    metrics_tx: watch::Sender<WebsocketMetrics>,
}

async fn run_reconnect_loop(mut context: ReconnectLoopContext) {
    let mut attempts = 0;

    while !*context.stop_rx.borrow() && attempts <= context.config.max_reconnect_attempts {
        context.state_tx.send_replace(if attempts == 0 {
            ConnectionState::Connecting
        } else {
            ConnectionState::Reconnecting
        });
        match connect_websocket(&context.url, context.proxy_url.as_deref()).await {
            Ok((stream, _)) => {
                context.state_tx.send_replace(ConnectionState::Connected);
                update_metrics(&context.metrics_tx, |metrics| {
                    metrics.connected_at = Some(Instant::now());
                    if metrics.connection_attempts > 0 {
                        metrics.reconnects += 1;
                    }
                    metrics.connection_attempts += 1;
                    metrics.last_error = None;
                });
                attempts = 0;
                let should_reconnect = run_connected_socket(
                    stream,
                    &context.subscriptions,
                    &context.message_tx,
                    &mut context.stop_rx,
                    &context.metrics_tx,
                )
                .await;
                if !should_reconnect {
                    break;
                }
            }
            Err(err) => {
                let message = err.to_string();
                update_metrics(&context.metrics_tx, |metrics| {
                    metrics.last_error = Some(message);
                    metrics.connection_attempts += 1;
                });
                attempts += 1;
            }
        }

        if *context.stop_rx.borrow() || attempts > context.config.max_reconnect_attempts {
            break;
        }

        tokio::select! {
            _ = sleep(context.config.reconnect_interval) => {}
            _ = context.stop_rx.changed() => {}
        }
    }
    context.state_tx.send_replace(if *context.stop_rx.borrow() {
        ConnectionState::Stopped
    } else {
        ConnectionState::Disconnected
    });
}

async fn run_connected_socket<S>(
    stream: tokio_tungstenite::WebSocketStream<S>,
    subscriptions: &[String],
    message_tx: &mpsc::Sender<Value>,
    stop_rx: &mut watch::Receiver<bool>,
    metrics_tx: &watch::Sender<WebsocketMetrics>,
) -> bool
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (mut write, mut read) = stream.split();
    if !subscriptions.is_empty() {
        let payload = websocket_operation("SUBSCRIBE", subscriptions, 1);
        if write.send(Message::Text(payload.into())).await.is_err() {
            return true;
        }
    }

    loop {
        tokio::select! {
            _ = stop_rx.changed() => {
                let _ = write.send(Message::Close(None)).await;
                return false;
            }
            message = read.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        if forward_json(text.as_str(), message_tx).await.is_err() {
                            return false;
                        }
                        record_message(metrics_tx);
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        let Ok(text) = std::str::from_utf8(&bytes) else {
                            continue;
                        };
                        if forward_json(text, message_tx).await.is_err() {
                            return false;
                        }
                        record_message(metrics_tx);
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if write.send(Message::Pong(payload)).await.is_err() {
                            return true;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return true,
                    _ => {}
                }
            }
        }
    }
}

fn websocket_operation<S: AsRef<str>>(method: &str, streams: &[S], id: u64) -> String {
    let params: Vec<&str> = streams.iter().map(AsRef::as_ref).collect();
    json!({
        "method": method,
        "params": params,
        "id": id
    })
    .to_string()
}

fn update_metrics<F>(metrics_tx: &watch::Sender<WebsocketMetrics>, update: F)
where
    F: FnOnce(&mut WebsocketMetrics),
{
    let mut metrics = metrics_tx.borrow().clone();
    update(&mut metrics);
    metrics_tx.send_replace(metrics);
}

fn record_message(metrics_tx: &watch::Sender<WebsocketMetrics>) {
    update_metrics(metrics_tx, |metrics| {
        metrics.messages_received += 1;
        metrics.last_message_at = Some(Instant::now());
    });
}

async fn forward_json(text: &str, tx: &mpsc::Sender<Value>) -> Result<(), ()> {
    match serde_json::from_str::<Value>(text) {
        Ok(value) => tx.send(value).await.map_err(|_| ()),
        Err(_) => Ok(()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BinanceStreamRoute {
    Public,
    Market,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSubscription {
    pub route: BinanceStreamRoute,
    pub url: String,
}

impl StreamSubscription {
    pub fn public(stream: impl Into<String>) -> Self {
        Self {
            route: BinanceStreamRoute::Public,
            url: stream.into(),
        }
    }

    pub fn market(stream: impl Into<String>) -> Self {
        Self {
            route: BinanceStreamRoute::Market,
            url: stream.into(),
        }
    }

    pub fn private(listen_key: impl AsRef<str>, events: &[impl AsRef<str>]) -> Self {
        let events = join_streams(events);
        let url = if events.is_empty() {
            format!("listenKey={}", listen_key.as_ref())
        } else {
            format!("listenKey={}&events={events}", listen_key.as_ref())
        };

        Self {
            route: BinanceStreamRoute::Private,
            url,
        }
    }
}

pub struct BinanceWebsocketHub {
    public_url: Option<String>,
    market_url: Option<String>,
    private_url: Option<String>,
    proxy_url: Option<String>,
    reconnect_config: ReconnectConfig,
}

impl BinanceWebsocketHub {
    pub fn new() -> Self {
        Self {
            public_url: None,
            market_url: None,
            private_url: None,
            proxy_url: None,
            reconnect_config: ReconnectConfig::default(),
        }
    }

    pub fn with_route_url(mut self, route: BinanceStreamRoute, url: impl Into<String>) -> Self {
        match route {
            BinanceStreamRoute::Public => self.public_url = Some(url.into()),
            BinanceStreamRoute::Market => self.market_url = Some(url.into()),
            BinanceStreamRoute::Private => self.private_url = Some(url.into()),
        }
        self
    }

    pub fn with_proxy_url(mut self, proxy_url: impl Into<String>) -> Self {
        self.proxy_url = Some(proxy_url.into());
        self
    }

    pub fn with_reconnect_config(mut self, reconnect_config: ReconnectConfig) -> Self {
        self.reconnect_config = reconnect_config;
        self
    }

    pub async fn start(
        self,
        subscriptions: Vec<StreamSubscription>,
    ) -> Result<mpsc::Receiver<Value>, Error> {
        let (output_tx, output_rx) = mpsc::channel::<Value>(WEBSOCKET_CHANNEL_SIZE);

        for route in [
            BinanceStreamRoute::Public,
            BinanceStreamRoute::Market,
            BinanceStreamRoute::Private,
        ] {
            let route_subscriptions: Vec<String> = subscriptions
                .iter()
                .filter(|subscription| subscription.route == route)
                .map(|subscription| subscription.url.clone())
                .collect();
            if route_subscriptions.is_empty() {
                continue;
            }

            let url = self
                .route_url(&route)
                .ok_or_else(|| Error::WebSocketError(format!("缺少 {route:?} WebSocket URL")))?;
            let should_send_subscribe = !route_url_embeds_subscription(&url);
            let mut manager = BinanceWebsocketManager::new(url, self.reconnect_config.clone());
            if let Some(proxy_url) = self.proxy_url.clone() {
                manager = manager.with_proxy_url(proxy_url);
            }
            if should_send_subscribe {
                for subscription in route_subscriptions {
                    manager.add_subscription(subscription);
                }
            }

            let mut route_rx = manager.start().await?;
            let route_output_tx = output_tx.clone();
            tokio::spawn(async move {
                let _manager = manager;
                while let Some(message) = route_rx.recv().await {
                    if route_output_tx.send(message).await.is_err() {
                        break;
                    }
                }
            });
        }

        Ok(output_rx)
    }

    fn route_url(&self, route: &BinanceStreamRoute) -> Option<String> {
        match route {
            BinanceStreamRoute::Public => self.public_url.clone(),
            BinanceStreamRoute::Market => self.market_url.clone(),
            BinanceStreamRoute::Private => self.private_url.clone(),
        }
    }
}

fn route_url_embeds_subscription(url: &str) -> bool {
    url.contains("?streams=") || url.contains("&streams=") || url.contains("listenKey=")
}

impl Default for BinanceWebsocketHub {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinanceWebsocketEvent {
    ListenKeyExpired(ListenKeyExpiredEvent),
    MarginCall(MarginCallEvent),
    OrderTradeUpdate(OrderTradeUpdateEvent),
    TradeLite(TradeLiteEvent),
    AccountUpdate(AccountUpdateEvent),
    AccountConfigUpdate(AccountConfigUpdateEvent),
    StrategyUpdate(StrategyUpdateEvent),
    GridUpdate(GridUpdateEvent),
    ConditionalOrderTriggerReject(ConditionalOrderTriggerRejectEvent),
    AlgoUpdate(Box<AlgoUpdateEvent>),
    Raw(Value),
}

impl BinanceWebsocketEvent {
    pub fn parse(value: Value) -> Result<Self, Error> {
        let typed_payload = value
            .get("data")
            .filter(|data| data.get("e").is_some())
            .cloned()
            .unwrap_or_else(|| value.clone());
        let event = typed_payload.get("e").and_then(Value::as_str);

        match event {
            Some("listenKeyExpired") => {
                serde_json::from_value::<ListenKeyExpiredEvent>(typed_payload)
                    .map(Self::ListenKeyExpired)
                    .map_err(Error::JsonError)
            }
            Some("MARGIN_CALL") => serde_json::from_value::<MarginCallEvent>(typed_payload)
                .map(Self::MarginCall)
                .map_err(Error::JsonError),
            Some("ORDER_TRADE_UPDATE") => {
                serde_json::from_value::<OrderTradeUpdateEvent>(typed_payload)
                    .map(Self::OrderTradeUpdate)
                    .map_err(Error::JsonError)
            }
            Some("TRADE_LITE") => serde_json::from_value::<TradeLiteEvent>(typed_payload)
                .map(Self::TradeLite)
                .map_err(Error::JsonError),
            Some("ACCOUNT_UPDATE") => serde_json::from_value::<AccountUpdateEvent>(typed_payload)
                .map(Self::AccountUpdate)
                .map_err(Error::JsonError),
            Some("ACCOUNT_CONFIG_UPDATE") => {
                serde_json::from_value::<AccountConfigUpdateEvent>(typed_payload)
                    .map(Self::AccountConfigUpdate)
                    .map_err(Error::JsonError)
            }
            Some("STRATEGY_UPDATE") => serde_json::from_value::<StrategyUpdateEvent>(typed_payload)
                .map(Self::StrategyUpdate)
                .map_err(Error::JsonError),
            Some("GRID_UPDATE") => serde_json::from_value::<GridUpdateEvent>(typed_payload)
                .map(Self::GridUpdate)
                .map_err(Error::JsonError),
            Some("CONDITIONAL_ORDER_TRIGGER_REJECT") => {
                serde_json::from_value::<ConditionalOrderTriggerRejectEvent>(typed_payload)
                    .map(Self::ConditionalOrderTriggerReject)
                    .map_err(Error::JsonError)
            }
            Some("ALGO_UPDATE") => serde_json::from_value::<AlgoUpdateEvent>(typed_payload)
                .map(Box::new)
                .map(Self::AlgoUpdate)
                .map_err(Error::JsonError),
            _ => Ok(Self::Raw(value)),
        }
    }
}

fn deserialize_u64_from_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(number) => number
            .as_u64()
            .ok_or_else(|| serde::de::Error::custom("expected unsigned integer")),
        Value::String(text) => text
            .parse::<u64>()
            .map_err(|err| serde::de::Error::custom(format!("expected unsigned integer: {err}"))),
        _ => Err(serde::de::Error::custom(
            "expected unsigned integer as number or string",
        )),
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ListenKeyExpiredEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(rename = "listenKey")]
    pub listen_key: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MarginCallEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(rename = "cw", default)]
    pub cross_wallet_balance: Option<String>,
    #[serde(rename = "p", default)]
    pub positions: Vec<MarginCallPosition>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MarginCallPosition {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "ps")]
    pub position_side: String,
    #[serde(rename = "pa")]
    pub position_amount: String,
    #[serde(rename = "mt")]
    pub margin_type: String,
    #[serde(rename = "iw", default)]
    pub isolated_wallet: Option<String>,
    #[serde(rename = "mp")]
    pub mark_price: String,
    #[serde(rename = "up")]
    pub unrealized_pnl: String,
    #[serde(rename = "mm")]
    pub maintenance_margin_required: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OrderTradeUpdateEvent {
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(rename = "o")]
    pub order: OrderTradeUpdate,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OrderTradeUpdate {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "c")]
    pub client_order_id: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "o")]
    pub order_type: String,
    #[serde(rename = "x")]
    pub execution_type: String,
    #[serde(rename = "X")]
    pub status: String,
    #[serde(rename = "i")]
    pub order_id: u64,
    #[serde(rename = "q")]
    pub original_quantity: String,
    #[serde(rename = "p")]
    pub original_price: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TradeLiteEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub transaction_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "q")]
    pub original_quantity: String,
    #[serde(rename = "p")]
    pub original_price: String,
    #[serde(rename = "m")]
    pub is_maker: bool,
    #[serde(rename = "c")]
    pub client_order_id: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "L")]
    pub last_filled_price: String,
    #[serde(rename = "l")]
    pub last_filled_quantity: String,
    #[serde(
        rename = "t",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub trade_id: u64,
    #[serde(
        rename = "i",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub order_id: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountUpdateEvent {
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(rename = "a")]
    pub data: AccountUpdateData,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountUpdateData {
    #[serde(rename = "m")]
    pub reason: String,
    #[serde(rename = "B", default)]
    pub balances: Vec<AccountUpdateBalance>,
    #[serde(rename = "P", default)]
    pub positions: Vec<AccountUpdatePosition>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountUpdateBalance {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "wb")]
    pub wallet_balance: String,
    #[serde(rename = "cw")]
    pub cross_wallet_balance: String,
    #[serde(rename = "bc")]
    pub balance_change: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountUpdatePosition {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "pa")]
    pub position_amount: String,
    #[serde(rename = "ep")]
    pub entry_price: String,
    #[serde(rename = "bep")]
    pub breakeven_price: String,
    #[serde(rename = "cr")]
    pub accumulated_realized: String,
    #[serde(rename = "up")]
    pub unrealized_pnl: String,
    #[serde(rename = "mt")]
    pub margin_type: String,
    #[serde(rename = "iw", default)]
    pub isolated_wallet: String,
    #[serde(rename = "ps")]
    pub position_side: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountConfigUpdateEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub transaction_time: u64,
    #[serde(rename = "ac", default)]
    pub symbol_config: Option<AccountConfigSymbolUpdate>,
    #[serde(rename = "ai", default)]
    pub user_config: Option<AccountConfigUserUpdate>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountConfigSymbolUpdate {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(
        rename = "l",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub leverage: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountConfigUserUpdate {
    #[serde(rename = "j")]
    pub multi_assets_margin_mode: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct StrategyUpdateEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub transaction_time: u64,
    #[serde(rename = "su")]
    pub update: StrategyUpdate,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct StrategyUpdate {
    #[serde(
        rename = "si",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub strategy_id: u64,
    #[serde(rename = "st")]
    pub strategy_type: String,
    #[serde(rename = "ss")]
    pub strategy_status: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(
        rename = "ut",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub update_time: u64,
    #[serde(
        rename = "c",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub opcode: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct GridUpdateEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub transaction_time: u64,
    #[serde(rename = "gu")]
    pub update: GridUpdate,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct GridUpdate {
    #[serde(
        rename = "si",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub strategy_id: u64,
    #[serde(rename = "st")]
    pub strategy_type: String,
    #[serde(rename = "ss")]
    pub strategy_status: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "r")]
    pub realized_pnl: String,
    #[serde(rename = "up")]
    pub unmatched_average_price: String,
    #[serde(rename = "uq")]
    pub unmatched_quantity: String,
    #[serde(rename = "uf")]
    pub unmatched_fee: String,
    #[serde(rename = "mp")]
    pub matched_pnl: String,
    #[serde(
        rename = "ut",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub update_time: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ConditionalOrderTriggerRejectEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub message_send_time: u64,
    #[serde(rename = "or")]
    pub order: ConditionalOrderTriggerRejectOrder,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ConditionalOrderTriggerRejectOrder {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(
        rename = "i",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub order_id: u64,
    #[serde(rename = "r")]
    pub reject_reason: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AlgoUpdateEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub transaction_time: u64,
    #[serde(rename = "o")]
    pub order: AlgoOrderUpdate,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AlgoOrderUpdate {
    #[serde(rename = "caid")]
    pub client_algo_id: String,
    #[serde(
        rename = "aid",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub algo_id: u64,
    #[serde(rename = "at")]
    pub algo_type: String,
    #[serde(rename = "o")]
    pub order_type: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "ps")]
    pub position_side: String,
    #[serde(rename = "f")]
    pub time_in_force: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "X")]
    pub algo_status: String,
    #[serde(rename = "ai")]
    pub order_id: String,
    #[serde(rename = "ap", default)]
    pub average_price: Option<String>,
    #[serde(rename = "aq", default)]
    pub executed_quantity: Option<String>,
    #[serde(rename = "act", default)]
    pub actual_order_type: Option<String>,
    #[serde(rename = "tp")]
    pub trigger_price: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "V")]
    pub self_trade_prevention_mode: String,
    #[serde(rename = "wt")]
    pub working_type: String,
    #[serde(rename = "pm")]
    pub price_match_mode: String,
    #[serde(rename = "cp")]
    pub close_position: bool,
    #[serde(rename = "pP")]
    pub price_protection: bool,
    #[serde(rename = "R")]
    pub reduce_only: bool,
    #[serde(
        rename = "tt",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub trigger_time: u64,
    #[serde(
        rename = "gtd",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub good_till_date: u64,
    #[serde(rename = "rm", default)]
    pub reject_reason: Option<String>,
}
