use super::{string_field, u64_field};
use crate::account::AccountMarginSummary;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use serde_json::{Map, Value};

/// 把同一 USDⓈ-M Account Information V2 响应映射成单资产 USDT 摘要。
pub(super) fn map_margin_summary(
    account: Value,
    quote_currency: &str,
) -> Result<AccountMarginSummary> {
    let exchange = ExchangeId::Binance;
    if quote_currency != "USDT" {
        return Err(Error::Adapter {
            exchange,
            message: "Binance margin summary only supports USDT".to_owned(),
        });
    }
    let account = object(&account, "Binance account information")?;
    let multi_assets = account
        .get("multiAssetsMargin")
        .and_then(Value::as_bool)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "Binance account information missing multiAssetsMargin".to_owned(),
        })?;
    if multi_assets {
        return Err(Error::Adapter {
            exchange,
            message: "Binance multi-assets margin is USD-denominated, not USDT".to_owned(),
        });
    }
    Ok(AccountMarginSummary {
        exchange,
        quote_currency: quote_currency.to_owned(),
        account_equity: required_string(account, "totalMarginBalance")?,
        available_margin: required_string(account, "availableBalance")?,
        initial_margin: Some(required_string(account, "totalInitialMargin")?),
        position_initial_margin: Some(required_string(account, "totalPositionInitialMargin")?),
        open_order_initial_margin: Some(required_string(account, "totalOpenOrderInitialMargin")?),
        source_updated_at_ms: latest_update_time(account)?,
        source_revision: "binance-usds-account-v2-single-asset-v1".to_owned(),
    })
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>> {
    value.as_object().ok_or_else(|| Error::Adapter {
        exchange: ExchangeId::Binance,
        message: format!("{label} response is not an object"),
    })
}

fn required_string(object: &Map<String, Value>, field: &'static str) -> Result<String> {
    string_field(object, field).ok_or_else(|| Error::Adapter {
        exchange: ExchangeId::Binance,
        message: format!("Binance account information missing {field}"),
    })
}

/// 账户级 totals 没有独立时间；使用其资产与持仓中最大的官方更新时间。
fn latest_update_time(account: &Map<String, Value>) -> Result<u64> {
    let latest = ["assets", "positions"]
        .into_iter()
        .filter_map(|field| account.get(field).and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_object)
        .filter_map(|item| u64_field(item, "updateTime"))
        .max()
        .filter(|value| *value > 0);
    latest.ok_or_else(|| Error::Adapter {
        exchange: ExchangeId::Binance,
        message: "Binance account information has no provider updateTime".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_single_asset_usdt_totals_and_latest_provider_time() {
        let summary =
            map_margin_summary(account_fixture(false), "USDT").expect("single-asset USDT summary");

        assert_eq!(summary.account_equity, "126.72469206");
        assert_eq!(summary.available_margin, "120.00000000");
        assert_eq!(summary.initial_margin.as_deref(), Some("6.72469206"));
        assert_eq!(summary.position_initial_margin.as_deref(), Some("5"));
        assert_eq!(
            summary.open_order_initial_margin.as_deref(),
            Some("1.72469206")
        );
        assert_eq!(summary.source_updated_at_ms, 1_625_474_304_766);
    }

    #[test]
    fn rejects_multi_assets_totals_instead_of_labeling_usd_as_usdt() {
        let error = map_margin_summary(account_fixture(true), "USDT")
            .expect_err("multi-assets totals are USD-denominated");

        assert!(error.to_string().contains("USD-denominated"));
    }

    fn account_fixture(multi_assets_margin: bool) -> Value {
        json!({
            "multiAssetsMargin": multi_assets_margin,
            "totalMarginBalance": "126.72469206",
            "availableBalance": "120.00000000",
            "totalInitialMargin": "6.72469206",
            "totalPositionInitialMargin": "5",
            "totalOpenOrderInitialMargin": "1.72469206",
            "assets": [{"asset": "USDT", "updateTime": 1625474304765_u64}],
            "positions": [{"symbol": "ETHUSDT", "updateTime": 1625474304766_u64}]
        })
    }
}
