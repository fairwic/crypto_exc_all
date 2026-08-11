use super::{
    PrivateAccountStreamChange, PrivateAccountStreamFrame, PrivateAccountStreamRecord,
    PrivateBalanceStreamChange, PrivateOrderStreamChange, PrivateOrderStreamKind,
    PrivatePositionStreamChange, adapter_error, nonempty, number_or_string,
};
use crate::{Error, ExchangeId, Result};
use binance_rs::api::websocket::BinanceWebsocketEvent;
use serde_json::Value;

/// 将 Binance 用户流 frame 转为最小协议事实，不在 SDK 内引入 Account 业务身份。
pub(super) fn parse(payload: Value) -> Result<PrivateAccountStreamFrame> {
    let typed = BinanceWebsocketEvent::parse(payload.clone()).map_err(Error::from_binance)?;
    let raw = payload
        .get("data")
        .filter(|value| value.get("e").is_some())
        .unwrap_or(&payload);
    match typed {
        BinanceWebsocketEvent::ListenKeyExpired(expired) => {
            Ok(PrivateAccountStreamFrame::Expired {
                exchange: ExchangeId::Binance,
                provider_event_time_ms: expired.event_time,
            })
        }
        BinanceWebsocketEvent::OrderTradeUpdate(update) => parse_order_update(raw, update),
        BinanceWebsocketEvent::AccountUpdate(update) => parse_account_update(raw, update),
        _ => Err(adapter_error(
            ExchangeId::Binance,
            "unsupported Binance private account event",
        )),
    }
}

/// 保留订单原始 payload，使 Gateway 能在业务边界计算稳定去重 hash。
fn parse_order_update(
    raw: &Value,
    update: binance_rs::api::websocket::OrderTradeUpdateEvent,
) -> Result<PrivateAccountStreamFrame> {
    let raw_order = raw.get("o").cloned().ok_or_else(|| {
        adapter_error(ExchangeId::Binance, "Binance order update missing payload")
    })?;
    let trade_id = raw_order
        .get("t")
        .and_then(number_or_string)
        .unwrap_or_else(|| "0".to_owned());
    Ok(PrivateAccountStreamFrame::Records(vec![
        PrivateAccountStreamRecord {
            exchange: ExchangeId::Binance,
            provider_event_time_ms: update.event_time,
            provider_transaction_time_ms: Some(update.transaction_time),
            event_identity: format!(
                "order:{}:{}:{trade_id}:{}:{}",
                update.order.symbol,
                update.order.order_id,
                update.transaction_time,
                update.order.execution_type
            ),
            change: PrivateAccountStreamChange::Order(PrivateOrderStreamChange {
                kind: PrivateOrderStreamKind::Regular,
                exchange_symbol: update.order.symbol,
                order_id: update.order.order_id.to_string(),
                client_order_id: nonempty(update.order.client_order_id),
                parent_order_id: None,
                parent_client_order_id: None,
                side: nonempty(update.order.side),
                order_type: nonempty(update.order.order_type),
                price: nonempty(update.order.original_price),
                size: nonempty(update.order.original_quantity),
                filled_size: raw_order.get("z").and_then(number_or_string),
                average_fill_price: raw_order.get("ap").and_then(number_or_string),
                status: update.order.status,
                created_at_ms: raw_order
                    .get("O")
                    .and_then(number_or_string)
                    .and_then(|value| value.parse().ok()),
                source_updated_at_ms: update.transaction_time,
            }),
            raw_payload: raw_order,
        },
    ]))
}

/// `ACCOUNT_UPDATE` 可只包含余额或持仓，缺失部分按“本帧未变化”而非协议错误处理。
fn parse_account_update(
    raw: &Value,
    update: binance_rs::api::websocket::AccountUpdateEvent,
) -> Result<PrivateAccountStreamFrame> {
    let raw_account = raw.get("a").ok_or_else(|| {
        adapter_error(
            ExchangeId::Binance,
            "Binance account update missing payload",
        )
    })?;
    let raw_balances = optional_update_items(
        raw_account,
        "B",
        "Binance account update balances is not an array",
    )?;
    let raw_positions = optional_update_items(
        raw_account,
        "P",
        "Binance account update positions is not an array",
    )?;
    if raw_balances.len() != update.data.balances.len()
        || raw_positions.len() != update.data.positions.len()
    {
        return Err(adapter_error(
            ExchangeId::Binance,
            "Binance account update typed/raw length mismatch",
        ));
    }
    let mut records = Vec::with_capacity(raw_balances.len() + raw_positions.len());
    for (balance, raw_balance) in update.data.balances.iter().zip(raw_balances) {
        records.push(PrivateAccountStreamRecord {
            exchange: ExchangeId::Binance,
            provider_event_time_ms: update.event_time,
            provider_transaction_time_ms: Some(update.transaction_time),
            event_identity: format!(
                "balance:{}:{}:{}",
                balance.asset, update.transaction_time, update.data.reason
            ),
            change: PrivateAccountStreamChange::Balance(PrivateBalanceStreamChange {
                asset: balance.asset.clone(),
                total: balance.wallet_balance.clone(),
                available: None,
                source_updated_at_ms: update.transaction_time,
            }),
            raw_payload: raw_balance.clone(),
        });
    }
    for (position, raw_position) in update.data.positions.iter().zip(raw_positions) {
        records.push(PrivateAccountStreamRecord {
            exchange: ExchangeId::Binance,
            provider_event_time_ms: update.event_time,
            provider_transaction_time_ms: Some(update.transaction_time),
            event_identity: format!(
                "position:{}:{}:{}:{}",
                position.symbol,
                position.position_side,
                update.transaction_time,
                update.data.reason
            ),
            change: PrivateAccountStreamChange::Position(PrivatePositionStreamChange {
                exchange_symbol: position.symbol.clone(),
                side: nonempty(position.position_side.clone()),
                size: position.position_amount.clone(),
                entry_price: nonempty(position.entry_price.clone()),
                mark_price: None,
                unrealized_pnl: nonempty(position.unrealized_pnl.clone()),
                leverage: None,
                margin_mode: nonempty(position.margin_type.clone()),
                liquidation_price: None,
                source_updated_at_ms: update.transaction_time,
            }),
            raw_payload: raw_position.clone(),
        });
    }
    Ok(PrivateAccountStreamFrame::Records(records))
}

/// Binance 会省略未变化的 `B` 或 `P`；存在但不是数组时仍按协议损坏处理。
fn optional_update_items<'a>(
    account: &'a Value,
    field: &str,
    invalid_message: &'static str,
) -> Result<&'a [Value]> {
    match account.get(field) {
        None => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(adapter_error(ExchangeId::Binance, invalid_message)),
    }
}
