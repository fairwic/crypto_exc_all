use crate::api::api_trait::BinanceApiTrait;
use crate::api::market::BinanceMarket;
use crate::client::BinancePublicResponse;
use crate::dto::market::BinanceExchangeInfo;
use crate::error::Error;
use reqwest::Method;

const EXCHANGE_INFO_PATH: &str = "/fapi/v1/exchangeInfo";

impl BinanceMarket {
    /// 获取 Binance USDⓈ-M 全量交易规则及合约元数据。
    ///
    /// 该 endpoint 官方不提供分页或 symbol query，因此这里固定无查询参数，
    /// 防止调用方误把局部响应当成完整 instrument collection。返回值只表达
    /// Binance wire 数据与 HTTP 证据，不判断永续合约、可交易状态或 Market readiness。
    pub async fn get_exchange_info_typed(
        &self,
    ) -> Result<BinancePublicResponse<BinanceExchangeInfo>, Error> {
        self.client()
            .send_public_request_with_evidence(Method::GET, EXCHANGE_INFO_PATH, &[])
            .await
    }
}
