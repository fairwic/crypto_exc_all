use super::non_empty;
use crate::account::{Balance, SourcedBalance};
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use binance_rs::dto::account::AccountBalance as BinanceAccountBalance;

/// 将 Binance 余额行转换为带来源时间的事实，同时剔除未初始化的资产目录行。
pub(super) fn map_sourced_balance(
    balance: BinanceAccountBalance,
) -> Result<Option<SourcedBalance>> {
    if balance.update_time == 0 {
        // 多资产模式会为从未变动的资产返回正的 availableBalance；它表示跨资产可用额度，
        // 不是该资产有时间戳的余额事实。只有钱包相关金额都为零时才能安全忽略该目录行。
        let has_wallet_state = [
            balance.balance.as_str(),
            balance.cross_wallet_balance.as_str(),
            balance.cross_un_pnl.as_str(),
            balance.max_withdraw_amount.as_str(),
        ]
        .into_iter()
        .any(|value| !is_zero_decimal_text(value));
        if has_wallet_state {
            return Err(Error::Adapter {
                exchange: ExchangeId::Binance,
                message: "Binance balance response has wallet state without updateTime".to_owned(),
            });
        }
        return Ok(None);
    }

    let raw = serde_json::to_value(&balance)?;
    Ok(Some(SourcedBalance {
        balance: Balance {
            exchange: ExchangeId::Binance,
            asset: balance.asset,
            total: balance.balance,
            available: balance.available_balance,
            frozen: non_empty(balance.cross_un_pnl),
            raw,
        },
        source_updated_at_ms: balance.update_time,
    }))
}

/// 判断交易所十进制文本是否精确为零；格式异常必须按非零处理并触发 fail-closed。
fn is_zero_decimal_text(value: &str) -> bool {
    let unsigned = value
        .trim()
        .strip_prefix(['+', '-'])
        .unwrap_or_else(|| value.trim());
    let mut saw_digit = false;
    let mut saw_decimal_point = false;
    for byte in unsigned.bytes() {
        match byte {
            b'0' => saw_digit = true,
            b'.' if !saw_decimal_point => saw_decimal_point = true,
            _ => return false,
        }
    }
    saw_digit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_uninitialized_balance_row_without_synthesizing_source_time() {
        let row = BinanceAccountBalance {
            account_alias: "account-alias".to_owned(),
            asset: "BNB".to_owned(),
            balance: "0.00000000".to_owned(),
            cross_wallet_balance: "0.00000000".to_owned(),
            cross_un_pnl: "0.00000000".to_owned(),
            available_balance: "42.00000000".to_owned(),
            max_withdraw_amount: "0.00000000".to_owned(),
            margin_available: true,
            update_time: 0,
        };

        let mapped = map_sourced_balance(row).expect("valid unused asset row");

        assert_eq!(mapped, None);
    }

    #[test]
    fn rejects_wallet_balance_without_provider_update_time() {
        let row = BinanceAccountBalance {
            account_alias: "account-alias".to_owned(),
            asset: "USDT".to_owned(),
            balance: "1.00000000".to_owned(),
            cross_wallet_balance: "1.00000000".to_owned(),
            cross_un_pnl: "0.00000000".to_owned(),
            available_balance: "1.00000000".to_owned(),
            max_withdraw_amount: "1.00000000".to_owned(),
            margin_available: true,
            update_time: 0,
        };

        let error = map_sourced_balance(row).expect_err("missing source time");

        assert!(matches!(
            error,
            Error::Adapter {
                exchange: ExchangeId::Binance,
                ..
            }
        ));
        assert!(!error.to_string().contains("1.00000000"));
    }
}
