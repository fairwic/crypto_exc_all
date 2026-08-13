use std::collections::VecDeque;

use serde_json::Value;
use tokio::sync::mpsc;

use super::auto_reconnect_client::{AutoReconnectWebsocketClient, WebsocketHealthSnapshot};
use super::{Args, ChannelType};
use crate::dto::market::CandleOkxRespDto;
use crate::error::Error;

/// OKX business WebSocket 推送的一根 typed K 线。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkxPublicCandleUpdate {
    pub instrument_id: String,
    pub interval: String,
    pub timestamp: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub volume_currency: String,
    pub quote_volume: String,
    pub confirmed: bool,
}

/// 只订阅一个产品和周期的公共 K 线连接器。
#[derive(Clone)]
pub struct OkxPublicCandleStreamClient {
    client: AutoReconnectWebsocketClient,
    instrument_id: String,
    interval: String,
}

impl OkxPublicCandleStreamClient {
    pub fn new(instrument_id: impl Into<String>, interval: impl Into<String>) -> Self {
        Self {
            client: AutoReconnectWebsocketClient::new_public_business(),
            instrument_id: instrument_id.into(),
            interval: interval.into(),
        }
    }

    /// 覆盖业务 WebSocket 地址，供本地协议测试使用。
    pub fn with_url(mut self, url: &str) -> Self {
        self.client = AutoReconnectWebsocketClient::new_with_config(url, None, Default::default());
        self
    }

    pub async fn connect(&self) -> Result<OkxPublicCandleStreamSession, Error> {
        validate_subscription(&self.instrument_id, &self.interval)?;
        self.client
            .subscribe(
                ChannelType::Candle(self.interval.clone()),
                Args::new().with_inst_id(self.instrument_id.clone()),
            )
            .await?;
        let receiver = self.client.start().await?;
        Ok(OkxPublicCandleStreamSession {
            client: self.client.clone(),
            receiver,
            instrument_id: self.instrument_id.clone(),
            interval: self.interval.clone(),
            pending: VecDeque::new(),
        })
    }
}

/// 已登记单一 candle 订阅的自动重连会话。
pub struct OkxPublicCandleStreamSession {
    client: AutoReconnectWebsocketClient,
    receiver: mpsc::UnboundedReceiver<Value>,
    instrument_id: String,
    interval: String,
    pending: VecDeque<OkxPublicCandleUpdate>,
}

impl OkxPublicCandleStreamSession {
    pub async fn recv(&mut self) -> Result<Option<OkxPublicCandleUpdate>, Error> {
        loop {
            if let Some(update) = self.pending.pop_front() {
                return Ok(Some(update));
            }
            let Some(message) = self.receiver.recv().await else {
                return Ok(None);
            };
            self.pending = parse_message(&message, &self.instrument_id, &self.interval)?.into();
        }
    }

    pub fn health_snapshot(&self) -> WebsocketHealthSnapshot {
        self.client.health_snapshot()
    }

    pub async fn close(self) {
        self.client.stop().await;
    }
}

fn validate_subscription(instrument_id: &str, interval: &str) -> Result<(), Error> {
    let valid = |value: &str| {
        !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    };
    if !valid(instrument_id) || !valid(interval) {
        return Err(Error::ConfigError(
            "OKX candle subscription instrument/interval 非法".to_string(),
        ));
    }
    Ok(())
}

fn parse_message(
    message: &Value,
    expected_instrument_id: &str,
    expected_interval: &str,
) -> Result<Vec<OkxPublicCandleUpdate>, Error> {
    if message.get("event").and_then(Value::as_str) == Some("error") {
        return Err(Error::WebSocketError(format!(
            "OKX candle subscription rejected code={} message={}",
            message
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            message
                .get("msg")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        )));
    }
    let Some(data) = message.get("data") else {
        return Ok(Vec::new());
    };
    let arg = message
        .get("arg")
        .and_then(Value::as_object)
        .ok_or_else(|| Error::WebSocketError("OKX candle message 缺少 arg".to_string()))?;
    let channel = arg
        .get("channel")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let instrument_id = arg
        .get("instId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected_channel = format!("candle{expected_interval}");
    if channel != expected_channel || instrument_id != expected_instrument_id {
        return Err(Error::WebSocketError(
            "OKX candle message scope 与订阅不一致".to_string(),
        ));
    }
    let rows = data
        .as_array()
        .ok_or_else(|| Error::WebSocketError("OKX candle data 不是数组".to_string()))?;
    rows.iter()
        .map(|row| {
            let values: Vec<String> = serde_json::from_value(row.clone()).map_err(|error| {
                Error::WebSocketError(format!("OKX candle row 解析失败: {error}"))
            })?;
            let candle = CandleOkxRespDto::try_from_vec(values)
                .map_err(|error| Error::WebSocketError(format!("OKX candle row 非法: {error}")))?;
            let confirmed = match candle.confirm.as_str() {
                "0" => false,
                "1" => true,
                _ => {
                    return Err(Error::WebSocketError(
                        "OKX candle confirm 只能是 0 或 1".to_string(),
                    ))
                }
            };
            Ok(OkxPublicCandleUpdate {
                instrument_id: instrument_id.to_string(),
                interval: expected_interval.to_string(),
                timestamp: candle.ts,
                open: candle.o,
                high: candle.h,
                low: candle.l,
                close: candle.c,
                volume: candle.v,
                volume_currency: candle.vol_ccy,
                quote_volume: candle.vol_ccy_quote,
                confirmed,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parser_distinguishes_open_and_confirmed_candles() {
        let open = parse_message(
            &json!({
                "arg": {"channel": "candle4H", "instId": "ETH-USDT-SWAP"},
                "data": [["1786593600000", "4500", "4510", "4490", "4505", "10", "0.1", "45050", "0"]]
            }),
            "ETH-USDT-SWAP",
            "4H",
        )
        .expect("open candle");
        let confirmed = parse_message(
            &json!({
                "arg": {"channel": "candle4H", "instId": "ETH-USDT-SWAP"},
                "data": [["1786593600000", "4500", "4520", "4490", "4515", "20", "0.2", "90300", "1"]]
            }),
            "ETH-USDT-SWAP",
            "4H",
        )
        .expect("confirmed candle");

        assert!(!open[0].confirmed);
        assert!(confirmed[0].confirmed);
        assert_eq!(confirmed[0].close, "4515");
    }

    #[test]
    fn parser_rejects_scope_drift_and_unknown_confirmation() {
        let drift = parse_message(
            &json!({
                "arg": {"channel": "candle1H", "instId": "ETH-USDT-SWAP"},
                "data": [["1", "1", "1", "1", "1", "1", "1", "1", "1"]]
            }),
            "ETH-USDT-SWAP",
            "4H",
        );
        let unknown = parse_message(
            &json!({
                "arg": {"channel": "candle4H", "instId": "ETH-USDT-SWAP"},
                "data": [["1", "1", "1", "1", "1", "1", "1", "1", "2"]]
            }),
            "ETH-USDT-SWAP",
            "4H",
        );

        assert!(drift.is_err());
        assert!(unknown.is_err());
    }
}
