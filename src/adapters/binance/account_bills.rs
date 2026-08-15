use super::BinanceAdapter;
use crate::account::{AccountBill, AccountBillQuery};
use crate::adapters::value::{
    map_first_string_field as first_string_field, map_first_u64_field as first_u64_field,
};
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use binance_rs::api::account::IncomeHistoryRequest;
use binance_rs::api::asset::{
    DepositHistoryRequest, UniversalTransferHistoryRequest, WithdrawHistoryRequest,
};
use serde_json::Value;

impl BinanceAdapter {
    pub(crate) async fn account_bills(&self, query: AccountBillQuery) -> Result<Vec<AccountBill>> {
        let bill_type = query.bill_type.as_deref().unwrap_or("dnw").trim();
        match bill_type.to_ascii_lowercase().as_str() {
            "income" => self.income_bills(&query, None).await,
            "funding_fee" => self.income_bills(&query, Some("FUNDING_FEE")).await,
            "all" | "dnw" => {
                ensure_asset_bill_query(&query)?;
                let mut output = self.deposit_bills(&query).await?;
                output.extend(self.withdrawal_bills(&query).await?);
                Ok(output)
            }
            "deposit" => {
                ensure_asset_bill_query(&query)?;
                self.deposit_bills(&query).await
            }
            "withdraw" | "withdrawal" => {
                ensure_asset_bill_query(&query)?;
                self.withdrawal_bills(&query).await
            }
            "transfer" => Err(Error::Unsupported {
                exchange: ExchangeId::Binance,
                capability: "account bills transfer type",
            }),
            _ => {
                ensure_asset_bill_query(&query)?;
                self.transfer_bills(&query, bill_type).await
            }
        }
    }

    async fn income_bills(
        &self,
        query: &AccountBillQuery,
        income_type: Option<&str>,
    ) -> Result<Vec<AccountBill>> {
        if query.archive
            || query
                .inst_type
                .as_deref()
                .is_some_and(|value| !matches!(value, "SWAP" | "PERPETUAL"))
        {
            return Err(Error::Unsupported {
                exchange: ExchangeId::Binance,
                capability: "futures income history filters",
            });
        }
        let mut request = IncomeHistoryRequest::new();
        if let Some(instrument) = query.instrument.as_ref() {
            request = request.with_symbol(instrument.symbol_for(ExchangeId::Binance));
        }
        if let Some(income_type) = income_type {
            request = request.with_income_type(income_type);
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
            .get_income_history(request)
            .await
            .map_err(Error::from_binance)?;
        binance_income_bills_from_value(raw, query)
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

fn ensure_asset_bill_query(query: &AccountBillQuery) -> Result<()> {
    if query.instrument.is_some() || query.inst_type.is_some() || query.archive {
        return Err(Error::Unsupported {
            exchange: ExchangeId::Binance,
            capability: "account bills filters",
        });
    }

    Ok(())
}

fn binance_income_bills_from_value(
    raw: Value,
    query: &AccountBillQuery,
) -> Result<Vec<AccountBill>> {
    let expected_asset = query.asset.as_deref();
    let expected_symbol = query
        .instrument
        .as_ref()
        .map(|instrument| instrument.symbol_for(ExchangeId::Binance));
    value_items(raw, "Binance futures income response")?
        .into_iter()
        .filter_map(|item| {
            let object = match item.as_object() {
                Some(object) => object,
                None => {
                    return Some(Err(Error::Adapter {
                        exchange: ExchangeId::Binance,
                        message: "Binance futures income item is not an object".to_owned(),
                    }));
                }
            };
            let asset = first_string_field(object, &["asset"]);
            let symbol = first_string_field(object, &["symbol"]);
            if expected_asset.is_some_and(|expected| asset.as_deref() != Some(expected))
                || expected_symbol
                    .as_deref()
                    .is_some_and(|expected| symbol.as_deref() != Some(expected))
            {
                return None;
            }
            Some(Ok(AccountBill {
                exchange: ExchangeId::Binance,
                instrument: query.instrument.clone(),
                exchange_symbol: symbol,
                bill_id: first_string_field(object, &["tranId"]),
                asset,
                balance_change: first_string_field(object, &["income"]),
                balance_after: None,
                fee: None,
                pnl: None,
                bill_type: first_string_field(object, &["incomeType"]),
                bill_sub_type: first_string_field(object, &["info"]),
                order_id: None,
                trade_id: first_string_field(object, &["tradeId"]),
                timestamp: first_u64_field(object, &["time"]),
                raw: item,
            }))
        })
        .collect()
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
