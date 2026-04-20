use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Balance {
    pub exchange: ExchangeId,
    pub asset: String,
    pub total: String,
    pub available: String,
    pub frozen: Option<String>,
    pub raw: Value,
}

pub struct AccountFacade<'a> {
    pub(crate) client: &'a ExchangeClient,
}

impl<'a> AccountFacade<'a> {
    pub(crate) fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    pub async fn balances(&self) -> Result<Vec<Balance>> {
        self.client.balances().await
    }
}
