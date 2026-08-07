use crate::account::AccountOrderPermission;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use serde_json::{Map, Value};

const SOURCE_REVISION: &str = "okx-account-config-v1";

/// 将 OKX `perm` 的精确 token 集合映射为非敏感 SDK 协议事实。
pub(super) fn map_account_order_permission(raw: Value) -> Result<AccountOrderPermission> {
    let exchange = ExchangeId::Okx;
    let object = first_object(&raw).ok_or_else(|| Error::Adapter {
        exchange,
        message: "OKX account config response missing object".to_owned(),
    })?;
    let permissions = object
        .get("perm")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX account config response missing perm".to_owned(),
        })?;
    Ok(AccountOrderPermission {
        exchange,
        can_create_orders: permissions
            .split(',')
            .any(|permission| permission.trim() == "trade"),
        source_revision: SOURCE_REVISION.to_owned(),
    })
}

fn first_object(raw: &Value) -> Option<&Map<String, Value>> {
    raw.as_array()
        .and_then(|items| items.first())
        .and_then(Value::as_object)
        .or_else(|| raw.as_object())
}
