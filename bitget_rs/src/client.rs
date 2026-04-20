use crate::config::{Config, Credentials};
use crate::error::Error;
use crate::utils::{build_query_string, current_timestamp_millis, generate_signature};
use reqwest::{Client, Method, Proxy};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

type TimestampProvider = Arc<dyn Fn() -> u64 + Send + Sync>;

#[derive(Clone)]
pub struct BitgetClient {
    client: Client,
    credentials: Option<Credentials>,
    config: Config,
    timestamp_provider: TimestampProvider,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BitgetApiResponse<T> {
    code: String,
    msg: String,
    data: T,
}

impl BitgetClient {
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
        self.send_request(method, path, params, None, false).await
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
        self.send_request(method, path, params, None, true).await
    }

    pub async fn send_signed_json_request<T, B>(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, String)],
        body: &B,
    ) -> Result<T, Error>
    where
        T: for<'de> Deserialize<'de>,
        B: Serialize,
    {
        let body = serde_json::to_string(body).map_err(Error::JsonError)?;
        self.send_request(method, path, params, Some(body), true)
            .await
    }

    async fn send_request<T>(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, String)],
        body: Option<String>,
        signed: bool,
    ) -> Result<T, Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        let query = build_query_string(params);
        let url = self.url(path, &query);
        let method_string = method.as_str().to_ascii_uppercase();
        let body = body.unwrap_or_default();
        let mut request = self.client.request(method, url);

        if signed {
            let credentials = self.credentials.as_ref().ok_or(Error::MissingCredentials)?;
            let timestamp = (self.timestamp_provider)().to_string();
            let payload = signature_payload(&timestamp, &method_string, path, &query, &body);
            let signature = generate_signature(&credentials.api_secret, &payload)?;
            request = request
                .header("ACCESS-KEY", &credentials.api_key)
                .header("ACCESS-SIGN", signature)
                .header("ACCESS-TIMESTAMP", timestamp)
                .header("ACCESS-PASSPHRASE", &credentials.passphrase)
                .header("locale", "en-US")
                .header("Content-Type", "application/json");
        }

        if !body.is_empty() {
            request = request.body(body);
        }

        let response = request.send().await.map_err(Error::HttpError)?;
        let status = response.status();
        let body = response.text().await.map_err(Error::HttpError)?;

        if status.is_success() {
            let result: BitgetApiResponse<T> =
                serde_json::from_str(&body).map_err(Error::JsonError)?;
            if result.code == "00000" {
                return Ok(result.data);
            }

            return Err(Error::BitgetApiError {
                status: Some(status.as_u16()),
                code: result.code,
                message: result.msg,
            });
        }

        if let Ok(result) = serde_json::from_str::<BitgetApiResponse<serde_json::Value>>(&body) {
            return Err(Error::BitgetApiError {
                status: Some(status.as_u16()),
                code: result.code,
                message: result.msg,
            });
        }

        Err(Error::BitgetApiError {
            status: Some(status.as_u16()),
            code: status.as_u16().to_string(),
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

fn signature_payload(timestamp: &str, method: &str, path: &str, query: &str, body: &str) -> String {
    if query.is_empty() {
        format!("{timestamp}{method}{path}{body}")
    } else {
        format!("{timestamp}{method}{path}?{query}{body}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_bitget_signature_payload_with_sorted_query() {
        let query = build_query_string(&[
            ("symbol", "btcusdt".to_string()),
            ("marginCoin", "usdt".to_string()),
        ]);
        assert_eq!(query, "marginCoin=usdt&symbol=btcusdt");
        assert_eq!(
            signature_payload(
                "1684814440729",
                "GET",
                "/api/v2/mix/account/account",
                &query,
                ""
            ),
            "1684814440729GET/api/v2/mix/account/account?marginCoin=usdt&symbol=btcusdt"
        );
    }
}
