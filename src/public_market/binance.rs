use super::{
    BinancePublicMarketResult, BinancePublicMarketSdkError, BinancePublicResponse,
    BinanceUsdmKline, BinanceUsdmMarkPrice, BinanceUsdmPublicKlineQuery,
};
use binance_rs::{BinanceClient, BinanceMarket, BinancePublicTransportConfig};

/// 不包含账户凭证的 Binance USD-M 公共 K 线客户端配置。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BinanceUsdmPublicKlineConfig {
    /// 可选 API 基地址，用于受控代理路由与 contract test。
    pub api_url: Option<String>,
}

/// Binance USD-M 匿名标记价客户端复用同一组无凭证 transport 配置。
pub type BinanceUsdmPublicMarkPriceConfig = BinanceUsdmPublicKlineConfig;

/// 只暴露 Binance USD-M 公共 K 线查询的具体客户端。
///
/// 该类型不提供账户读取、签名请求、分页调度、重试或交易 mutation 方法。
#[derive(Clone)]
pub struct BinanceUsdmPublicKlineClient {
    /// 仅持有匿名 transport 的 provider market API，不具备签名能力。
    market: BinanceMarket,
}

/// 只暴露 Binance USD-M 匿名标记价读取的具体客户端。
#[derive(Clone)]
pub struct BinanceUsdmPublicMarkPriceClient {
    market: BinanceMarket,
}

impl BinanceUsdmPublicMarkPriceClient {
    pub fn new(config: BinanceUsdmPublicMarkPriceConfig) -> BinancePublicMarketResult<Self> {
        let transport = match config.api_url {
            Some(api_url) => BinancePublicTransportConfig {
                api_url,
                ..BinancePublicTransportConfig::default()
            },
            None => BinancePublicTransportConfig::default(),
        };
        Self::with_transport(transport)
    }

    pub fn with_transport(
        transport: BinancePublicTransportConfig,
    ) -> BinancePublicMarketResult<Self> {
        let client = BinanceClient::new_public_with_transport(transport)?;
        Ok(Self {
            market: BinanceMarket::new(client),
        })
    }

    /// 读取精确 symbol；provider 返回其他 identity 或空价格时拒绝。
    pub async fn mark_price(
        &self,
        symbol: &str,
    ) -> BinancePublicMarketResult<BinancePublicResponse<BinanceUsdmMarkPrice>> {
        let response = self.market.get_mark_price_typed(symbol).await?;
        let price_is_empty = match &response.data.mark_price {
            binance_rs::dto::market::BinanceWireDecimal::Text(value) => value.is_empty(),
            binance_rs::dto::market::BinanceWireDecimal::Number(_) => false,
        };
        if response.data.symbol != symbol || response.data.time == 0 || price_is_empty {
            return Err(BinancePublicMarketSdkError::InvalidRequest(
                "Binance mark-price identity、价格或时间戳不完整".to_owned(),
            ));
        }
        Ok(response)
    }
}

impl BinanceUsdmPublicKlineClient {
    /// 创建不持有 API Key/Secret 的 Binance USD-M 公共客户端。
    pub fn new(config: BinanceUsdmPublicKlineConfig) -> BinancePublicMarketResult<Self> {
        let transport = match config.api_url {
            Some(api_url) => BinancePublicTransportConfig {
                api_url,
                ..BinancePublicTransportConfig::default()
            },
            None => BinancePublicTransportConfig::default(),
        };
        Self::with_transport(transport)
    }

    /// 使用显式 endpoint、超时和代理创建无凭证公共客户端。
    pub fn with_transport(
        transport: BinancePublicTransportConfig,
    ) -> BinancePublicMarketResult<Self> {
        let client = BinanceClient::new_public_with_transport(transport)?;
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
