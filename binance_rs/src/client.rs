use crate::config::{Config, Credentials};
use crate::error::Error;
use crate::utils::{build_query_string, current_timestamp_millis, generate_signature};
use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::{Client, Method, Proxy};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

type TimestampProvider = Arc<dyn Fn() -> u64 + Send + Sync>;

#[derive(Clone)]
pub struct BinanceClient {
    client: Client,
    credentials: Option<Credentials>,
    config: Config,
    timestamp_provider: TimestampProvider,
}

#[derive(Debug, Deserialize)]
struct BinanceApiErrorBody {
    code: i64,
    msg: String,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

/// Binance public REST 响应的 HTTP 与限频证据。
///
/// 这里只记录 provider 已返回的事实，不根据 header 推导剩余额度或重试策略；
/// 调度与退避属于上层 Market runtime。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinanceHttpEvidence {
    /// HTTP 响应状态码。
    pub http_status: u16,
    /// 所有 `x-mbx-used-weight*` 响应头，key 保留为小写 header 名。
    pub used_weight_headers: BTreeMap<String, String>,
    /// 所有 `x-mbx-order-count*` 响应头，key 保留为小写 header 名。
    pub order_count_headers: BTreeMap<String, String>,
    /// Provider 建议的等待时间；单位由 header 值本身决定，SDK 不进行换算。
    pub retry_after: Option<String>,
}

/// 带 Binance HTTP 证据的成功 public REST 响应。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinancePublicResponse<T> {
    /// 已完整反序列化的 provider wire body。
    pub data: T,
    /// 与该 body 同一次 HTTP 请求对应的状态码和限频 header。
    pub evidence: BinanceHttpEvidence,
}

/// Binance public REST 失败发生的协议阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinancePublicFailureKind {
    /// 请求尚未取得 HTTP 响应，例如 DNS、连接或超时错误。
    Transport,
    /// 已取得 HTTP 响应，但读取 response body 失败。
    ResponseBody,
    /// HTTP 成功，但 provider body 不符合目标 DTO 契约。
    Decode,
    /// HTTP 非成功，且 body 是 Binance `code/msg` 错误 envelope。
    Provider,
    /// HTTP 非成功，但 body 不是可识别的 Binance 错误 envelope。
    Http,
}

/// Binance public REST 失败的结构化 provider 证据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinancePublicRequestFailure {
    /// 失败所在阶段，用于上层区分 transport、协议损坏与 provider 拒绝。
    pub kind: BinancePublicFailureKind,
    /// 已收到响应时的 HTTP/限频证据；网络错误发生在响应前时为 `None`。
    pub evidence: Option<BinanceHttpEvidence>,
    /// Binance `code/msg` envelope 中的 provider code；非 provider envelope 时为 `None`。
    pub provider_code: Option<i64>,
    /// Provider message 或底层 transport/decode 错误说明。
    pub message: String,
    /// Binance 错误 envelope 的新增字段；非 provider envelope 时为空。
    pub provider_extra: BTreeMap<String, Value>,
}

impl fmt::Display for BinancePublicRequestFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = self.evidence.as_ref().map(|evidence| evidence.http_status);
        write!(
            formatter,
            "Binance public API failure ({:?}, HTTP: {status:?}, code: {:?}): {}",
            self.kind, self.provider_code, self.message
        )
    }
}

impl BinanceClient {
    pub fn new(credentials: Credentials) -> Result<Self, Error> {
        Self::with_config(Some(credentials), Config::from_env())
    }

    pub fn new_public() -> Result<Self, Error> {
        Self::with_config(None, Config::from_env())
    }

    pub fn from_env() -> Result<Self, Error> {
        Self::new(Credentials::from_env()?)
    }

    pub fn with_config(credentials: Option<Credentials>, config: Config) -> Result<Self, Error> {
        let mut builder = Client::builder().timeout(Duration::from_millis(config.api_timeout_ms));
        if let Some(proxy_url) = &config.proxy_url {
            builder = builder.proxy(Proxy::all(proxy_url).map_err(Error::HttpError)?);
        }

        let client = builder.build().map_err(Error::HttpError)?;

        Ok(Self {
            client,
            credentials,
            config,
            timestamp_provider: Arc::new(current_timestamp_millis),
        })
    }

    pub fn set_base_url(&mut self, base_url: impl Into<String>) {
        self.config.api_url = base_url.into();
    }

    pub fn set_timestamp_provider<F>(&mut self, provider: F)
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.timestamp_provider = Arc::new(provider);
    }

    pub async fn send_public_request<T>(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        let query = build_query_string(params);
        self.send_request(method, path, &query, false).await
    }

    /// 发送匿名 Binance public REST 请求并保留 HTTP/限频证据。
    ///
    /// 与既有 [`Self::send_public_request`] 分离，避免改变 legacy 调用方的错误
    /// contract。该方法只执行一次请求，不在 SDK 内重试或计算 quota。
    pub async fn send_public_request_with_evidence<T>(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<BinancePublicResponse<T>, Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        let query = build_query_string(params);
        let url = self.url(path, &query);
        let response = self
            .client
            .request(method, url)
            .send()
            .await
            .map_err(|source| {
                public_request_error(
                    BinancePublicFailureKind::Transport,
                    None,
                    None,
                    source.to_string(),
                    BTreeMap::new(),
                )
            })?;

        let evidence =
            BinanceHttpEvidence::from_headers(response.status().as_u16(), response.headers());
        let status_is_success = response.status().is_success();
        let body = response.text().await.map_err(|source| {
            public_request_error(
                BinancePublicFailureKind::ResponseBody,
                Some(evidence.clone()),
                None,
                source.to_string(),
                BTreeMap::new(),
            )
        })?;

        if status_is_success {
            let data = serde_json::from_str(&body).map_err(|source| {
                public_request_error(
                    BinancePublicFailureKind::Decode,
                    Some(evidence.clone()),
                    None,
                    source.to_string(),
                    BTreeMap::new(),
                )
            })?;
            return Ok(BinancePublicResponse { data, evidence });
        }

        if let Ok(error_body) = serde_json::from_str::<BinanceApiErrorBody>(&body) {
            return Err(public_request_error(
                BinancePublicFailureKind::Provider,
                Some(evidence),
                Some(error_body.code),
                error_body.msg,
                error_body.extra,
            ));
        }

        Err(public_request_error(
            BinancePublicFailureKind::Http,
            Some(evidence),
            None,
            body,
            BTreeMap::new(),
        ))
    }

    pub async fn send_signed_request<T>(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        let credentials = self.credentials.as_ref().ok_or(Error::MissingCredentials)?;

        let recv_window = self.config.recv_window_ms.to_string();
        let timestamp = (self.timestamp_provider)().to_string();
        let mut signed_params = params.to_vec();
        signed_params.push(("recvWindow", recv_window));
        signed_params.push(("timestamp", timestamp));

        let payload = build_query_string(&signed_params);
        let signature = generate_signature(&credentials.api_secret, &payload)?;
        let mut final_params = signed_params;
        final_params.push(("signature", signature));

        let query = build_query_string(&final_params);
        self.send_request(method, path, &query, true).await
    }

    pub async fn send_api_key_request<T>(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        let _ = self.credentials.as_ref().ok_or(Error::MissingCredentials)?;
        let query = build_query_string(params);
        self.send_request(method, path, &query, true).await
    }

    async fn send_request<T>(
        &self,
        method: Method,
        path: &str,
        query: &str,
        signed: bool,
    ) -> Result<T, Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = self.url(path, query);
        let mut request = self.client.request(method, url);

        if signed {
            let credentials = self.credentials.as_ref().ok_or(Error::MissingCredentials)?;
            request = request.header("X-MBX-APIKEY", &credentials.api_key);
        }

        let response = request.send().await.map_err(Error::HttpError)?;
        let status = response.status();
        let body = response.text().await.map_err(Error::HttpError)?;

        if status.is_success() {
            // Binance 的部分 DELETE 端点以空 body 表示成功；按 JSON null 解析可保留泛型返回合同。
            let body = if body.trim().is_empty() {
                "null"
            } else {
                &body
            };
            return serde_json::from_str(body).map_err(Error::JsonError);
        }

        if let Ok(error_body) = serde_json::from_str::<BinanceApiErrorBody>(&body) {
            return Err(Error::BinanceApiError {
                status: Some(status.as_u16()),
                code: error_body.code,
                message: error_body.msg,
            });
        }

        Err(Error::BinanceApiError {
            status: Some(status.as_u16()),
            code: i64::from(status.as_u16()),
            message: body,
        })
    }

    fn url(&self, path: &str, query: &str) -> String {
        let base_url = self.config.api_url.trim_end_matches('/');
        if query.is_empty() {
            format!("{base_url}{path}")
        } else {
            format!("{base_url}{path}?{query}")
        }
    }
}

impl BinanceHttpEvidence {
    fn from_headers(http_status: u16, headers: &HeaderMap) -> Self {
        Self {
            http_status,
            used_weight_headers: collect_prefixed_headers(headers, "x-mbx-used-weight"),
            order_count_headers: collect_prefixed_headers(headers, "x-mbx-order-count"),
            retry_after: headers
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        }
    }
}

fn collect_prefixed_headers(headers: &HeaderMap, prefix: &str) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            let name = name.as_str();
            if !name.starts_with(prefix) {
                return None;
            }
            value
                .to_str()
                .ok()
                .map(|value| (name.to_owned(), value.to_owned()))
        })
        .collect()
}

fn public_request_error(
    kind: BinancePublicFailureKind,
    evidence: Option<BinanceHttpEvidence>,
    provider_code: Option<i64>,
    message: String,
    provider_extra: BTreeMap<String, Value>,
) -> Error {
    Error::BinancePublicRequestFailed {
        failure: Box::new(BinancePublicRequestFailure {
            kind,
            evidence,
            provider_code,
            message,
            provider_extra,
        }),
    }
}
