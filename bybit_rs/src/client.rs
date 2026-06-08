use crate::config::{Config, Credentials};
use crate::error::Error;
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, Proxy};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type TimestampProvider = Arc<dyn Fn() -> u64 + Send + Sync>;

#[derive(Clone)]
pub struct BybitClient {
    client: Client,
    credentials: Option<Credentials>,
    config: Config,
    timestamp_provider: TimestampProvider,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BybitResponse {
    ret_code: i64,
    ret_msg: String,
    result: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderRequest {
    pub category: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub qty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_link_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderRequest {
    pub category: String,
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_link_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderStatusRequest {
    pub category: String,
    pub symbol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_link_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionListRequest {
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

impl BybitClient {
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
            timestamp_provider: Arc::new(current_timestamp_millis),
        })
    }

    pub fn set_timestamp_provider<F>(&mut self, provider: F)
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.timestamp_provider = Arc::new(provider);
    }

    pub async fn ticker(&self, category: &str, symbol: &str) -> Result<Value, Error> {
        self.send_public(
            "/v5/market/tickers",
            &[
                ("category", category.to_string()),
                ("symbol", symbol.to_string()),
            ],
        )
        .await
    }

    pub async fn orderbook(
        &self,
        category: &str,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Value, Error> {
        let mut params = vec![
            ("category", category.to_string()),
            ("symbol", symbol.to_string()),
        ];
        if let Some(limit) = limit {
            params.push(("limit", limit.to_string()));
        }
        self.send_public("/v5/market/orderbook", &params).await
    }

    pub async fn kline(
        &self,
        category: &str,
        symbol: &str,
        interval: &str,
        limit: Option<u32>,
        start: Option<u64>,
        end: Option<u64>,
    ) -> Result<Value, Error> {
        let mut params = vec![
            ("category", category.to_string()),
            ("symbol", symbol.to_string()),
            ("interval", interval.to_string()),
        ];
        if let Some(limit) = limit {
            params.push(("limit", limit.to_string()));
        }
        if let Some(start) = start {
            params.push(("start", start.to_string()));
        }
        if let Some(end) = end {
            params.push(("end", end.to_string()));
        }
        self.send_public("/v5/market/kline", &params).await
    }

    pub async fn instruments(&self, category: &str, symbol: Option<&str>) -> Result<Value, Error> {
        let mut params = vec![("category", category.to_string())];
        if let Some(symbol) = symbol {
            params.push(("symbol", symbol.to_string()));
        }
        self.send_public("/v5/market/instruments-info", &params)
            .await
    }

    pub async fn place_order(&self, request: &OrderRequest) -> Result<Value, Error> {
        self.send_signed_json(Method::POST, "/v5/order/create", request)
            .await
    }

    pub async fn cancel_order(&self, request: &CancelOrderRequest) -> Result<Value, Error> {
        self.send_signed_json(Method::POST, "/v5/order/cancel", request)
            .await
    }

    pub async fn order_status(&self, request: &OrderStatusRequest) -> Result<Value, Error> {
        let params = to_params(request)?;
        self.send_signed_get("/v5/order/realtime", &params).await
    }

    pub async fn positions(&self, request: &PositionListRequest) -> Result<Value, Error> {
        let params = to_params(request)?;
        self.send_signed_get("/v5/position/list", &params).await
    }

    async fn send_public(&self, path: &str, params: &[(&str, String)]) -> Result<Value, Error> {
        let query = build_query_string(params);
        let response = self
            .client
            .get(self.url(path, &query))
            .send()
            .await
            .map_err(Error::HttpError)?;
        self.decode(response).await
    }

    async fn send_signed_get(&self, path: &str, params: &[(&str, String)]) -> Result<Value, Error> {
        let query = build_query_string(params);
        let request = self.signed_request(Method::GET, path, &query, "")?;
        let response = request.send().await.map_err(Error::HttpError)?;
        self.decode(response).await
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
        let request = self.signed_request(method, path, "", &body)?.body(body);
        let response = request.send().await.map_err(Error::HttpError)?;
        self.decode(response).await
    }

    fn signed_request(
        &self,
        method: Method,
        path: &str,
        query: &str,
        body: &str,
    ) -> Result<reqwest::RequestBuilder, Error> {
        let credentials = self.credentials.as_ref().ok_or(Error::MissingCredentials)?;
        let timestamp = (self.timestamp_provider)().to_string();
        let recv_window = self.config.recv_window_ms.to_string();
        let payload = if method == Method::GET { query } else { body };
        let signature = sign(
            &credentials.api_secret,
            &timestamp,
            &credentials.api_key,
            &recv_window,
            payload,
        )?;
        Ok(self
            .client
            .request(method, self.url(path, query))
            .header("X-BAPI-API-KEY", &credentials.api_key)
            .header("X-BAPI-SIGN", signature)
            .header("X-BAPI-TIMESTAMP", timestamp)
            .header("X-BAPI-RECV-WINDOW", recv_window)
            .header("Content-Type", "application/json"))
    }

    async fn decode(&self, response: reqwest::Response) -> Result<Value, Error> {
        let status = response.status();
        let body = response.text().await.map_err(Error::HttpError)?;
        let value: BybitResponse = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(_) => {
                return Err(Error::BybitApiError {
                    status: Some(status.as_u16()),
                    code: status.as_u16().to_string(),
                    message: body.chars().take(240).collect(),
                });
            }
        };
        if status.is_success() && value.ret_code == 0 {
            Ok(value.result)
        } else {
            Err(Error::BybitApiError {
                status: Some(status.as_u16()),
                code: value.ret_code.to_string(),
                message: value.ret_msg,
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

fn to_params<T: Serialize>(value: &T) -> Result<Vec<(&'static str, String)>, Error> {
    let value = serde_json::to_value(value).map_err(Error::JsonError)?;
    let object = value.as_object().ok_or(Error::JsonError(
        serde_json::from_str::<Value>("").unwrap_err(),
    ))?;
    let mut params = Vec::new();
    for (key, value) in object {
        if value.is_null() {
            continue;
        }
        let value = value
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| value.to_string());
        params.push((leak_key(key), value));
    }
    Ok(params)
}

fn leak_key(key: &str) -> &'static str {
    Box::leak(key.to_string().into_boxed_str())
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

fn sign(
    secret: &str,
    timestamp: &str,
    api_key: &str,
    recv_window: &str,
    payload: &str,
) -> Result<String, Error> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| Error::SignatureError)?;
    mac.update(format!("{timestamp}{api_key}{recv_window}{payload}").as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_v5_payload_with_timestamp_key_recv_window_and_body() {
        let signature = sign(
            "secret",
            "1700000000000",
            "api-key",
            "5000",
            r#"{"category":"linear"}"#,
        )
        .unwrap();

        assert_eq!(signature.len(), 64);
        assert_ne!(
            signature,
            sign("secret", "1700000000001", "api-key", "5000", "{}").unwrap()
        );
    }

    #[tokio::test]
    async fn sends_public_ticker_to_v5_market_path() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/v5/market/tickers?category=linear&symbol=BTCUSDT")
            .with_status(200)
            .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"list":[{"symbol":"BTCUSDT"}]}}"#)
            .create_async()
            .await;

        let client = BybitClient::with_config(
            None,
            Config {
                api_url: server.url(),
                api_timeout_ms: 1_000,
                recv_window_ms: 5_000,
                proxy_url: None,
            },
        )
        .unwrap();
        let result = client.ticker("linear", "BTCUSDT").await.unwrap();

        mock.assert_async().await;
        assert_eq!(result["list"][0]["symbol"], "BTCUSDT");
    }

    #[tokio::test]
    async fn sends_public_kline_with_start_and_end_window() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/v5/market/kline?category=linear&end=1700007200000&interval=1&limit=200&start=1700000000000&symbol=TESTUSDT",
            )
            .with_status(200)
            .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"list":[]}}"#)
            .create_async()
            .await;

        let client = BybitClient::with_config(
            None,
            Config {
                api_url: server.url(),
                api_timeout_ms: 1_000,
                recv_window_ms: 5_000,
                proxy_url: None,
            },
        )
        .unwrap();

        let result = client
            .kline(
                "linear",
                "TESTUSDT",
                "1",
                Some(200),
                Some(1_700_000_000_000),
                Some(1_700_007_200_000),
            )
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(result["list"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn maps_non_json_public_error_body_to_api_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock(
                "GET",
                "/v5/market/kline?category=linear&interval=1&symbol=TESTUSDT",
            )
            .with_status(403)
            .with_body("{\n    error:blocked by edge\n}")
            .create_async()
            .await;

        let client = BybitClient::with_config(
            None,
            Config {
                api_url: server.url(),
                api_timeout_ms: 1_000,
                recv_window_ms: 5_000,
                proxy_url: None,
            },
        )
        .unwrap();

        let error = client
            .kline("linear", "TESTUSDT", "1", None, None, None)
            .await
            .expect_err("non-json body should map to api error");

        mock.assert_async().await;
        assert!(error.to_string().contains("status=Some(403)"));
        assert!(error.to_string().contains("blocked by edge"));
    }

    #[tokio::test]
    async fn signed_order_request_sends_bybit_auth_headers() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v5/order/create")
            .match_header("X-BAPI-API-KEY", "api-key")
            .match_header("X-BAPI-TIMESTAMP", "1700000000000")
            .match_header("X-BAPI-RECV-WINDOW", "5000")
            .with_status(200)
            .with_body(r#"{"retCode":0,"retMsg":"OK","result":{"orderId":"1"}}"#)
            .create_async()
            .await;

        let mut client = BybitClient::with_config(
            Some(Credentials::new("api-key", "secret")),
            Config {
                api_url: server.url(),
                api_timeout_ms: 1_000,
                recv_window_ms: 5_000,
                proxy_url: None,
            },
        )
        .unwrap();
        client.set_timestamp_provider(|| 1_700_000_000_000);
        let result = client
            .place_order(&OrderRequest {
                category: "linear".to_string(),
                symbol: "BTCUSDT".to_string(),
                side: "Buy".to_string(),
                order_type: "Market".to_string(),
                qty: "0.01".to_string(),
                price: None,
                time_in_force: None,
                order_link_id: Some("cid-1".to_string()),
                reduce_only: None,
            })
            .await
            .unwrap();

        mock.assert_async().await;
        assert_eq!(result["orderId"], "1");
    }
}
