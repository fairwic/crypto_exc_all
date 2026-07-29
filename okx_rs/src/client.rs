#[cfg(feature = "full")]
use crate::config::{Credentials, CONFIG};

use crate::enums::language_enums::Language;
use crate::error::Error;
#[cfg(feature = "full")]
use crate::utils;
use log::{debug, error};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Deserializer};
use serde_path_to_error;
use std::time::Duration;

const DEFAULT_PUBLIC_API_URL: &str = "https://www.okx.com";
const DEFAULT_PUBLIC_API_TIMEOUT_MS: u64 = 5000;

/// 通用的OKX API响应结构
#[derive(Serialize, Deserialize, Debug)]
pub struct OkxApiResponse<T: Serialize> {
    pub code: String,
    pub msg: String,
    pub data: T,
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

    /// 创建不持有账户凭证的公共只读客户端。
    ///
    /// 该客户端只能通过 `send_public_request` 调用公开 endpoint；受限 feature
    /// 不编译 signed API，full feature 下误用 signed request 会在 I/O 前失败。
    pub fn new_public() -> Result<Self, Error> {
        let client = Self::build_http_client(DEFAULT_PUBLIC_API_TIMEOUT_MS)?;
        Ok(Self {
            client,
            #[cfg(feature = "full")]
            credentials: None,
            #[cfg(feature = "full")]
            is_simulated_trading: "0".to_string(),
            base_url: DEFAULT_PUBLIC_API_URL.to_string(),
            #[cfg(feature = "full")]
            request_expiration_ms: 1000,
            accept_language: None,
        })
    }

    /// 统一构造底层 HTTP client，避免公共与私有入口产生不同的超时行为。
    #[cfg(feature = "full")]
    fn build(
        credentials: Option<Credentials>,
        is_simulated_trading: String,
    ) -> Result<Self, Error> {
        let client = Self::build_http_client(CONFIG.api_timeout_ms)?;

        Ok(Self {
            client,
            is_simulated_trading,
            credentials,
            base_url: CONFIG.api_url.clone(),
            request_expiration_ms: CONFIG.request_expiration_ms,
            accept_language: None,
        })
    }

    /// 创建使用统一超时策略的 reqwest client。
    fn build_http_client(timeout_ms: u64) -> Result<Client, Error> {
        Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(Error::HttpError)
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
