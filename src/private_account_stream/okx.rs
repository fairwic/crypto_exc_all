use super::{
    PrivateAccountStreamChange, PrivateAccountStreamFrame, PrivateAccountStreamRecord,
    PrivateBalanceStreamChange, PrivateOrderStreamChange, PrivatePositionStreamChange,
    adapter_error, number_or_string,
};
use crate::{ExchangeId, Result};
use serde_json::Value;

/// 将 OKX account/positions/orders channel 收敛为最小协议事实。
pub(super) fn parse(payload: Value) -> Result<PrivateAccountStreamFrame> {
    if let Some(event) = payload.get("event").and_then(Value::as_str) {
        if matches!(event, "login" | "subscribe" | "unsubscribe") {
            return Ok(PrivateAccountStreamFrame::Control {
                exchange: ExchangeId::Okx,
                event: event.to_owned(),
            });
        }
        return Err(adapter_error(
            ExchangeId::Okx,
            "unknown OKX private control event",
        ));
    }
    let channel = payload
        .pointer("/arg/channel")
        .and_then(Value::as_str)
        .ok_or_else(|| adapter_error(ExchangeId::Okx, "OKX private frame missing channel"))?;
    let data = payload
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| adapter_error(ExchangeId::Okx, "OKX private frame missing data"))?;
    match channel {
        "account" => parse_balances(data),
        "positions" => parse_positions(data),
        "orders" => parse_orders(data),
        _ => Err(adapter_error(
            ExchangeId::Okx,
            "unsupported OKX private account channel",
        )),
    }
}

/// OKX 账户 frame 可携带多个币种明细，各自保留原始更新时间和 payload。
fn parse_balances(data: &[Value]) -> Result<PrivateAccountStreamFrame> {
    let mut records = Vec::new();
    for account in data {
        let fallback_time = optional_u64(account, "uTime")?;
        let details = account
            .get("details")
            .and_then(Value::as_array)
            .ok_or_else(|| adapter_error(ExchangeId::Okx, "OKX account frame missing details"))?;
        for detail in details {
            let asset = required_string(detail, "ccy", ExchangeId::Okx)?;
            let update_time = optional_u64(detail, "uTime")?
                .or(fallback_time)
                .ok_or_else(|| adapter_error(ExchangeId::Okx, "OKX account frame missing uTime"))?;
            records.push(PrivateAccountStreamRecord {
                exchange: ExchangeId::Okx,
                provider_event_time_ms: update_time,
                provider_transaction_time_ms: Some(update_time),
                event_identity: format!("account:{asset}:{update_time}"),
                change: PrivateAccountStreamChange::Balance(PrivateBalanceStreamChange {
                    asset,
                    total: required_string(detail, "eq", ExchangeId::Okx)?,
                    available: optional_nonempty_string(detail, "availBal")
                        .or_else(|| optional_nonempty_string(detail, "availEq")),
                    source_updated_at_ms: update_time,
                }),
                raw_payload: detail.clone(),
            });
        }
    }
    Ok(PrivateAccountStreamFrame::Records(records))
}

/// 持仓 identity 优先使用 `posId`，缺失时只用 provider 的 symbol/side 组合兜底。
fn parse_positions(data: &[Value]) -> Result<PrivateAccountStreamFrame> {
    let mut records = Vec::with_capacity(data.len());
    for position in data {
        let update_time = required_u64(position, "uTime", ExchangeId::Okx)?;
        let push_time = optional_u64(position, "pTime")?.unwrap_or(update_time);
        let position_id = optional_nonempty_string(position, "posId").unwrap_or_else(|| {
            format!(
                "{}:{}",
                value_string(position, "instId"),
                value_string(position, "posSide")
            )
        });
        if position_id == ":" {
            return Err(adapter_error(
                ExchangeId::Okx,
                "OKX position frame missing business identity",
            ));
        }
        records.push(PrivateAccountStreamRecord {
            exchange: ExchangeId::Okx,
            provider_event_time_ms: push_time,
            provider_transaction_time_ms: Some(update_time),
            event_identity: format!("position:{position_id}:{update_time}:{push_time}"),
            change: PrivateAccountStreamChange::Position(PrivatePositionStreamChange {
                exchange_symbol: required_string(position, "instId", ExchangeId::Okx)?,
                side: optional_nonempty_string(position, "posSide"),
                size: required_string(position, "pos", ExchangeId::Okx)?,
                entry_price: optional_nonempty_string(position, "avgPx"),
                mark_price: optional_nonempty_string(position, "markPx"),
                unrealized_pnl: optional_nonempty_string(position, "upl"),
                leverage: optional_nonempty_string(position, "lever"),
                margin_mode: optional_nonempty_string(position, "mgnMode"),
                liquidation_price: optional_nonempty_string(position, "liqPx"),
                source_updated_at_ms: update_time,
            }),
            raw_payload: position.clone(),
        });
    }
    Ok(PrivateAccountStreamFrame::Records(records))
}

/// 订单 frame 保留 provider 订单和成交 identity，避免 SDK 推断业务订单归属。
fn parse_orders(data: &[Value]) -> Result<PrivateAccountStreamFrame> {
    let mut records = Vec::with_capacity(data.len());
    for order in data {
        let order_id = required_string(order, "ordId", ExchangeId::Okx)?;
        let symbol = required_string(order, "instId", ExchangeId::Okx)?;
        let update_time = required_u64(order, "uTime", ExchangeId::Okx)?;
        let trade_id =
            optional_nonempty_string(order, "tradeId").unwrap_or_else(|| "none".to_owned());
        records.push(PrivateAccountStreamRecord {
            exchange: ExchangeId::Okx,
            provider_event_time_ms: update_time,
            provider_transaction_time_ms: Some(update_time),
            event_identity: format!("order:{symbol}:{order_id}:{trade_id}:{update_time}"),
            change: PrivateAccountStreamChange::Order(PrivateOrderStreamChange {
                exchange_symbol: symbol,
                order_id,
                client_order_id: optional_nonempty_string(order, "clOrdId"),
                side: optional_nonempty_string(order, "side"),
                order_type: optional_nonempty_string(order, "ordType"),
                price: optional_nonempty_string(order, "px"),
                size: optional_nonempty_string(order, "sz"),
                filled_size: optional_nonempty_string(order, "accFillSz"),
                status: required_string(order, "state", ExchangeId::Okx)?,
                created_at_ms: optional_u64(order, "cTime")?,
                source_updated_at_ms: update_time,
            }),
            raw_payload: order.clone(),
        });
    }
    Ok(PrivateAccountStreamFrame::Records(records))
}

/// 读取私有流中的必填非空文本；错误只保留协议边界，不回显 payload。
fn required_string(value: &Value, field: &str, exchange: ExchangeId) -> Result<String> {
    optional_nonempty_string(value, field)
        .ok_or_else(|| adapter_error(exchange, "private account frame missing identity"))
}

/// 读取私有流中的必填正时间戳。
fn required_u64(value: &Value, field: &str, exchange: ExchangeId) -> Result<u64> {
    optional_u64(value, field)?
        .ok_or_else(|| adapter_error(exchange, "private account frame missing timestamp"))
}

/// OKX 时间字段可能是字符串或 JSON 数字，但零值不构成可比较的 provider 时间。
fn optional_u64(value: &Value, field: &str) -> Result<Option<u64>> {
    value
        .get(field)
        .map(|value| {
            number_or_string(value)
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|value| *value > 0)
                .ok_or_else(|| adapter_error(ExchangeId::Okx, "invalid OKX private timestamp"))
        })
        .transpose()
}

/// 保留 provider 文本原值，仅把空白文本收敛为缺失。
fn optional_nonempty_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// 仅用于构造 provider identity 兜底，不向公开合同暴露空字符串。
fn value_string(value: &Value, field: &str) -> String {
    optional_nonempty_string(value, field).unwrap_or_default()
}
