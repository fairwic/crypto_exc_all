use super::{BinanceExchangeInfo, BinancePublicResponse, PublicInstrumentResult};
use binance_rs::{BinanceClient, BinanceMarket};

/// Binance USDⓈ-M `exchangeInfo` 官方声明的单次 IP 权重。
///
/// 该常量只暴露协议预算，不在 SDK 内实现配额累计、等待或重试。
pub const BINANCE_USDM_EXCHANGE_INFO_IP_WEIGHT: u32 = 1;

/// 不包含账户凭证的 Binance USDⓈ-M instrument client 配置。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BinanceUsdmPublicInstrumentConfig {
    /// 可选 API 基地址，用于受控代理路由与 contract test。
    pub api_url: Option<String>,
}

/// 只暴露 Binance USDⓈ-M 公共 instrument 查询的具体客户端。
///
/// 该类型不提供账户读取、签名请求或交易 mutation 方法。
#[derive(Clone)]
pub struct BinanceUsdmPublicInstrumentClient {
    market: BinanceMarket,
}

impl BinanceUsdmPublicInstrumentClient {
    /// 创建不持有 API Key/Secret 的 Binance USDⓈ-M 公共客户端。
    pub fn new(config: BinanceUsdmPublicInstrumentConfig) -> PublicInstrumentResult<Self> {
        let mut client = BinanceClient::new_public()?;
        if let Some(api_url) = config.api_url {
            client.set_base_url(api_url);
        }

        Ok(Self {
            market: BinanceMarket::new(client),
        })
    }

    /// 获取 Binance USDⓈ-M 全量 instrument wire response 与同次 HTTP 证据。
    ///
    /// Endpoint 固定无 query；空集合、生命周期状态和业务完整性由 Market owner
    /// 在消费该响应时判断。
    pub async fn exchange_info(
        &self,
    ) -> PublicInstrumentResult<BinancePublicResponse<BinanceExchangeInfo>> {
        self.market
            .get_exchange_info_typed()
            .await
            .map_err(Into::into)
    }
}
