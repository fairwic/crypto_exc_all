use crate::client::BinanceClient;
use crate::config::{Config, DEFAULT_API_TIMEOUT_MS, DEFAULT_API_URL};
use crate::error::Error;
use reqwest::{Proxy, Url};
use std::fmt;

/// Binance 匿名 REST capability 的显式传输配置。
///
/// 该配置不读取进程环境，也不包含账户凭证。Market owner 可以据此把确定的
/// endpoint、超时和出口代理绑定到自己的数据源配置。
#[derive(Clone, PartialEq, Eq)]
pub struct BinancePublicTransportConfig {
    /// Binance USDⓈ-M REST 基地址。
    pub api_url: String,
    /// 单次 HTTP 请求超时，单位为毫秒。
    pub request_timeout_ms: u64,
    /// 可选 HTTP(S) 或 SOCKS 代理地址。
    pub proxy_url: Option<String>,
}

impl Default for BinancePublicTransportConfig {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_API_URL.to_string(),
            request_timeout_ms: DEFAULT_API_TIMEOUT_MS,
            proxy_url: None,
        }
    }
}

impl fmt::Debug for BinancePublicTransportConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let api_url = if validate_endpoint(&self.api_url).is_ok() {
            self.api_url.as_str()
        } else {
            "<invalid-or-sensitive>"
        };
        formatter
            .debug_struct("BinancePublicTransportConfig")
            .field("api_url", &api_url)
            .field("request_timeout_ms", &self.request_timeout_ms)
            .field("proxy_configured", &self.proxy_url.is_some())
            .finish()
    }
}

impl BinanceClient {
    /// 使用调用方提供的确定性配置创建匿名公共 REST 客户端。
    ///
    /// 配置会在 HTTP client 构造前完成校验；代理地址不会出现在配置错误中。
    pub fn new_public_with_transport(
        transport: BinancePublicTransportConfig,
    ) -> Result<Self, Error> {
        transport.validate()?;

        let config = Config {
            api_url: transport.api_url,
            api_timeout_ms: transport.request_timeout_ms,
            proxy_url: transport.proxy_url,
            ..Config::default()
        };
        Self::with_config(None, config)
    }
}

impl BinancePublicTransportConfig {
    /// 在建立 HTTP client 前拒绝无法安全、确定表达的传输配置。
    fn validate(&self) -> Result<(), Error> {
        validate_endpoint(&self.api_url)?;
        if self.request_timeout_ms == 0 {
            return Err(Error::ConfigError(
                "Binance public request_timeout_ms 必须大于 0".to_string(),
            ));
        }
        if let Some(proxy_url) = &self.proxy_url {
            Proxy::all(proxy_url)
                .map_err(|_| Error::ConfigError("Binance public proxy_url 配置无效".to_string()))?;
        }
        Ok(())
    }
}

/// Endpoint 只接受不携带认证信息和附加查询语义的 HTTP(S) 基地址。
fn validate_endpoint(api_url: &str) -> Result<(), Error> {
    let endpoint = Url::parse(api_url)
        .map_err(|_| Error::ConfigError("Binance public api_url 配置无效".to_string()))?;
    let valid_scheme = matches!(endpoint.scheme(), "http" | "https");
    let has_userinfo = !endpoint.username().is_empty() || endpoint.password().is_some();
    if !valid_scheme
        || has_userinfo
        || endpoint.host_str().is_none()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(Error::ConfigError(
            "Binance public api_url 必须是无认证信息、query 和 fragment 的 HTTP(S) 地址"
                .to_string(),
        ));
    }
    Ok(())
}
