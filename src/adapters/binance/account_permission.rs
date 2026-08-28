use crate::account::{AccountIdentity, AccountOrderPermission, AccountOrderPermissionWithIdentity};
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use serde_json::Value;

const SOURCE_REVISION: &str = "binance-usdm-account-config-v1";

/// 将 Binance USDⓈ-M `canTrade` 精确映射为非敏感 SDK 协议事实。
pub(super) fn map_account_order_permission(raw: &Value) -> Result<AccountOrderPermission> {
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

pub(super) fn map_account_order_permission_with_identity(
    provider_account_id: String,
    raw: &Value,
) -> Result<AccountOrderPermissionWithIdentity> {
    Ok(AccountOrderPermissionWithIdentity {
        identity: map_account_identity(provider_account_id, raw)?,
        order_permission: map_account_order_permission(raw)?,
    })
}

pub(super) fn map_account_identity(
    provider_account_id: String,
    raw: &Value,
) -> Result<AccountIdentity> {
    let exchange = ExchangeId::Binance;
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Binance accountConfig response is not an object".to_owned(),
    })?;
    let multi_assets = object
        .get("multiAssetsMargin")
        .and_then(Value::as_bool)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "Binance accountConfig missing multiAssetsMargin".to_owned(),
        })?;
    let dual_side = object
        .get("dualSidePosition")
        .and_then(Value::as_bool)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "Binance accountConfig missing dualSidePosition".to_owned(),
        })?;
    Ok(AccountIdentity {
        exchange,
        provider_account_id,
        parent_account_id: None,
        margin_mode: if multi_assets {
            "multi_asset"
        } else {
            "single_asset"
        }
        .to_owned(),
        position_mode: if dual_side { "hedge" } else { "one_way" }.to_owned(),
        settlement_asset: "USDT".to_owned(),
    })
}
