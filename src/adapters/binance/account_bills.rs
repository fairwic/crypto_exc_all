use super::BinanceAdapter;
use crate::account::{AccountBill, AccountBillQuery};
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use binance_rs::api::asset::{
    DepositHistoryRequest, UniversalTransferHistoryRequest, WithdrawHistoryRequest,
};
use serde_json::Value;

impl BinanceAdapter {
    pub(crate) async fn account_bills(&self, query: AccountBillQuery) -> Result<Vec<AccountBill>> {
        ensure_account_bill_query(&query)?;

        let bill_type = query.bill_type.as_deref().unwrap_or("dnw").trim();
        match bill_type.to_ascii_lowercase().as_str() {
            "all" | "dnw" => {
                let mut output = self.deposit_bills(&query).await?;
                output.extend(self.withdrawal_bills(&query).await?);
                Ok(output)
            }
            "deposit" => self.deposit_bills(&query).await,
            "withdraw" | "withdrawal" => self.withdrawal_bills(&query).await,
            "transfer" => Err(Error::Unsupported {
                exchange: ExchangeId::Binance,
                capability: "account bills transfer type",
            }),
            _ => self.transfer_bills(&query, bill_type).await,
        }
    }

    async fn deposit_bills(&self, query: &AccountBillQuery) -> Result<Vec<AccountBill>> {
        let mut request = DepositHistoryRequest::new();
        if let Some(asset) = query.asset.as_deref() {
            request = request.with_coin(asset);
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
            .asset
            .get_deposit_history(request)
            .await
            .map_err(Error::from_binance)?;
        binance_account_bills_from_value(raw, "deposit", &["id", "txId"])
    }

    async fn withdrawal_bills(&self, query: &AccountBillQuery) -> Result<Vec<AccountBill>> {
        let mut request = WithdrawHistoryRequest::new();
        if let Some(asset) = query.asset.as_deref() {
            request = request.with_coin(asset);
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
            .asset
            .get_withdraw_history(request)
            .await
            .map_err(Error::from_binance)?;
        binance_account_bills_from_value(raw, "withdrawal", &["id", "withdrawOrderId", "txId"])
    }

    async fn transfer_bills(
        &self,
        query: &AccountBillQuery,
        transfer_type: &str,
    ) -> Result<Vec<AccountBill>> {
        if query.asset.is_some() {
            return Err(Error::Unsupported {
                exchange: ExchangeId::Binance,
                capability: "account bills transfer asset filter",
            });
        }

        let mut request = UniversalTransferHistoryRequest::new(transfer_type);
        if let Some(start_time) = query.start_time {
            request = request.with_start_time(start_time);
        }
        if let Some(end_time) = query.end_time {
            request = request.with_end_time(end_time);
        }
        if let Some(limit) = query.limit {
            request = request.with_size(limit);
        }

        let raw = self
            .asset
            .get_transfer_history(request)
            .await
            .map_err(Error::from_binance)?;
        binance_account_bills_from_value(raw, transfer_type, &["tranId", "id"])
    }
}

fn ensure_account_bill_query(query: &AccountBillQuery) -> Result<()> {
    if query.instrument.is_some() || query.inst_type.is_some() || query.archive {
        return Err(Error::Unsupported {
            exchange: ExchangeId::Binance,
            capability: "account bills filters",
        });
    }

    Ok(())
}

fn binance_account_bills_from_value(
    raw: Value,
    fallback_type: &str,
    id_fields: &[&str],
) -> Result<Vec<AccountBill>> {
    value_items(raw, "Binance account bills response")?
        .into_iter()
        .map(|item| binance_account_bill(item, fallback_type, id_fields))
        .collect()
}

fn value_items(raw: Value, label: &str) -> Result<Vec<Value>> {
    match raw {
        Value::Array(values) => Ok(values),
        Value::Object(mut object) => object
            .remove("rows")
            .or_else(|| object.remove("data"))
            .and_then(|value| value.as_array().cloned())
            .ok_or_else(|| Error::Adapter {
                exchange: ExchangeId::Binance,
                message: format!("{label} does not contain rows"),
            }),
        _ => Err(Error::Adapter {
            exchange: ExchangeId::Binance,
            message: format!("{label} is neither an array nor an object"),
        }),
    }
}

fn binance_account_bill(
    raw: Value,
    fallback_type: &str,
    id_fields: &[&str],
) -> Result<AccountBill> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange: ExchangeId::Binance,
        message: "Binance account bill item is not an object".to_string(),
    })?;

    Ok(AccountBill {
        exchange: ExchangeId::Binance,
        instrument: None,
        exchange_symbol: None,
        bill_id: first_string_field(object, id_fields),
        asset: first_string_field(object, &["coin", "asset"]),
        balance_change: first_string_field(object, &["amount", "balanceChange"]),
        balance_after: first_string_field(object, &["balance", "balanceAfter"]),
        fee: first_string_field(object, &["transactionFee", "fee"]),
        pnl: first_string_field(object, &["realizedPnl", "pnl"]),
        bill_type: first_string_field(object, &["type"])
            .or_else(|| Some(fallback_type.to_string())),
        bill_sub_type: first_string_field(object, &["status"]),
        order_id: first_string_field(object, &["orderId", "clientTranId"]),
        trade_id: first_string_field(object, &["tradeId"]),
        timestamp: first_u64_field(
            object,
            &[
                "timestamp",
                "insertTime",
                "applyTime",
                "completeTime",
                "time",
            ],
        ),
        raw,
    })
}

fn first_string_field(object: &serde_json::Map<String, Value>, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| string_field(object, field))
}

fn string_field(object: &serde_json::Map<String, Value>, field: &str) -> Option<String> {
    object.get(field).and_then(non_empty_value)
}

fn first_u64_field(object: &serde_json::Map<String, Value>, fields: &[&str]) -> Option<u64> {
    fields.iter().find_map(|field| u64_field(object, field))
}

fn u64_field(object: &serde_json::Map<String, Value>, field: &str) -> Option<u64> {
    object.get(field).and_then(|value| match value {
        Value::Number(value) => value.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    })
}

fn non_empty_value(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}
