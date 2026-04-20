use crate::config::{Config, Credentials};
use crate::error::Error;
use crate::utils::{build_query_string, current_timestamp_millis, generate_signature};
use reqwest::{Client, Method, Proxy};
use serde::Deserialize;
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
            return serde_json::from_str(&body).map_err(Error::JsonError);
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
