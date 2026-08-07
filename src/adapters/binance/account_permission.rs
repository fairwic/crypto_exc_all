use crate::account::AccountOrderPermission;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use serde_json::Value;

const SOURCE_REVISION: &str = "binance-usdm-account-config-v1";

/// 将 Binance USDⓈ-M `canTrade` 精确映射为非敏感 SDK 协议事实。
pub(super) fn map_account_order_permission(raw: Value) -> Result<AccountOrderPermission> {
    let exchange = ExchangeId::Binance;
    let can_create_orders = raw
        .as_object()
        .and_then(|object| object.get("canTrade"))
        .and_then(Value::as_bool)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "Binance accountConfig missing canTrade".to_owned(),
        })?;
    Ok(AccountOrderPermission {
        exchange,
        can_create_orders,
        source_revision: SOURCE_REVISION.to_owned(),
    })
}
