use crate::client::{push_optional_str, push_optional_u32, push_optional_u64};
use crate::{BybitClient, Error};
use serde_json::Value;

impl BybitClient {
    pub async fn ticker(&self, category: &str, symbol: &str) -> Result<Value, Error> {
        self.send_public(
            "/v5/market/tickers",
            &[
                ("category", category.to_string()),
                ("symbol", symbol.to_string()),
            ],
        )
        .await
    }

    pub async fn orderbook(
        &self,
        category: &str,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<Value, Error> {
        let mut params = vec![
            ("category", category.to_string()),
            ("symbol", symbol.to_string()),
        ];
        push_optional_u32(&mut params, "limit", limit);
        self.send_public("/v5/market/orderbook", &params).await
    }

    pub async fn kline(
        &self,
        category: &str,
        symbol: &str,
        interval: &str,
        limit: Option<u32>,
        start: Option<u64>,
        end: Option<u64>,
    ) -> Result<Value, Error> {
        let mut params = vec![
            ("category", category.to_string()),
            ("symbol", symbol.to_string()),
            ("interval", interval.to_string()),
        ];
        push_optional_u32(&mut params, "limit", limit);
        push_optional_u64(&mut params, "start", start);
        push_optional_u64(&mut params, "end", end);
        self.send_public("/v5/market/kline", &params).await
    }

    pub async fn instruments(&self, category: &str, symbol: Option<&str>) -> Result<Value, Error> {
        let mut params = vec![("category", category.to_string())];
        push_optional_str(&mut params, "symbol", symbol);
        self.send_public("/v5/market/instruments-info", &params)
            .await
    }

    pub async fn funding_rate_history(
        &self,
        category: &str,
        symbol: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
    ) -> Result<Value, Error> {
        let mut params = vec![
            ("category", category.to_string()),
            ("symbol", symbol.to_string()),
        ];
        push_optional_u64(&mut params, "startTime", start_time);
        push_optional_u64(&mut params, "endTime", end_time);
        push_optional_u32(&mut params, "limit", limit);
        self.send_public("/v5/market/funding/history", &params)
            .await
    }

    pub async fn open_interest(
        &self,
        category: &str,
        symbol: &str,
        interval_time: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<Value, Error> {
        let mut params = vec![
            ("category", category.to_string()),
            ("symbol", symbol.to_string()),
            ("intervalTime", interval_time.to_string()),
        ];
        push_optional_u64(&mut params, "startTime", start_time);
        push_optional_u64(&mut params, "endTime", end_time);
        push_optional_u32(&mut params, "limit", limit);
        push_optional_str(&mut params, "cursor", cursor);
        self.send_public("/v5/market/open-interest", &params).await
    }

    pub async fn long_short_ratio(
        &self,
        category: &str,
        symbol: &str,
        period: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<Value, Error> {
        let mut params = vec![
            ("category", category.to_string()),
            ("symbol", symbol.to_string()),
            ("period", period.to_string()),
        ];
        push_optional_u64(&mut params, "startTime", start_time);
        push_optional_u64(&mut params, "endTime", end_time);
        push_optional_u32(&mut params, "limit", limit);
        push_optional_str(&mut params, "cursor", cursor);
        self.send_public("/v5/market/account-ratio", &params).await
    }
}
