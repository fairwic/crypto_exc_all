use crate::config::{Config, Credentials};
use crate::error::Error;
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, Proxy};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha512};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type TimestampProvider = Arc<dyn Fn() -> u64 + Send + Sync>;

#[derive(Clone)]
pub struct GateClient {
    client: Client,
    credentials: Option<Credentials>,
    config: Config,
    timestamp_provider: TimestampProvider,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderRequest {
    pub contract: String,
    pub size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tif: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CancelOrderRequest {
    pub settle: String,
    pub order_id: String,
    pub contract: String,
}

impl GateClient {
    pub fn new(credentials: Credentials) -> Result<Self, Error> {
        Self::with_config(Some(credentials), Config::from_env())
    }

    pub fn new_public() -> Result<Self, Error> {
        Self::with_config(None, Config::from_env())
    }

    pub fn with_config(credentials: Option<Credentials>, config: Config) -> Result<Self, Error> {
        let mut builder = Client::builder().timeout(Duration::from_millis(config.api_timeout_ms));
        if let Some(proxy_url) = &config.proxy_url {
            builder = builder.proxy(Proxy::all(proxy_url).map_err(Error::HttpError)?);
        }
        Ok(Self {
            client: builder.build().map_err(Error::HttpError)?,
            credentials,
            config,
            timestamp_provider: Arc::new(current_timestamp_secs),
        })
    }

    pub fn set_timestamp_provider<F>(&mut self, provider: F)
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.timestamp_provider = Arc::new(provider);
    }

    pub async fn place_order(&self, settle: &str, request: &OrderRequest) -> Result<Value, Error> {
        self.send_signed_json(Method::POST, &format!("/futures/{settle}/orders"), request)
            .await
    }

    pub async fn order(
        &self,
        settle: &str,
        order_id: &str,
        contract: &str,
    ) -> Result<Value, Error> {
        self.send_signed_get(
            &format!("/futures/{settle}/orders/{order_id}"),
            &[("contract", contract.to_string())],
        )
        .await
    }

    pub async fn cancel_order(&self, request: &CancelOrderRequest) -> Result<Value, Error> {
        self.send_signed(
            Method::DELETE,
            &format!("/futures/{}/orders/{}", request.settle, request.order_id),
            &[("contract", request.contract.clone())],
            "",
        )
        .await
    }

    pub async fn position(&self, settle: &str, contract: &str) -> Result<Value, Error> {
        self.send_signed_get(&format!("/futures/{settle}/positions/{contract}"), &[])
            .await
    }

    pub(crate) async fn send_public(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<Value, Error> {
        let query = build_query_string(params);
        let response = self
            .client
            .get(self.url(path, &query))
            .send()
            .await
            .map_err(Error::HttpError)?;
        self.decode(response).await
    }

    pub(crate) async fn send_signed_get(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<Value, Error> {
        self.send_signed(Method::GET, path, params, "").await
    }

    async fn send_signed_json<B>(
        &self,
        method: Method,
        path: &str,
        body: &B,
    ) -> Result<Value, Error>
    where
        B: Serialize,
    {
        let body = serde_json::to_string(body).map_err(Error::JsonError)?;
        self.send_signed(method, path, &[], &body).await
    }

    async fn send_signed(
        &self,
        method: Method,
        path: &str,
        params: &[(&str, String)],
        body: &str,
    ) -> Result<Value, Error> {
        let credentials = self.credentials.as_ref().ok_or(Error::MissingCredentials)?;
        let query = build_query_string(params);
        let timestamp = (self.timestamp_provider)().to_string();
        let signature = sign(
            &credentials.api_secret,
            method.as_str(),
            path,
            &query,
            body,
            &timestamp,
        )?;
        let mut request = self
            .client
            .request(method, self.url(path, &query))
            .header("KEY", &credentials.api_key)
            .header("Timestamp", timestamp)
            .header("SIGN", signature)
            .header("Content-Type", "application/json");
        if !body.is_empty() {
            request = request.body(body.to_string());
        }
        let response = request.send().await.map_err(Error::HttpError)?;
        self.decode(response).await
    }

    async fn decode(&self, response: reqwest::Response) -> Result<Value, Error> {
        let status = response.status();
        let body = response.text().await.map_err(Error::HttpError)?;
        if status.is_success() {
            Ok(serde_json::from_str(&body).map_err(Error::JsonError)?)
        } else {
            let value = serde_json::from_str::<Value>(&body).unwrap_or_else(|_| json!({}));
            Err(Error::GateApiError {
                status: Some(status.as_u16()),
                code: value
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or_else(|| status.as_str())
                    .to_string(),
                message: value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or(&body)
                    .to_string(),
            })
        }
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

fn build_query_string(params: &[(&str, String)]) -> String {
    let mut params = params.to_vec();
    params.sort_by(|left, right| left.0.cmp(right.0));
    params
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

pub(crate) fn push_optional_str(
    params: &mut Vec<(&str, String)>,
    key: &'static str,
    value: Option<&str>,
) {
    if let Some(value) = value {
        params.push((key, value.to_string()));
    }
}

pub(crate) fn push_optional_u64(
    params: &mut Vec<(&str, String)>,
    key: &'static str,
    value: Option<u64>,
) {
    if let Some(value) = value {
        params.push((key, value.to_string()));
    }
}

pub(crate) fn push_optional_u32(
    params: &mut Vec<(&str, String)>,
    key: &'static str,
    value: Option<u32>,
) {
    if let Some(value) = value {
        params.push((key, value.to_string()));
    }
}

fn sign(
    secret: &str,
    method: &str,
    path: &str,
    query: &str,
    body: &str,
    timestamp: &str,
) -> Result<String, Error> {
    let body_hash = hex::encode(Sha512::digest(body.as_bytes()));
    let payload = format!(
        "{}\n{}\n{}\n{}\n{}",
        method.to_ascii_uppercase(),
        path,
        query,
        body_hash,
        timestamp
    );
    let mut mac =
        Hmac::<Sha512>::new_from_slice(secret.as_bytes()).map_err(|_| Error::SignatureError)?;
    mac.update(payload.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn current_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_apiv4_payload_with_request_hash() {
        let signature = sign(
            "secret",
            "GET",
            "/futures/usdt/orders/1",
            "contract=BTC_USDT",
            "",
            "1700000000",
        )
        .unwrap();

        assert_eq!(signature.len(), 128);
        assert_ne!(
            signature,
            sign(
                "secret",
                "GET",
                "/futures/usdt/orders/2",
                "",
                "",
                "1700000000"
            )
            .unwrap()
        );
    }

    #[tokio::test]
    async fn sends_public_ticker_to_futures_path() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/futures/usdt/tickers?contract=BTC_USDT")
            .with_status(200)
            .with_body(r#"[{"contract":"BTC_USDT","last":"68000"}]"#)
            .create_async()
            .await;

        let client = GateClient::with_config(
            None,
            Config {
                api_url: server.url(),
                api_timeout_ms: 1_000,
                proxy_url: None,
            },
        )
        .unwrap();
        let result = client.ticker("usdt", "BTC_USDT").await.unwrap();

        mock.assert_async().await;
        assert_eq!(result[0]["contract"], "BTC_USDT");
    }

    #[tokio::test]
    async fn sends_public_candlesticks_with_from_and_to_window() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/futures/usdt/candlesticks?contract=TEST_USDT&from=1700000000&interval=1m&to=1700007200",
            )
            .with_status(200)
            .with_body(r#"[]"#)
            .create_async()
            .await;

        let client = GateClient::with_config(
            None,
            Config {
                api_url: server.url(),
                api_timeout_ms: 1_000,
                proxy_url: None,
            },
        )
        .unwrap();

        let result = client
            .candlesticks(
                "usdt",
                "TEST_USDT",
                "1m",
                Some(200),
                Some(1_700_000_000),
                Some(1_700_007_200),
            )
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(result, serde_json::json!([]));
    }

    #[tokio::test]
    async fn signed_order_request_sends_gate_auth_headers() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/futures/usdt/orders")
            .match_header("KEY", "api-key")
            .match_header("Timestamp", "1700000000")
            .with_status(200)
            .with_body(r#"{"id":"1","status":"open"}"#)
            .create_async()
            .await;

        let mut client = GateClient::with_config(
            Some(Credentials::new("api-key", "secret")),
            Config {
                api_url: server.url(),
                api_timeout_ms: 1_000,
                proxy_url: None,
            },
        )
        .unwrap();
        client.set_timestamp_provider(|| 1_700_000_000);
        let result = client
            .place_order(
                "usdt",
                &OrderRequest {
                    contract: "BTC_USDT".to_string(),
                    size: 1,
                    price: None,
                    tif: Some("ioc".to_string()),
                    text: Some("t-test".to_string()),
                    reduce_only: None,
                },
            )
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(result["id"], "1");
    }
}
