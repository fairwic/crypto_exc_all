use super::{
    BinancePublicMarketResult, BinancePublicResponse, BinanceUsdmKline, BinanceUsdmPublicKlineQuery,
};
use binance_rs::{BinanceClient, BinanceMarket};

/// 不包含账户凭证的 Binance USD-M 公共 K 线客户端配置。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BinanceUsdmPublicKlineConfig {
    /// 可选 API 基地址，用于受控代理路由与 contract test。
    pub api_url: Option<String>,
}

/// 只暴露 Binance USD-M 公共 K 线查询的具体客户端。
///
/// 该类型不提供账户读取、签名请求、分页调度、重试或交易 mutation 方法。
#[derive(Clone)]
pub struct BinanceUsdmPublicKlineClient {
    /// 仅持有匿名 transport 的 provider market API，不具备签名能力。
    market: BinanceMarket,
}

impl BinanceUsdmPublicKlineClient {
    /// 创建不持有 API Key/Secret 的 Binance USD-M 公共客户端。
    pub fn new(config: BinanceUsdmPublicKlineConfig) -> BinancePublicMarketResult<Self> {
        let mut client = BinanceClient::new_public()?;
        if let Some(api_url) = config.api_url {
            client.set_base_url(api_url);
        }

        Ok(Self {
            market: BinanceMarket::new(client),
        })
    }

    /// 获取一页 Binance USD-M K 线 wire rows 与同次 HTTP 证据。
    ///
    /// SDK 忠实传递调用方窗口，不排序、不分页，也不根据 close time 推断 finality；
    /// 这些语义由 Market owner 在消费 typed response 时统一处理。
    pub async fn klines(
        &self,
        query: BinanceUsdmPublicKlineQuery,
    ) -> BinancePublicMarketResult<BinancePublicResponse<Vec<BinanceUsdmKline>>> {
        self.market.get_klines_typed(query).await
    }
}
