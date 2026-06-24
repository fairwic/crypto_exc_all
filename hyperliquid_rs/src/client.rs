use crate::config::Config;
use crate::error::Error;
use reqwest::{Client, Proxy};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone)]
pub struct HyperliquidClient {
    client: Client,
    config: Config,
}

impl HyperliquidClient {
    pub fn new_public() -> Result<Self, Error> {
        Self::with_config(Config::from_env())
    }

    pub fn with_config(config: Config) -> Result<Self, Error> {
        let mut builder = Client::builder().timeout(Duration::from_millis(config.api_timeout_ms));
        if let Some(proxy_url) = &config.proxy_url {
            builder = builder.proxy(Proxy::all(proxy_url).map_err(Error::HttpError)?);
        }

        Ok(Self {
            client: builder.build().map_err(Error::HttpError)?,
            config,
        })
    }

    pub async fn send_info(&self, body: Value) -> Result<Value, Error> {
        let response = self
            .client
            .post(format!(
                "{}/info",
                self.config.api_url.trim_end_matches('/')
            ))
            .json(&body)
            .send()
            .await
            .map_err(Error::HttpError)?;
        self.decode(response).await
    }

    async fn decode(&self, response: reqwest::Response) -> Result<Value, Error> {
        let status = response.status();
        let body = response.text().await.map_err(Error::HttpError)?;
        if status.is_success() {
            let value: Value = serde_json::from_str(&body).map_err(Error::JsonError)?;
            if let Some(error) = value.get("error").and_then(Value::as_str) {
                return Err(Error::HyperliquidApiError {
                    status: Some(status.as_u16()),
                    code: "hyperliquid_error".to_string(),
                    message: error.to_string(),
                });
            }
            Ok(value)
        } else {
            let message = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|value| {
                    value
                        .get("error")
                        .or_else(|| value.get("message"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .unwrap_or_else(|| body.chars().take(240).collect());
            Err(Error::HyperliquidApiError {
                status: Some(status.as_u16()),
                code: status.as_u16().to_string(),
                message,
            })
        }
    }
}
