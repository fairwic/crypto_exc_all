use crate::api::api_trait::BinanceApiTrait;
use crate::api::market::{BinanceMarket, KlineRequest};
use crate::client::BinancePublicResponse;
use crate::dto::market::BinanceUsdmKline;
use crate::error::Error;
use reqwest::Method;

const KLINES_PATH: &str = "/fapi/v1/klines";

impl BinanceMarket {
    /// 获取一页 Binance USD-M K 线并保留 HTTP/provider 证据。
    ///
    /// 这里只验证 endpoint 能无歧义表达的基本参数，不硬编码文档存在分歧的最大
    /// `limit`，也不负责 pagination、finality、业务完整性或恢复。
    pub async fn get_klines_typed(
        &self,
        request: KlineRequest,
    ) -> Result<BinancePublicResponse<Vec<BinanceUsdmKline>>, Error> {
        validate_kline_request(&request)?;
        self.client()
            .send_public_request_with_evidence(Method::GET, KLINES_PATH, &request.to_params())
            .await
    }
}

/// 在发起网络请求前拒绝空白 identity 与无意义页长，避免 provider 默认值改变语义。
fn validate_kline_request(request: &KlineRequest) -> Result<(), Error> {
    if !is_graphic_ascii(&request.symbol) {
        return Err(Error::InvalidRequest(
            "Binance Kline symbol 必须是非空、无空白的可打印 ASCII".to_owned(),
        ));
    }
    if !is_graphic_ascii(&request.interval) {
        return Err(Error::InvalidRequest(
            "Binance Kline interval 必须是非空、无空白的可打印 ASCII".to_owned(),
        ));
    }
    if request.limit == Some(0) {
        return Err(Error::InvalidRequest(
            "Binance Kline limit 必须大于零".to_owned(),
        ));
    }
    Ok(())
}

/// 只接受 URL query 中无需额外转义空白语义的 provider identity。
fn is_graphic_ascii(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_graphic())
}
