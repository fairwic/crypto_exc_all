use crate::client::{push_optional_str, push_optional_u32, push_optional_u64};
use crate::{Error, GateClient};
use serde_json::Value;

impl GateClient {
    pub async fn tickers(&self, settle: &str, contract: Option<&str>) -> Result<Value, Error> {
        let mut params = Vec::new();
        push_optional_str(&mut params, "contract", contract);
        self.send_public(&format!("/futures/{settle}/tickers"), &params)
            .await
    }

    pub async fn ticker(&self, settle: &str, contract: &str) -> Result<Value, Error> {
        self.tickers(settle, Some(contract)).await
    }

    pub async fn orderbook(
        &self,
        settle: &str,
        contract: &str,
        limit: Option<u32>,
    ) -> Result<Value, Error> {
        let mut params = vec![("contract", contract.to_string())];
        push_optional_u32(&mut params, "limit", limit);
        self.send_public(&format!("/futures/{settle}/order_book"), &params)
            .await
    }

    pub async fn candlesticks(
        &self,
        settle: &str,
        contract: &str,
        interval: &str,
        limit: Option<u32>,
        from: Option<u64>,
        to: Option<u64>,
    ) -> Result<Value, Error> {
        let mut params = vec![
            ("contract", contract.to_string()),
            ("interval", interval.to_string()),
        ];
        if from.is_none() && to.is_none() {
            push_optional_u32(&mut params, "limit", limit);
        }
        push_optional_u64(&mut params, "from", from);
        push_optional_u64(&mut params, "to", to);
        self.send_public(&format!("/futures/{settle}/candlesticks"), &params)
            .await
    }

    pub async fn contract(&self, settle: &str, contract: &str) -> Result<Value, Error> {
        self.send_public(&format!("/futures/{settle}/contracts/{contract}"), &[])
            .await
    }

    pub async fn funding_rate_history(
        &self,
        settle: &str,
        contract: &str,
        limit: Option<u32>,
    ) -> Result<Value, Error> {
        let mut params = vec![("contract", contract.to_string())];
        push_optional_u32(&mut params, "limit", limit);
        self.send_public(&format!("/futures/{settle}/funding_rate"), &params)
            .await
    }

    pub async fn insurance_history(
        &self,
        settle: &str,
        limit: Option<u32>,
    ) -> Result<Value, Error> {
        let mut params = Vec::new();
        push_optional_u32(&mut params, "limit", limit);
        self.send_public(&format!("/futures/{settle}/insurance"), &params)
            .await
    }
}
