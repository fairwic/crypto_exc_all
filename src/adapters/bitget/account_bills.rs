use super::BitgetAdapter;
use crate::account::{AccountBill, AccountBillQuery};
use crate::adapters::value::{
    map_first_string_field as first_string_field, map_first_u64_field as first_u64_field,
};
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use bitget_rs::api::account::AccountBillRequest;
use serde_json::Value;

impl BitgetAdapter {
    pub(crate) async fn account_bills(&self, query: AccountBillQuery) -> Result<Vec<AccountBill>> {
        if query.instrument.is_some() || query.archive {
            return Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "account bills filters",
            });
        }

        let product_type = query.inst_type.as_deref().unwrap_or(&self.product_type);
        let mut request = AccountBillRequest::new(product_type);
        if let Some(asset) = query.asset.as_deref() {
            request = request.with_coin(asset);
        }
        if let Some(bill_type) = query.bill_type.as_deref() {
            request = request.with_business_type(bill_type);
        }
        if let Some(start_time) = query.start_time {
            request = request.with_start_time(start_time);
        }
        if let Some(end_time) = query.end_time {
            request = request.with_end_time(end_time);
        }
        if let Some(limit) = query.limit {
            request = request.with_limit(limit);
        }

        let raw = self
            .account
            .get_account_bills(request)
            .await
            .map_err(Error::from_bitget)?;
        bitget_account_bills_from_value(raw)
    }
}

fn bitget_account_bills_from_value(raw: Value) -> Result<Vec<AccountBill>> {
    value_items(raw)?
        .into_iter()
        .map(bitget_account_bill)
        .collect()
}

fn value_items(raw: Value) -> Result<Vec<Value>> {
    match raw {
        Value::Array(values) => Ok(values),
        Value::Object(mut object) => object
            .remove("bills")
            .or_else(|| object.remove("items"))
            .and_then(|value| value.as_array().cloned())
            .ok_or_else(|| Error::Adapter {
                exchange: ExchangeId::Bitget,
                message: "Bitget account bills response does not contain bills".to_string(),
            }),
        _ => Err(Error::Adapter {
            exchange: ExchangeId::Bitget,
            message: "Bitget account bills response is neither an array nor an object".to_string(),
        }),
    }
}

fn bitget_account_bill(raw: Value) -> Result<AccountBill> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange: ExchangeId::Bitget,
        message: "Bitget account bill item is not an object".to_string(),
    })?;

    Ok(AccountBill {
        exchange: ExchangeId::Bitget,
        instrument: None,
        exchange_symbol: first_string_field(object, &["symbol", "instId"]),
        bill_id: first_string_field(object, &["id", "billId"]),
        asset: first_string_field(object, &["coin", "marginCoin"]),
        balance_change: first_string_field(object, &["amount", "change", "balanceChange"]),
        balance_after: first_string_field(object, &["balance", "balanceAfter"]),
        fee: first_string_field(object, &["fee"]),
        pnl: first_string_field(object, &["pnl", "profit"]),
        bill_type: first_string_field(object, &["businessType", "type"]),
        bill_sub_type: first_string_field(object, &["bizType", "status"]),
        order_id: first_string_field(object, &["orderId"]),
        trade_id: first_string_field(object, &["tradeId"]),
        timestamp: first_u64_field(object, &["uTime", "cTime", "ts", "time", "timestamp"]),
        raw,
    })
}
