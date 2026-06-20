use std::env;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, timeout};
use tokio_tungstenite::tungstenite::Message;

use crate::config::Credentials;
use crate::error::Error;
use crate::utils::current_timestamp_millis;

use super::connection::{connect_websocket, forward_event};
use super::{
    BitgetWebsocket, BitgetWebsocketChannel, BitgetWebsocketEvent, WEBSOCKET_CHANNEL_SIZE,
    login_request,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ReconnectConfig {
    pub reconnect_interval: Duration,
    pub max_reconnect_attempts: u32,
    pub ping_interval: Duration,
    pub message_timeout: Duration,
    pub backoff_factor: f64,
    pub max_backoff: Duration,
}

impl ReconnectConfig {
    pub fn new(reconnect_interval: Duration, max_reconnect_attempts: u32) -> Self {
        Self {
            reconnect_interval,
            max_reconnect_attempts,
            ping_interval: Duration::from_secs(30),
            message_timeout: Duration::from_secs(90),
            backoff_factor: 1.5,
            max_backoff: Duration::from_secs(60),
        }
    }

    pub fn with_ping_interval(mut self, value: Duration) -> Self {
        self.ping_interval = value;
        self
    }

    pub fn with_message_timeout(mut self, value: Duration) -> Self {
        self.message_timeout = value;
        self
    }

    pub fn with_backoff_factor(mut self, value: f64) -> Self {
        self.backoff_factor = value;
        self
    }

    pub fn with_max_backoff(mut self, value: Duration) -> Self {
        self.max_backoff = value;
        self
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

pub struct BitgetAutoReconnectWebsocketClient {
    urls: Vec<String>,
    config: ReconnectConfig,
    subscriptions: Vec<BitgetWebsocketChannel>,
    login_credentials: Option<Credentials>,
    proxy_url: Option<String>,
    command_tx: Option<mpsc::Sender<BitgetWebsocketCommand>>,
    state_tx: watch::Sender<ConnectionState>,
    metrics_tx: watch::Sender<WebsocketMetrics>,
    stop_tx: Option<watch::Sender<bool>>,
    task: Option<JoinHandle<()>>,
}

enum BitgetWebsocketCommand {
    Subscribe(BitgetWebsocketChannel),
    Unsubscribe(BitgetWebsocketChannel),
}

impl BitgetAutoReconnectWebsocketClient {
    pub fn new(url: impl Into<String>, config: ReconnectConfig) -> Self {
        let (state_tx, _) = watch::channel(ConnectionState::Disconnected);
        let (metrics_tx, _) = watch::channel(WebsocketMetrics::default());
        Self {
            urls: build_websocket_url_pool(url.into()),
            config,
            subscriptions: Vec::new(),
            login_credentials: None,
            proxy_url: None,
            command_tx: None,
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

    pub fn with_fallback_urls<I, S>(mut self, urls: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for url in urls {
            let url = url.into();
            push_websocket_url_candidate(&mut self.urls, &url);
        }
        self
    }

    pub fn urls(&self) -> &[String] {
        &self.urls
    }

    pub fn with_login_credentials(mut self, credentials: Credentials) -> Self {
        self.login_credentials = Some(credentials);
        self
    }

    pub fn add_subscription(&mut self, channel: BitgetWebsocketChannel) {
        if !self.subscriptions.contains(&channel) {
            self.subscriptions.push(channel);
        }
    }

    pub async fn subscribe(&mut self, channel: BitgetWebsocketChannel) -> Result<(), Error> {
        let inserted = if self.subscriptions.contains(&channel) {
            false
        } else {
            self.subscriptions.push(channel.clone());
            true
        };

        if inserted {
            self.send_command(BitgetWebsocketCommand::Subscribe(channel))
                .await?;
        }

        Ok(())
    }

    pub async fn unsubscribe(&mut self, channel: BitgetWebsocketChannel) -> Result<(), Error> {
        let previous_len = self.subscriptions.len();
        self.subscriptions.retain(|item| item != &channel);

        if self.subscriptions.len() != previous_len {
            self.send_command(BitgetWebsocketCommand::Unsubscribe(channel))
                .await?;
        }

        Ok(())
    }

    pub fn subscriptions(&self) -> &[BitgetWebsocketChannel] {
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

    pub async fn start(&mut self) -> Result<mpsc::Receiver<BitgetWebsocketEvent>, Error> {
        if self.task.is_some() {
            return Err(Error::WebSocketError(
                "WebSocket manager 已经启动".to_string(),
            ));
        }

        let (message_tx, message_rx) =
            mpsc::channel::<BitgetWebsocketEvent>(WEBSOCKET_CHANNEL_SIZE);
        let (command_tx, command_rx) =
            mpsc::channel::<BitgetWebsocketCommand>(WEBSOCKET_CHANNEL_SIZE);
        let (stop_tx, stop_rx) = watch::channel(false);
        self.command_tx = Some(command_tx);
        self.stop_tx = Some(stop_tx);

        let context = ReconnectLoopContext {
            urls: self.urls.clone(),
            config: self.config.clone(),
            subscriptions: self.subscriptions.clone(),
            login_credentials: self.login_credentials.clone(),
            proxy_url: self.proxy_url.clone(),
            command_rx,
            message_tx,
            stop_rx,
            state_tx: self.state_tx.clone(),
            metrics_tx: self.metrics_tx.clone(),
        };
        self.task = Some(tokio::spawn(async move {
            run_reconnect_loop(context).await;
        }));

        Ok(message_rx)
    }

    pub async fn stop(&mut self) {
        self.command_tx = None;
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(true);
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        self.state_tx.send_replace(ConnectionState::Stopped);
    }

    async fn send_command(&self, command: BitgetWebsocketCommand) -> Result<(), Error> {
        if let Some(command_tx) = &self.command_tx {
            command_tx.send(command).await.map_err(|err| {
                Error::WebSocketError(format!("发送 Bitget WebSocket manager 命令失败: {err}"))
            })?;
        }

        Ok(())
    }
}

pub struct BitgetWebsocketManager {
    client: BitgetAutoReconnectWebsocketClient,
}

impl BitgetWebsocketManager {
    pub fn new(url: impl Into<String>, config: ReconnectConfig) -> Self {
        Self {
            client: BitgetAutoReconnectWebsocketClient::new(url, config),
        }
    }

    pub fn with_proxy_url(mut self, proxy_url: impl Into<String>) -> Self {
        self.client = self.client.with_proxy_url(proxy_url);
        self
    }

    pub fn with_fallback_urls<I, S>(mut self, urls: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.client = self.client.with_fallback_urls(urls);
        self
    }

    pub fn urls(&self) -> &[String] {
        self.client.urls()
    }

    pub fn with_login_credentials(mut self, credentials: Credentials) -> Self {
        self.client = self.client.with_login_credentials(credentials);
        self
    }

    pub fn add_subscription(&mut self, channel: BitgetWebsocketChannel) {
        self.client.add_subscription(channel);
    }

    pub async fn subscribe(&mut self, channel: BitgetWebsocketChannel) -> Result<(), Error> {
        self.client.subscribe(channel).await
    }

    pub async fn unsubscribe(&mut self, channel: BitgetWebsocketChannel) -> Result<(), Error> {
        self.client.unsubscribe(channel).await
    }

    pub fn subscriptions(&self) -> &[BitgetWebsocketChannel] {
        self.client.subscriptions()
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.client.connection_state()
    }

    pub fn metrics(&self) -> WebsocketMetrics {
        self.client.metrics()
    }

    pub fn is_healthy(&self, max_message_age: Duration) -> bool {
        self.client.is_healthy(max_message_age)
    }

    pub async fn start(&mut self) -> Result<mpsc::Receiver<BitgetWebsocketEvent>, Error> {
        self.client.start().await
    }

    pub async fn stop(&mut self) {
        self.client.stop().await;
    }
}

struct ReconnectLoopContext {
    urls: Vec<String>,
    config: ReconnectConfig,
    subscriptions: Vec<BitgetWebsocketChannel>,
    login_credentials: Option<Credentials>,
    proxy_url: Option<String>,
    command_rx: mpsc::Receiver<BitgetWebsocketCommand>,
    message_tx: mpsc::Sender<BitgetWebsocketEvent>,
    stop_rx: watch::Receiver<bool>,
    state_tx: watch::Sender<ConnectionState>,
    metrics_tx: watch::Sender<WebsocketMetrics>,
}

async fn run_reconnect_loop(mut context: ReconnectLoopContext) {
    let mut attempts = 0;
    let mut current_url_idx = 0;
    let mut backoff_delay = context
        .config
        .reconnect_interval
        .min(context.config.max_backoff);

    while !*context.stop_rx.borrow() && attempts <= context.config.max_reconnect_attempts {
        context.state_tx.send_replace(if attempts == 0 {
            ConnectionState::Connecting
        } else {
            ConnectionState::Reconnecting
        });

        let url = context
            .urls
            .get(current_url_idx)
            .cloned()
            .unwrap_or_else(|| context.urls[0].clone());

        match connect_websocket(&url, context.proxy_url.as_deref()).await {
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
                let should_reconnect = run_connected_socket(
                    stream,
                    ConnectedSocketContext {
                        subscriptions: &mut context.subscriptions,
                        command_rx: &mut context.command_rx,
                        login_credentials: context.login_credentials.as_ref(),
                        message_tx: &context.message_tx,
                        stop_rx: &mut context.stop_rx,
                        metrics_tx: &context.metrics_tx,
                        ping_interval: context.config.ping_interval,
                        message_timeout: context.config.message_timeout,
                    },
                )
                .await;
                if !should_reconnect {
                    break;
                }
                attempts += 1;
                current_url_idx = next_websocket_url_index(&context.urls, current_url_idx);
            }
            Err(err) => {
                let message = err.to_string();
                update_metrics(&context.metrics_tx, |metrics| {
                    metrics.last_error = Some(message);
                    metrics.connection_attempts += 1;
                });
                attempts += 1;
                current_url_idx = next_websocket_url_index(&context.urls, current_url_idx);
            }
        }

        if *context.stop_rx.borrow() || attempts > context.config.max_reconnect_attempts {
            break;
        }

        tokio::select! {
            _ = sleep(backoff_delay) => {}
            _ = context.stop_rx.changed() => {}
        }
        backoff_delay = next_backoff_delay(
            backoff_delay,
            context.config.backoff_factor,
            context.config.max_backoff,
        );
    }

    context.state_tx.send_replace(if *context.stop_rx.borrow() {
        ConnectionState::Stopped
    } else {
        ConnectionState::Disconnected
    });
}

struct ConnectedSocketContext<'a> {
    subscriptions: &'a mut Vec<BitgetWebsocketChannel>,
    command_rx: &'a mut mpsc::Receiver<BitgetWebsocketCommand>,
    login_credentials: Option<&'a Credentials>,
    message_tx: &'a mpsc::Sender<BitgetWebsocketEvent>,
    stop_rx: &'a mut watch::Receiver<bool>,
    metrics_tx: &'a watch::Sender<WebsocketMetrics>,
    ping_interval: Duration,
    message_timeout: Duration,
}

async fn run_connected_socket<S>(
    stream: tokio_tungstenite::WebSocketStream<S>,
    context: ConnectedSocketContext<'_>,
) -> bool
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (mut write, mut read) = stream.split();
    if let Some(credentials) = context.login_credentials {
        let Ok(request) = login_request(credentials, current_timestamp_millis()) else {
            return true;
        };
        if write
            .send(Message::Text(request.to_string().into()))
            .await
            .is_err()
        {
            return true;
        }

        loop {
            let login_message = match timeout(context.ping_interval, read.next()).await {
                Ok(message) => message,
                Err(_) => {
                    update_metrics(context.metrics_tx, |metrics| {
                        metrics.last_error = Some(format!(
                            "Bitget WebSocket login ack 超时: {:?}",
                            context.ping_interval
                        ));
                    });
                    return true;
                }
            };

            match login_message {
                Some(Ok(Message::Text(text))) => {
                    match process_login_wait_text(
                        text.as_str(),
                        context.message_tx,
                        context.metrics_tx,
                    )
                    .await
                    {
                        LoginWaitOutcome::Authenticated => break,
                        LoginWaitOutcome::Continue => {}
                        LoginWaitOutcome::Reconnect => return true,
                        LoginWaitOutcome::Stop => return false,
                    }
                }
                Some(Ok(Message::Binary(bytes))) => {
                    let Ok(text) = std::str::from_utf8(&bytes) else {
                        continue;
                    };
                    match process_login_wait_text(text, context.message_tx, context.metrics_tx)
                        .await
                    {
                        LoginWaitOutcome::Authenticated => break,
                        LoginWaitOutcome::Continue => {}
                        LoginWaitOutcome::Reconnect => return true,
                        LoginWaitOutcome::Stop => return false,
                    }
                }
                Some(Ok(Message::Ping(payload))) => {
                    match write.send(Message::Pong(payload)).await {
                        Ok(()) => {}
                        Err(_) => return true,
                    }
                }
                Some(Ok(Message::Pong(_))) => {}
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return true,
                _ => {}
            }
        }
    }

    if !context.subscriptions.is_empty()
        && write
            .send(Message::Text(
                BitgetWebsocket::subscribe_request(context.subscriptions.as_slice()).into(),
            ))
            .await
            .is_err()
    {
        return true;
    }

    let mut ping_timer = interval(context.ping_interval);
    ping_timer.reset();
    let stale_after = context.message_timeout;
    let stale_check_interval = std::cmp::max(
        Duration::from_millis(1),
        std::cmp::min(context.ping_interval, context.message_timeout),
    );
    let mut stale_timer = interval(stale_check_interval);
    stale_timer.reset();
    let mut last_inbound_at = Instant::now();

    loop {
        tokio::select! {
            _ = context.stop_rx.changed() => {
                let _ = write.send(Message::Close(None)).await;
                return false;
            }
            command = context.command_rx.recv() => {
                let Some(command) = command else {
                    return false;
                };
                match command {
                    BitgetWebsocketCommand::Subscribe(channel) => {
                        if !context.subscriptions.contains(&channel) {
                            context.subscriptions.push(channel.clone());
                        }
                        if write
                            .send(Message::Text(
                                BitgetWebsocket::subscribe_request(std::slice::from_ref(&channel)).into(),
                            ))
                            .await
                            .is_err()
                        {
                            return true;
                        }
                    }
                    BitgetWebsocketCommand::Unsubscribe(channel) => {
                        context.subscriptions.retain(|item| item != &channel);
                        if write
                            .send(Message::Text(
                                BitgetWebsocket::unsubscribe_request(std::slice::from_ref(&channel)).into(),
                            ))
                            .await
                            .is_err()
                        {
                            return true;
                        }
                    }
                }
            }
            _ = ping_timer.tick() => {
                if write.send(Message::Text("ping".into())).await.is_err() {
                    return true;
                }
            }
            _ = stale_timer.tick() => {
                if last_inbound_at.elapsed() >= stale_after {
                    update_metrics(context.metrics_tx, |metrics| {
                        metrics.last_error = Some(format!(
                            "Bitget WebSocket 入站消息超时: {stale_after:?}"
                        ));
                    });
                    return true;
                }
            }
            message = read.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        last_inbound_at = Instant::now();
                        if forward_event(text.as_str(), context.message_tx).await.is_err() {
                            return false;
                        }
                        record_message(context.metrics_tx);
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        last_inbound_at = Instant::now();
                        let Ok(text) = std::str::from_utf8(&bytes) else {
                            continue;
                        };
                        if forward_event(text, context.message_tx).await.is_err() {
                            return false;
                        }
                        record_message(context.metrics_tx);
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        last_inbound_at = Instant::now();
                        let send_failed = write.send(Message::Pong(payload)).await.is_err();
                        if send_failed {
                            return true;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_inbound_at = Instant::now();
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return true,
                    _ => {}
                }
            }
        }
    }
}

enum LoginWaitOutcome {
    Continue,
    Authenticated,
    Reconnect,
    Stop,
}

async fn process_login_wait_text(
    text: &str,
    message_tx: &mpsc::Sender<BitgetWebsocketEvent>,
    metrics_tx: &watch::Sender<WebsocketMetrics>,
) -> LoginWaitOutcome {
    let event = match BitgetWebsocketEvent::parse(text) {
        Ok(event) => event,
        Err(err) => {
            update_metrics(metrics_tx, |metrics| {
                metrics.last_error = Some(format!("解析 Bitget WebSocket login ack 失败: {err}"));
            });
            return LoginWaitOutcome::Reconnect;
        }
    };
    let login_result = login_wait_result(&event);
    let login_failure = login_wait_failure_message(&event);
    if message_tx.send(event).await.is_err() {
        return LoginWaitOutcome::Stop;
    }
    record_message(metrics_tx);

    match login_result {
        Some(true) => LoginWaitOutcome::Authenticated,
        Some(false) => {
            update_metrics(metrics_tx, |metrics| {
                metrics.last_error = login_failure;
            });
            LoginWaitOutcome::Reconnect
        }
        None => LoginWaitOutcome::Continue,
    }
}

fn login_wait_result(event: &BitgetWebsocketEvent) -> Option<bool> {
    match event {
        BitgetWebsocketEvent::Login { code, .. } => {
            let success = matches!(code.as_deref(), Some("0") | Some("00000"));
            Some(success)
        }
        BitgetWebsocketEvent::Error { .. } => Some(false),
        _ => None,
    }
}

fn login_wait_failure_message(event: &BitgetWebsocketEvent) -> Option<String> {
    match event {
        BitgetWebsocketEvent::Login { code, msg, .. } => Some(format!(
            "Bitget WebSocket login failed: code={}, msg={}",
            code.as_deref().unwrap_or("<missing>"),
            msg.as_deref().unwrap_or("<missing>")
        )),
        BitgetWebsocketEvent::Error { code, msg, .. } => Some(format!(
            "Bitget WebSocket login error: code={}, msg={}",
            code.as_deref().unwrap_or("<missing>"),
            msg.as_deref().unwrap_or("<missing>")
        )),
        _ => None,
    }
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

fn build_websocket_url_pool(primary: String) -> Vec<String> {
    let mut urls = Vec::new();
    push_websocket_url_candidate(&mut urls, &primary);

    if let Ok(extra_urls) = env::var("BITGET_WS_FALLBACKS") {
        for item in extra_urls.split(',') {
            push_websocket_url_candidate(&mut urls, item);
        }
    }

    if urls.is_empty() {
        urls.push(primary);
    }

    urls
}

fn push_websocket_url_candidate(urls: &mut Vec<String>, candidate: &str) {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return;
    }

    let normalized = match url::Url::parse(trimmed) {
        Ok(url) if matches!(url.scheme(), "ws" | "wss") && url.host_str().is_some() => {
            url.to_string()
        }
        Ok(_) => return,
        Err(_) => trimmed.to_string(),
    };

    if !urls.contains(&normalized) {
        urls.push(normalized);
    }
}

fn next_websocket_url_index(urls: &[String], current_url_idx: usize) -> usize {
    if urls.len() <= 1 {
        current_url_idx
    } else {
        (current_url_idx + 1) % urls.len()
    }
}

fn next_backoff_delay(current: Duration, backoff_factor: f64, max_backoff: Duration) -> Duration {
    if current >= max_backoff {
        return max_backoff;
    }
    if !backoff_factor.is_finite() || backoff_factor <= 1.0 {
        return current.min(max_backoff);
    }

    let scaled = current.as_secs_f64() * backoff_factor;
    if !scaled.is_finite() {
        return max_backoff;
    }

    Duration::from_secs_f64(scaled.min(max_backoff.as_secs_f64()))
}
