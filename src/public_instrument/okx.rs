use super::{OkxPublicInstrument, OkxPublicResponse, PublicInstrumentResult};
use okx_rs::OkxPublicInstruments;

/// OKX public instruments endpoint 每个限频窗口允许的请求数。
pub const OKX_SWAP_INSTRUMENT_RATE_LIMIT: u32 = 20;

/// OKX public instruments endpoint 的限频窗口，单位为毫秒。
pub const OKX_SWAP_INSTRUMENT_RATE_WINDOW_MS: u64 = 2_000;

/// 不包含账户凭证的 OKX SWAP instrument client 配置。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OkxSwapPublicInstrumentConfig {
    /// 可选 API 基地址，用于受控代理路由与 contract test。
    pub api_url: Option<String>,
}

/// 只暴露 OKX 公共 `SWAP` instrument 查询的具体客户端。
///
/// 查询范围固定为 `instType=SWAP`，不会接收调用方下推的币种、状态或账户参数。
#[derive(Debug, Clone)]
pub struct OkxSwapPublicInstrumentClient {
    instruments: OkxPublicInstruments,
}

impl OkxSwapPublicInstrumentClient {
    /// 创建不持有 API Key、passphrase 或签名能力的 OKX 公共客户端。
    pub fn new(config: OkxSwapPublicInstrumentConfig) -> PublicInstrumentResult<Self> {
        let instruments = match config.api_url {
            Some(api_url) => OkxPublicInstruments::with_base_url(api_url)?,
            None => OkxPublicInstruments::new()?,
        };
        Ok(Self { instruments })
    }

    /// 获取 OKX `SWAP` instrument wire response 与同次 HTTP/provider 证据。
    ///
    /// SDK 不筛选结算币、线性合约或 lifecycle state，也不把空列表推断为完整快照。
    pub async fn instruments(
        &self,
    ) -> PublicInstrumentResult<OkxPublicResponse<Vec<OkxPublicInstrument>>> {
        self.instruments.list_swap().await.map_err(Into::into)
    }
}
