use crate::api::api_trait::BinanceApiTrait;
use crate::api::market::BinanceMarket;
use crate::client::BinancePublicResponse;
use crate::dto::market::BinanceUsdmMarkPrice;
use crate::error::Error;
use reqwest::Method;

const PREMIUM_INDEX_PATH: &str = "/fapi/v1/premiumIndex";

impl BinanceMarket {
    /// 匿名读取一个明确 symbol 的 USD-M 标记价并保留同次 HTTP 证据。
    pub async fn get_mark_price_typed(
        &self,
        symbol: &str,
    ) -> Result<BinancePublicResponse<BinanceUsdmMarkPrice>, Error> {
        if !is_graphic_ascii(symbol) {
            return Err(Error::InvalidRequest(
                "Binance mark-price symbol 必须是非空、无空白的可打印 ASCII".to_owned(),
            ));
        }
        self.client()
            .send_public_request_with_evidence(
                Method::GET,
                PREMIUM_INDEX_PATH,
                &[("symbol", symbol.to_owned())],
            )
            .await
    }
}

fn is_graphic_ascii(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_graphic())
}
