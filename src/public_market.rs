#[cfg(feature = "binance-public-kline")]
mod binance;

#[cfg(feature = "binance-public-kline")]
pub use binance::{BinanceUsdmPublicKlineClient, BinanceUsdmPublicKlineConfig};
#[cfg(feature = "binance-public-kline")]
pub use binance_rs::Error as BinancePublicMarketSdkError;
#[cfg(feature = "binance-public-kline")]
pub use binance_rs::api::market::KlineRequest as BinanceUsdmPublicKlineQuery;
#[cfg(feature = "binance-public-kline")]
pub use binance_rs::client::{
    BinanceHttpEvidence, BinancePublicFailureKind, BinancePublicRequestFailure,
    BinancePublicResponse,
};
#[cfg(feature = "binance-public-kline")]
pub use binance_rs::dto::market::{BinanceUsdmKline, BinanceWireDecimal};

/// Binance 公共 Market 门面的返回类型。
#[cfg(feature = "binance-public-kline")]
pub type BinancePublicMarketResult<T> = std::result::Result<T, BinancePublicMarketSdkError>;

#[cfg(feature = "okx-public-market")]
use crate::error::{Error, Result};
#[cfg(feature = "okx-public-market")]
use crate::exchange::ExchangeId;
#[cfg(feature = "okx-public-market")]
use okx_rs::api::api_trait::OkxApiTrait;
#[cfg(feature = "okx-public-market")]
use okx_rs::dto::CandleOkxRespDto;
#[cfg(feature = "okx-public-market")]
use okx_rs::{OkxClient, OkxMarket};
#[cfg(feature = "okx-public-market")]
use serde::{Deserialize, Serialize};

/// OKX 公共 K 线 endpoint 的单页上限。
#[cfg(feature = "okx-public-market")]
pub const OKX_MAX_CANDLE_PAGE_SIZE: u32 = 300;

/// OKX 公共 K 线数据集。
#[cfg(feature = "okx-public-market")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OkxCandleDataset {
    /// 最近最多 1,440 根 K 线的数据集。
    Recent,
    /// 更长历史范围的数据集。
    History,
}

/// 不包含账户凭证的 OKX 公共行情客户端配置。
#[cfg(feature = "okx-public-market")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OkxPublicMarketConfig {
    /// 可选 API 基地址，主要用于部署路由或 contract test。
    pub api_url: Option<String>,
}

/// 由调用方显式选择时间边界的 OKX 公共 K 线查询。
#[cfg(feature = "okx-public-market")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkxPublicCandleQuery {
    instrument_id: String,
    interval: String,
    limit: Option<u32>,
    after: Option<String>,
    before: Option<String>,
}

#[cfg(feature = "okx-public-market")]
impl OkxPublicCandleQuery {
    /// 使用 OKX canonical `instId` 创建不含隐式时间边界的查询。
    ///
    /// SDK 不从 base/quote 反推交割合约名称，避免丢失到期日等 symbol 语义。
    pub fn new(instrument_id: impl Into<String>, interval: impl Into<String>) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            interval: interval.into(),
            limit: None,
            after: None,
            before: None,
        }
    }

    /// 设置 provider 单页数量。
    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }

    /// 设置 OKX `after` 边界；SDK 忠实传递，不改变方向。
    pub fn with_after(mut self, value: impl Into<String>) -> Self {
        self.after = Some(value.into());
        self
    }

    /// 设置 OKX `before` 边界；SDK 忠实传递，不改变方向。
    pub fn with_before(mut self, value: impl Into<String>) -> Self {
        self.before = Some(value.into());
        self
    }
}

/// SDK 对外暴露的无损 OKX K 线行。
///
/// 三个成交量字段保留交易所原始单位语义，由 Core exchange-gateway 按产品类型
/// 映射为领域 contracts/base/quote volume，避免 SDK 在缺少业务上下文时猜测。
#[cfg(feature = "okx-public-market")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OkxPublicCandle {
    /// K 线开始时间，Unix 毫秒文本。
    pub timestamp: String,
    /// 开盘价 Decimal 文本。
    pub open: String,
    /// 最高价 Decimal 文本。
    pub high: String,
    /// 最低价 Decimal 文本。
    pub low: String,
    /// 收盘价 Decimal 文本。
    pub close: String,
    /// OKX `vol` 字段。
    pub volume: String,
    /// OKX `volCcy` 字段。
    pub volume_currency: String,
    /// OKX `volCcyQuote` 字段。
    pub quote_volume: String,
    /// OKX `confirm` 字段。
    pub confirm: String,
}

/// 仅暴露 OKX 公共 Market API 的 SDK capability。
///
/// 该类型内部使用无 credential client，且没有账户、私有读取或 mutation 方法。
#[cfg(feature = "okx-public-market")]
#[derive(Debug)]
pub struct OkxPublicMarketClient {
    market: OkxMarket,
}

#[cfg(feature = "okx-public-market")]
impl OkxPublicMarketClient {
    /// 创建无账户凭证的公共行情客户端。
    pub fn new(config: OkxPublicMarketConfig) -> Result<Self> {
        let mut client = OkxClient::new_public().map_err(Error::from_okx)?;
        if let Some(api_url) = config.api_url {
            client.set_base_url(api_url);
        }

        Ok(Self {
            market: <OkxMarket as OkxApiTrait>::new(client),
        })
    }

    /// 按显式 recent/history 数据集读取一页 K 线。
    ///
    /// `after`/`before` 被原样交给 SDK endpoint；Core Adapter 必须先把领域时间
    /// 方向映射为 OKX 参数，SDK 不会交换或静默丢弃窗口边界。
    pub async fn candles(
        &self,
        dataset: OkxCandleDataset,
        query: OkxPublicCandleQuery,
    ) -> Result<Vec<OkxPublicCandle>> {
        validate_query(&query)?;
        let limit = query.limit.map(|value| value.to_string());
        let rows = match dataset {
            OkxCandleDataset::Recent => {
                self.market
                    .get_candles(
                        &query.instrument_id,
                        &query.interval,
                        query.after.as_deref(),
                        query.before.as_deref(),
                        limit.as_deref(),
                    )
                    .await
            }
            OkxCandleDataset::History => {
                self.market
                    .get_history_candles(
                        &query.instrument_id,
                        &query.interval,
                        query.after.as_deref(),
                        query.before.as_deref(),
                        limit.as_deref(),
                    )
                    .await
            }
        }
        .map_err(Error::from_okx)?;

        Ok(rows.into_iter().map(OkxPublicCandle::from).collect())
    }
}

/// 拒绝 SDK 当前无法无歧义表达的查询，避免字段存在但被静默忽略。
#[cfg(feature = "okx-public-market")]
fn validate_query(query: &OkxPublicCandleQuery) -> Result<()> {
    if query.instrument_id.is_empty()
        || !query
            .instrument_id
            .bytes()
            .all(|byte| byte.is_ascii_graphic())
    {
        return Err(Error::Adapter {
            exchange: ExchangeId::Okx,
            message: "OKX candle instId 必须是非空、无空白的可打印 ASCII".to_string(),
        });
    }
    match query.limit {
        Some(0) => {
            return Err(Error::Adapter {
                exchange: ExchangeId::Okx,
                message: "OKX candle limit 必须大于零".to_string(),
            });
        }
        Some(limit) if limit > OKX_MAX_CANDLE_PAGE_SIZE => {
            return Err(Error::Adapter {
                exchange: ExchangeId::Okx,
                message: format!("OKX candle limit {limit} 超过上限 {OKX_MAX_CANDLE_PAGE_SIZE}"),
            });
        }
        _ => {}
    }
    Ok(())
}

#[cfg(feature = "okx-public-market")]
impl From<CandleOkxRespDto> for OkxPublicCandle {
    /// 保留全部九个 provider 字段，不在 SDK facade 中压平成交量单位。
    fn from(value: CandleOkxRespDto) -> Self {
        Self {
            timestamp: value.ts,
            open: value.o,
            high: value.h,
            low: value.l,
            close: value.c,
            volume: value.v,
            volume_currency: value.vol_ccy,
            quote_volume: value.vol_ccy_quote,
            confirm: value.confirm,
        }
    }
}

#[cfg(all(test, feature = "okx-public-market"))]
mod tests {
    use super::*;

    /// SDK 层先拒绝超限页，不能让 provider 默认行为接管语义。
    #[test]
    fn oversized_page_is_rejected_before_network_io() {
        let query = OkxPublicCandleQuery::new("BTC-USDT-SWAP", "1m").with_limit(301);

        let error = validate_query(&query).expect_err("oversized page must fail");

        assert!(error.to_string().contains("超过上限 300"));
    }
}
