use crate::error::Error;
use std::env;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{client_async_tls_with_config, MaybeTlsStream, WebSocketStream};
use url::Url;
pub(super) trait SocketIo: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T> SocketIo for T where T: AsyncRead + AsyncWrite + Send + Unpin {}
pub(super) type OkxSocket = WebSocketStream<MaybeTlsStream<Box<dyn SocketIo>>>;

pub(super) async fn connect_socket(
    url: &str,
    config: WebSocketConfig,
) -> Result<
    (
        OkxSocket,
        tokio_tungstenite::tungstenite::handshake::client::Response,
    ),
    Error,
> {
    let target = Url::parse(url)
        .map_err(|_| Error::ConfigError("OKX private WebSocket URL 无效".to_string()))?;
    let host = target
        .host_str()
        .ok_or_else(|| Error::ConfigError("OKX private WebSocket 缺少 host".to_string()))?;
    let port = target
        .port_or_known_default()
        .ok_or_else(|| Error::ConfigError("OKX private WebSocket 缺少 port".to_string()))?;
    let transport: Box<dyn SocketIo> = match proxy_endpoint(host)? {
        Some((proxy_host, proxy_port)) => Box::new(
            Socks5Stream::connect((proxy_host.as_str(), proxy_port), (host, port))
                .await
                .map_err(|error| {
                    Error::WebSocketError(format!("OKX private SOCKS5 连接失败: {error}"))
                })?,
        ),
        None => Box::new(TcpStream::connect((host, port)).await.map_err(|error| {
            Error::WebSocketError(format!("OKX private TCP 连接失败: {error}"))
        })?),
    };
    client_async_tls_with_config(url, transport, Some(config), None)
        .await
        .map_err(|error| Error::WebSocketError(error.to_string()))
}

fn proxy_endpoint(target_host: &str) -> Result<Option<(String, u16)>, Error> {
    if matches!(target_host, "localhost" | "127.0.0.1" | "::1") {
        return Ok(None);
    }
    let Some(raw_proxy) = ["ALL_PROXY", "all_proxy"]
        .into_iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
    else {
        return Ok(None);
    };
    let proxy = Url::parse(raw_proxy.trim())
        .map_err(|_| Error::ConfigError("ALL_PROXY 不是有效 URL".to_string()))?;
    if !matches!(proxy.scheme(), "socks5" | "socks5h")
        || !proxy.username().is_empty()
        || proxy.password().is_some()
    {
        return Err(Error::ConfigError(
            "OKX private WebSocket 只支持无认证 socks5/socks5h ALL_PROXY".to_string(),
        ));
    }
    let host = proxy
        .host_str()
        .ok_or_else(|| Error::ConfigError("ALL_PROXY 缺少 host".to_string()))?;
    let port = proxy
        .port()
        .ok_or_else(|| Error::ConfigError("ALL_PROXY 缺少 port".to_string()))?;
    Ok(Some((host.to_string(), port)))
}
