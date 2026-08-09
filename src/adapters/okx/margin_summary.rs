use super::{non_empty, parse_u64_string};
use crate::account::AccountMarginSummary;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use okx_rs::dto::account_dto::Balance;

/// 把 OKX balance 币种明细映射为同币种保证金摘要。
pub(super) fn map_margin_summary(
    accounts: Vec<Balance>,
    quote_currency: &str,
) -> Result<AccountMarginSummary> {
    let exchange = ExchangeId::Okx;
    if quote_currency != "USDT" {
        return Err(Error::Adapter {
            exchange,
            message: "OKX margin summary only supports USDT".to_owned(),
        });
    }
    if accounts.len() != 1 {
        return Err(Error::Adapter {
            exchange,
            message: "OKX balance response must contain one account summary".to_owned(),
        });
    }
    let mut account = accounts.into_iter().next().expect("one account checked");
    let source_updated_at_ms = parse_u64_string(&account.u_time)
        .filter(|value| *value > 0)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX balance response missing uTime".to_owned(),
        })?;
    let matching = account
        .details
        .iter()
        .filter(|detail| detail.ccy == quote_currency)
        .count();
    if matching != 1 {
        return Err(Error::Adapter {
            exchange,
            message: "OKX balance response has no unique USDT detail".to_owned(),
        });
    }
    let detail = account
        .details
        .drain(..)
        .find(|detail| detail.ccy == quote_currency)
        .expect("one detail checked");
    if detail.eq.is_empty() || detail.avail_eq.is_empty() {
        return Err(Error::Adapter {
            exchange,
            message: "OKX USDT detail missing eq or availEq".to_owned(),
        });
    }

    Ok(AccountMarginSummary {
        exchange,
        quote_currency: quote_currency.to_owned(),
        account_equity: detail.eq,
        available_margin: detail.avail_eq,
        initial_margin: non_empty(detail.imr),
        position_initial_margin: None,
        open_order_initial_margin: None,
        source_updated_at_ms,
        source_revision: "okx-account-balance-currency-detail-v1".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use okx_rs::dto::account_dto::BalanceDetail;

    #[test]
    fn uses_currency_equity_and_available_equity_not_available_balance() {
        let summary = map_margin_summary(vec![account_fixture()], "USDT")
            .expect("USDT currency margin summary");

        assert_eq!(summary.account_equity, "100.5");
        assert_eq!(summary.available_margin, "80.25");
        assert_ne!(summary.available_margin, "90");
        assert_eq!(summary.initial_margin.as_deref(), Some("20.25"));
        assert_eq!(summary.position_initial_margin, None);
        assert_eq!(summary.open_order_initial_margin, None);
        assert_eq!(summary.source_updated_at_ms, 1_597_026_383_085);
    }

    fn account_fixture() -> Balance {
        Balance {
            u_time: "1597026383085".to_owned(),
            total_eq: "100.5".to_owned(),
            iso_eq: String::new(),
            adj_eq: String::new(),
            avail_eq: String::new(),
            ord_froz: String::new(),
            imr: String::new(),
            mmr: String::new(),
            borrow_froz: String::new(),
            mgn_ratio: String::new(),
            notional_usd: String::new(),
            notional_usd_for_borrow: String::new(),
            notional_usd_for_swap: String::new(),
            notional_usd_for_futures: String::new(),
            notional_usd_for_option: String::new(),
            upl: String::new(),
            details: vec![detail_fixture()],
        }
    }

    fn detail_fixture() -> BalanceDetail {
        BalanceDetail {
            ccy: "USDT".to_owned(),
            eq: "100.5".to_owned(),
            cash_bal: "95".to_owned(),
            iso_eq: String::new(),
            avail_eq: "80.25".to_owned(),
            dis_eq: String::new(),
            fixed_bal: String::new(),
            avail_bal: "90".to_owned(),
            frozen_bal: String::new(),
            ord_frozen: String::new(),
            liab: String::new(),
            upl: String::new(),
            upl_liab: String::new(),
            cross_liab: String::new(),
            iso_liab: String::new(),
            reward_bal: String::new(),
            mgn_ratio: String::new(),
            imr: "20.25".to_owned(),
            mmr: String::new(),
            interest: String::new(),
            twap: String::new(),
            max_loan: String::new(),
            eq_usd: String::new(),
            borrow_froz: String::new(),
            notional_lever: String::new(),
            stgy_eq: String::new(),
            iso_upl: String::new(),
            spot_in_use_amt: String::new(),
            cl_spot_in_use_amt: String::new(),
            max_spot_in_use: String::new(),
            spot_iso_bal: String::new(),
            smt_sync_eq: String::new(),
            spot_copy_trading_eq: String::new(),
            spot_bal: String::new(),
            open_avg_px: String::new(),
            acc_avg_px: String::new(),
            spot_upl: String::new(),
            spot_upl_ratio: String::new(),
            total_pnl: String::new(),
            total_pnl_ratio: String::new(),
            collateral_enabled: false,
            collateral_restrict: false,
        }
    }
}
