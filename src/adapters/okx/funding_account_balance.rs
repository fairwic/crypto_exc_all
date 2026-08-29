use super::*;
use crate::account::FundingAccountBalance;
use okx_rs::dto::asset::asset_dto::AssetBalance as OkxAssetBalance;

impl OkxAdapter {
    /// 读取 OKX 资金账户余额，不与 `/account/balance` 的交易保证金语义合并。
    pub(crate) async fn funding_account_balances(
        &self,
        asset: Option<&str>,
    ) -> Result<Vec<FundingAccountBalance>> {
        let currencies = asset.map(|asset| vec![asset.to_ascii_uppercase()]);
        self.asset
            .get_balances(currencies.as_ref())
            .await
            .map_err(Error::from_okx)?
            .into_iter()
            .map(map_funding_account_balance)
            .collect()
    }
}

fn map_funding_account_balance(balance: OkxAssetBalance) -> Result<FundingAccountBalance> {
    let raw = serde_json::to_value(&balance)?;
    Ok(FundingAccountBalance {
        exchange: ExchangeId::Okx,
        asset: balance.ccy,
        total: balance.bal,
        available: balance.avail_bal,
        frozen: non_empty(balance.frozen_bal),
        raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_okx_funding_balance_without_turning_it_into_trading_margin() {
        let balance = map_funding_account_balance(OkxAssetBalance {
            ccy: "USDT".to_owned(),
            bal: "12.5".to_owned(),
            frozen_bal: "0.5".to_owned(),
            avail_bal: "12".to_owned(),
        })
        .expect("funding balance");

        assert_eq!(balance.exchange, ExchangeId::Okx);
        assert_eq!(balance.asset, "USDT");
        assert_eq!(balance.total, "12.5");
        assert_eq!(balance.available, "12");
        assert_eq!(balance.frozen.as_deref(), Some("0.5"));
    }
}
