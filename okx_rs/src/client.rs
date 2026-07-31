#[cfg(feature = "full")]
use crate::config::{Credentials, CONFIG};

use crate::enums::language_enums::Language;
use crate::error::Error;
#[cfg(feature = "full")]
use crate::utils;
use log::{debug, error};
#[cfg(feature = "public-market")]
use reqwest::header::HeaderMap;
use reqwest::{Client, Method, RequestBuilder, StatusCode};
#[cfg(feature = "public-market")]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
#[cfg(feature = "public-market")]
use serde_json::Value;
use serde_json::{json, Deserializer};
use serde_path_to_error;
#[cfg(feature = "public-market")]
use std::collections::BTreeMap;
#[cfg(feature = "public-market")]
use std::fmt;

/// 通用的OKX API响应结构
#[derive(Serialize, Deserialize, Debug)]
pub struct OkxApiResponse<T: Serialize> {
    pub code: String,
    pub msg: String,
    pub data: T,
}

/// OKX 公共请求成功时随 typed data 返回的协议证据。
///
/// 该结构只保留非敏感的限频响应头；SDK 不根据这些值自动睡眠或重试。
#[cfg(feature = "public-market")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkxPublicResponseEvidence {
    /// HTTP 状态码；成功响应通常为 `200`。
    pub http_status: u16,
    /// OKX envelope 业务码；成功时固定为原始文本 `0`。
    pub okx_code: String,
    /// OKX envelope 消息原文；成功时通常为空字符串。
    pub okx_message: String,
    /// `Retry-After` 原始文本；不存在时为 `None`。
    pub retry_after: Option<String>,
    /// 安全白名单内的限频响应头，键统一为小写。
    pub rate_limit_headers: BTreeMap<String, String>,
}

/// OKX 公共请求的 typed data 与同一次响应证据。
#[cfg(feature = "public-market")]
#[derive(Debug, Clone, PartialEq)]
pub struct OkxPublicResponse<T> {
    /// 从 OKX envelope `data` 解码出的 provider DTO。
    pub data: T,
    /// 与 `data` 来自同一次 HTTP 响应的状态、业务码和限频证据。
    pub evidence: OkxPublicResponseEvidence,
}

/// OKX 公共响应失败的首个协议差异层。
#[cfg(feature = "public-market")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkxPublicFailureKind {
    /// HTTP 状态非成功；可能包含可供上层调度判断的 `Retry-After`。
    HttpStatus,
    /// HTTP 200 但响应体为空，无法形成 OKX envelope。
    EmptyBody,
    /// HTTP 200 但 JSON 或 envelope 的 `code`/`msg` 结构无效。
    MalformedEnvelope,
    /// HTTP 200 且 envelope 合法，但 OKX 业务码不是 `0`。
    ProviderRejected,
    /// HTTP 200、业务码为 `0`，但 `data` 不符合目标 provider DTO。
    MalformedData,
}

#[cfg(feature = "public-market")]
impl fmt::Display for OkxPublicFailureKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::HttpStatus => "http_status",
            Self::EmptyBody => "empty_body",
            Self::MalformedEnvelope => "malformed_envelope",
            Self::ProviderRejected => "provider_rejected",
            Self::MalformedData => "malformed_data",
        };
        formatter.write_str(value)
    }
}

/// OKX 公共请求失败时保留的非敏感协议证据。
#[cfg(feature = "public-market")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkxPublicFailureEvidence {
    /// 失败发生的首个协议层，供上层选择恢复策略。
    pub kind: OkxPublicFailureKind,
    /// OKX 已返回响应时的 HTTP 状态码。
    pub http_status: u16,
    /// 可解析时的 OKX envelope 业务码；响应非 JSON 时为 `None`。
    pub okx_code: Option<String>,
    /// 可解析时的 OKX envelope 消息；响应非 JSON 时为 `None`。
    pub okx_message: Option<String>,
    /// `Retry-After` 原始文本；SDK 仅透传，不自动重试。
    pub retry_after: Option<String>,
    /// 安全白名单内的限频响应头，键统一为小写。
    pub rate_limit_headers: BTreeMap<String, String>,
    /// 不包含凭证或完整响应体的解析诊断。
    pub detail: String,
}

#[cfg(feature = "public-market")]
impl fmt::Display for OkxPublicFailureEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "kind={} status={} code={:?} msg={:?}: {}",
            self.kind, self.http_status, self.okx_code, self.okx_message, self.detail
        )
    }
}

/// OKX HTTP API客户端
#[derive(Debug, Clone)]
pub struct OkxClient {
    /// HTTP客户端
    client: Client,
    /// API凭证
    #[cfg(feature = "full")]
    credentials: Option<Credentials>,
    /// 是否使用模拟交易
    #[cfg(feature = "full")]
    is_simulated_trading: String,
    /// API基础URL
    base_url: String,
    /// 请求有效期（毫秒）
    #[cfg(feature = "full")]
    request_expiration_ms: i64,
    /// 请求头中 Accept-Language
    accept_language: Option<Language>,
}

impl OkxClient {
    /// 创建一个新的OKX客户端
    #[cfg(feature = "full")]
    pub fn new(credentials: Credentials) -> Result<Self, Error> {
        let is_simulated_trading = credentials.is_simulated_trading.clone();
        Self::build(Some(credentials), is_simulated_trading)
    }

    /// 统一构造底层 HTTP client，避免公共与私有入口产生不同的超时行为。
    #[cfg(feature = "full")]
    fn build(
        credentials: Option<Credentials>,
        is_simulated_trading: String,
    ) -> Result<Self, Error> {
        let client = crate::public_transport::build_http_client(CONFIG.api_timeout_ms, None)?;

        Ok(Self {
            client,
            is_simulated_trading,
            credentials,
            base_url: CONFIG.api_url.clone(),
            request_expiration_ms: CONFIG.request_expiration_ms,
            accept_language: None,
        })
    }

    /// 只允许 public transport 模块组装无凭证客户端，避免复制字段默认值。
    pub(crate) fn from_public_transport(client: Client, base_url: String) -> Self {
        Self {
            client,
            #[cfg(feature = "full")]
            credentials: None,
            #[cfg(feature = "full")]
            is_simulated_trading: "0".to_string(),
            base_url,
            #[cfg(feature = "full")]
            request_expiration_ms: 1_000,
            accept_language: None,
        }
    }

    /// 从环境变量创建OKX客户端
    #[cfg(feature = "full")]
    pub fn from_env() -> Result<Self, Error> {
        let credentials = Credentials::from_env()?;
        Self::new(credentials)
    }

    /// 从环境变量创建OKX客户端，并设置模拟交易
    #[cfg(feature = "full")]
    pub fn from_env_with_simulated_trading() -> Result<Self, Error> {
        let credentials = Credentials::from_env_with_simulated_trading()?;
        let client = Self::new(credentials)?;
        Ok(client)
    }

    /// 设置是否使用模拟交易
    #[cfg(feature = "full")]
    pub fn set_simulated_trading(&mut self, is_simulated: String) {
        self.is_simulated_trading = is_simulated;
    }

    /// 设置API基础URL
    pub fn set_base_url(&mut self, base_url: impl Into<String>) {
        self.base_url = base_url.into();
    }

    /// 设置请求有效期
    #[cfg(feature = "full")]
    pub fn set_request_expiration(&mut self, expiration_ms: i64) {
        self.request_expiration_ms = expiration_ms;
    }

    /// 设置请求头中 Accept-Language
    pub fn set_accept_language(&mut self, accept_language: Language) {
        self.accept_language = Some(accept_language);
    }

    /// 发送API请求并返回反序列化的响应
    #[cfg(feature = "full")]
    pub async fn send_request<T: for<'a> Deserialize<'a> + Serialize>(
        &self,
        method: Method,
        path: &str,
        body: &str,
    ) -> Result<T, Error> {
        let credentials = self.credentials.as_ref().ok_or_else(|| {
            Error::AuthenticationError(
                "公共只读 OKX client 不能调用需要签名的 endpoint".to_string(),
            )
        })?;
        let method_str = method.to_string(); // 克隆方法字符串用于错误报告
        let timestamp = utils::generate_timestamp();
        let signature =
            utils::generate_signature(&credentials.api_secret, &timestamp, &method, path, body)?;
        let exp_time = utils::generate_expiration_timestamp(self.request_expiration_ms);

        let url = format!("{}{}", self.base_url, path);

        let mut request_builder = self
            .client
            .request(method, &url)
            .header("OK-ACCESS-KEY", &credentials.api_key)
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", &credentials.passphrase)
            .header("Content-Type", "application/json")
            .header("expTime", exp_time.to_string());
        if self.is_simulated_trading == "1" {
            request_builder = request_builder.header("x-simulated-trading", "1");
        }
        if let Some(accept_language) = &self.accept_language {
            request_builder =
                request_builder.header("Accept-Language", accept_language.to_string());
        }
        debug!("OKX API请求: {}", url);
        debug!("OKX API请求: {}", body);
        self.execute_request(request_builder.body(body.to_string()), &url, &method_str)
            .await
    }

    /// 发送不带账户签名、passphrase 或模拟交易头的公共只读请求。
    pub async fn send_public_request<T: for<'a> Deserialize<'a> + Serialize>(
        &self,
        method: Method,
        path: &str,
        body: &str,
    ) -> Result<T, Error> {
        let method_str = method.to_string();
        let url = format!("{}{}", self.base_url, path);
        let mut request_builder = self
            .client
            .request(method, &url)
            .header("Content-Type", "application/json");
        if let Some(accept_language) = &self.accept_language {
            request_builder =
                request_builder.header("Accept-Language", accept_language.to_string());
        }
        debug!("OKX public API请求: {}", url);
        self.execute_request(request_builder.body(body.to_string()), &url, &method_str)
            .await
    }

    /// 发送公共只读请求，并把 typed data 与同一次响应的协议证据一起返回。
    ///
    /// 该入口不读取凭证、不添加模拟交易头，也不在 429 或 5xx 后自动重试；调度与恢复
    /// 属于调用方 owner。
    #[cfg(feature = "public-market")]
    pub async fn send_public_request_with_evidence<T>(
        &self,
        method: Method,
        path: &str,
        body: &str,
    ) -> Result<OkxPublicResponse<T>, Error>
    where
        T: DeserializeOwned,
    {
        let method_str = method.to_string();
        let url = format!("{}{}", self.base_url, path);
        let mut request_builder = self
            .client
            .request(method, &url)
            .header("Content-Type", "application/json");
        if let Some(accept_language) = &self.accept_language {
            request_builder =
                request_builder.header("Accept-Language", accept_language.to_string());
        }
        debug!("OKX public API请求: {}", url);

        let response = request_builder
            .body(body.to_string())
            .send()
            .await
            .map_err(Error::HttpError)?;
        let status = response.status();
        let (retry_after, rate_limit_headers) = public_rate_limit_evidence(response.headers());
        let response_body = response.text().await.map_err(Error::HttpError)?;

        decode_public_response(
            status,
            retry_after,
            rate_limit_headers,
            &response_body,
            &url,
            &method_str,
        )
    }

    /// 执行已经按 capability 构造完成的请求，并统一解析 OKX envelope。
    async fn execute_request<T: for<'a> Deserialize<'a> + Serialize>(
        &self,
        request_builder: RequestBuilder,
        url: &str,
        method_str: &str,
    ) -> Result<T, Error> {
        let response = request_builder.send().await.map_err(Error::HttpError)?;
        let status_code = response.status();
        let response_body = response.text().await.map_err(Error::HttpError)?;
        debug!("okx result: {:?}", response_body);
        match status_code {
            StatusCode::OK => {
                // 使用 serde_path_to_error 来获取详细的字段路径信息
                let deserializer = &mut Deserializer::from_str(&response_body);
                let result: OkxApiResponse<T> = serde_path_to_error::deserialize(deserializer)
                    .map_err(|e| {
                        error!("JSON解析错误详情: {}", e);
                        error!("请求URL: {}, 请求方法: {}", url, method_str);
                        Error::JsonError(e.into_inner())
                    })?;
                if result.code == "0" {
                    return Ok(result.data);
                }
                // result={"code":"1","data":[{"clOrdId":"","ordId":"","sCode":"51000","sMsg":"Parameter ordId error","ts":"1752558485701"}],"inTime":"1752558485701589","msg":"All operations failed","outTime":"1752558485701884"}
                // 尝试从data数组的第一个元素中提取sMsg
                let smg = if let Ok(data_array) =
                    serde_json::from_str::<Vec<serde_json::Value>>(&json!(result.data).to_string())
                {
                    data_array
                        .first()
                        .and_then(|item| item.get("sMsg"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("未知错误")
                        .to_string()
                } else {
                    error!("解析错误信息失败: {}", response_body);
                    "解析错误信息失败".to_string()
                };

                error!("OKX API错误响应: {}", response_body);
                Err(Error::OkxApiError {
                    code: result.code,
                    message: result.msg,
                    smg,
                })
            }
            StatusCode::NOT_FOUND => {
                error!("OKX API错误响应: {}", response_body);
                Err(Error::OkxApiError {
                    code: "404".to_string(),
                    message: format!("API not found: {}", url),
                    smg: "".to_string(),
                })
            }
            _ => {
                error!("OKX API错误响应: {}", response_body);
                Err(Error::OkxApiError {
                    code: status_code.to_string(),
                    message: response_body,
                    smg: "".to_string(),
                })
            }
        }
    }
}

#[cfg(feature = "public-market")]
const SAFE_PUBLIC_RATE_LIMIT_HEADERS: &[&str] = &[
    "ratelimit-limit",
    "ratelimit-remaining",
    "ratelimit-reset",
    "x-rate-limit-limit",
    "x-rate-limit-remaining",
    "x-rate-limit-reset",
    "x-ratelimit-limit",
    "x-ratelimit-remaining",
    "x-ratelimit-reset",
];

/// 只采集明确的 quota 头，避免把 cookie、链路身份或代理内部头带入审计证据。
#[cfg(feature = "public-market")]
fn public_rate_limit_evidence(headers: &HeaderMap) -> (Option<String>, BTreeMap<String, String>) {
    let retry_after = headers
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let rate_limit_headers = SAFE_PUBLIC_RATE_LIMIT_HEADERS
        .iter()
        .filter_map(|name| {
            headers
                .get(*name)
                .and_then(|value| value.to_str().ok())
                .map(|value| ((*name).to_string(), value.to_string()))
        })
        .collect();
    (retry_after, rate_limit_headers)
}

/// 先固定 HTTP/envelope 证据，再解码目标 DTO，保证 malformed data 不会抹掉 provider 线索。
#[cfg(feature = "public-market")]
fn decode_public_response<T>(
    status: StatusCode,
    retry_after: Option<String>,
    rate_limit_headers: BTreeMap<String, String>,
    response_body: &str,
    url: &str,
    method: &str,
) -> Result<OkxPublicResponse<T>, Error>
where
    T: DeserializeOwned,
{
    if response_body.trim().is_empty() {
        let kind = if status.is_success() {
            OkxPublicFailureKind::EmptyBody
        } else {
            OkxPublicFailureKind::HttpStatus
        };
        return Err(public_failure(
            kind,
            status,
            None,
            None,
            retry_after,
            rate_limit_headers,
            format!("{method} {url} returned an empty response body"),
        ));
    }

    let envelope: Value = match serde_json::from_str(response_body) {
        Ok(value) => value,
        Err(parse_error) => {
            let kind = if status.is_success() {
                OkxPublicFailureKind::MalformedEnvelope
            } else {
                OkxPublicFailureKind::HttpStatus
            };
            return Err(public_failure(
                kind,
                status,
                None,
                None,
                retry_after,
                rate_limit_headers,
                format!("{method} {url} returned invalid JSON: {parse_error}"),
            ));
        }
    };
    let okx_code = envelope
        .get("code")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let okx_message = envelope
        .get("msg")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    if !status.is_success() {
        return Err(public_failure(
            OkxPublicFailureKind::HttpStatus,
            status,
            okx_code,
            okx_message,
            retry_after,
            rate_limit_headers,
            format!("{method} {url} returned a non-success HTTP status"),
        ));
    }

    let (okx_code, okx_message) = match (okx_code, okx_message) {
        (Some(code), Some(message)) => (code, message),
        (code, message) => {
            return Err(public_failure(
                OkxPublicFailureKind::MalformedEnvelope,
                status,
                code,
                message,
                retry_after,
                rate_limit_headers,
                format!("{method} {url} response must contain string code and msg"),
            ));
        }
    };
    if okx_code != "0" {
        return Err(public_failure(
            OkxPublicFailureKind::ProviderRejected,
            status,
            Some(okx_code),
            Some(okx_message),
            retry_after,
            rate_limit_headers,
            format!("{method} {url} was rejected by OKX"),
        ));
    }

    let data = envelope.get("data").cloned().ok_or_else(|| {
        public_failure(
            OkxPublicFailureKind::MalformedData,
            status,
            Some(okx_code.clone()),
            Some(okx_message.clone()),
            retry_after.clone(),
            rate_limit_headers.clone(),
            format!("{method} {url} response is missing data"),
        )
    })?;
    let data = serde_json::from_value(data).map_err(|parse_error| {
        public_failure(
            OkxPublicFailureKind::MalformedData,
            status,
            Some(okx_code.clone()),
            Some(okx_message.clone()),
            retry_after.clone(),
            rate_limit_headers.clone(),
            format!("{method} {url} data does not match the provider DTO: {parse_error}"),
        )
    })?;

    Ok(OkxPublicResponse {
        data,
        evidence: OkxPublicResponseEvidence {
            http_status: status.as_u16(),
            okx_code,
            okx_message,
            retry_after,
            rate_limit_headers,
        },
    })
}

/// 集中构造公共错误，防止不同失败分支遗漏 status、provider code 或 quota 证据。
#[cfg(feature = "public-market")]
fn public_failure(
    kind: OkxPublicFailureKind,
    status: StatusCode,
    okx_code: Option<String>,
    okx_message: Option<String>,
    retry_after: Option<String>,
    rate_limit_headers: BTreeMap<String, String>,
    detail: String,
) -> Error {
    Error::PublicApiError(Box::new(OkxPublicFailureEvidence {
        kind,
        http_status: status.as_u16(),
        okx_code,
        okx_message,
        retry_after,
        rate_limit_headers,
        detail,
    }))
}
