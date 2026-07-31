use crate::client::OkxClient;
use crate::error::Error;
use reqwest::{Client, Proxy, Url};
use std::fmt;
use std::time::Duration;

const DEFAULT_PUBLIC_API_URL: &str = "https://www.okx.com";
const DEFAULT_PUBLIC_API_TIMEOUT_MS: u64 = 5_000;

/// OKX 匿名 REST capability 的显式传输配置。
///
/// 该配置不读取进程环境，也不包含 API Key、Secret 或 passphrase。
#[derive(Clone, PartialEq, Eq)]
pub struct OkxPublicTransportConfig {
    /// OKX REST 基地址。
    pub api_url: String,
    /// 单次 HTTP 请求超时，单位为毫秒。
    pub request_timeout_ms: u64,
    /// 可选 HTTP(S) 或 SOCKS 代理地址。
    pub proxy_url: Option<String>,
}

impl Default for OkxPublicTransportConfig {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_PUBLIC_API_URL.to_string(),
            request_timeout_ms: DEFAULT_PUBLIC_API_TIMEOUT_MS,
            proxy_url: None,
        }
    }
}

impl fmt::Debug for OkxPublicTransportConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let api_url = if validate_endpoint(&self.api_url).is_ok() {
            self.api_url.as_str()
        } else {
            "<invalid-or-sensitive>"
        };
        formatter
            .debug_struct("OkxPublicTransportConfig")
            .field("api_url", &api_url)
            .field("request_timeout_ms", &self.request_timeout_ms)
            .field("proxy_configured", &self.proxy_url.is_some())
            .finish()
    }
}

impl OkxClient {
    /// 使用内置确定性默认值创建匿名公共 REST 客户端。
    pub fn new_public() -> Result<Self, Error> {
        Self::new_public_with_transport(OkxPublicTransportConfig::default())
    }

    /// 使用调用方提供的确定性配置创建匿名公共 REST 客户端。
    ///
    /// 配置会在 HTTP client 构造前完成校验；代理地址不会出现在配置错误中。
    pub fn new_public_with_transport(transport: OkxPublicTransportConfig) -> Result<Self, Error> {
        transport.validate()?;
        let client =
            build_http_client(transport.request_timeout_ms, transport.proxy_url.as_deref())?;
        Ok(Self::from_public_transport(client, transport.api_url))
    }
}

impl OkxPublicTransportConfig {
    /// 在建立 HTTP client 前拒绝无法安全、确定表达的传输配置。
    fn validate(&self) -> Result<(), Error> {
        validate_endpoint(&self.api_url)?;
        if self.request_timeout_ms == 0 {
            return Err(Error::ConfigError(
                "OKX public request_timeout_ms 必须大于 0".to_string(),
            ));
        }
        if let Some(proxy_url) = &self.proxy_url {
            Proxy::all(proxy_url)
                .map_err(|_| Error::ConfigError("OKX public proxy_url 配置无效".to_string()))?;
        }
        Ok(())
    }
}

/// 构造共享 reqwest client；完整账户客户端继续复用相同的超时实现。
pub(crate) fn build_http_client(timeout_ms: u64, proxy_url: Option<&str>) -> Result<Client, Error> {
    let mut builder = Client::builder().timeout(Duration::from_millis(timeout_ms));
    if let Some(proxy_url) = proxy_url {
        let proxy = Proxy::all(proxy_url)
            .map_err(|_| Error::ConfigError("OKX public proxy_url 配置无效".to_string()))?;
        builder = builder.proxy(proxy);
    }
    builder.build().map_err(Error::HttpError)
}

/// Endpoint 只接受不携带认证信息和附加查询语义的 HTTP(S) 基地址。
fn validate_endpoint(api_url: &str) -> Result<(), Error> {
    let endpoint = Url::parse(api_url)
        .map_err(|_| Error::ConfigError("OKX public api_url 配置无效".to_string()))?;
    let valid_scheme = matches!(endpoint.scheme(), "http" | "https");
    let has_userinfo = !endpoint.username().is_empty() || endpoint.password().is_some();
    if !valid_scheme
        || has_userinfo
        || endpoint.host_str().is_none()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(Error::ConfigError(
            "OKX public api_url 必须是无认证信息、query 和 fragment 的 HTTP(S) 地址".to_string(),
        ));
    }
    Ok(())
}
