use crate::client::{OkxClient, OkxPublicResponse};
use crate::dto::public_data::OkxPublicInstrument;
use crate::error::Error;
use reqwest::Method;

const OKX_SWAP_INSTRUMENTS_PATH: &str = "/api/v5/public/instruments?instType=SWAP";

/// 只暴露 OKX 公共 `SWAP` 产品规格的无凭证 capability。
///
/// 查询范围固定在 endpoint 路径中，调用方不能把 Market 的币种、状态或完整性策略
/// 下推到 SDK，从而保证 provider 快照语义可审计且不依赖用户账户凭证。
#[derive(Debug, Clone)]
pub struct OkxPublicInstruments {
    client: OkxClient,
}

impl OkxPublicInstruments {
    /// 使用 `OkxClient::new_public` 创建不持有 API Key、签名或 passphrase 的客户端。
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            client: OkxClient::new_public()?,
        })
    }

    /// 创建仍然无凭证、但使用指定基地址的客户端。
    ///
    /// 该入口用于受控代理与 contract test；它只替换 origin，不允许改变固定的 SWAP query。
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self, Error> {
        let mut client = OkxClient::new_public()?;
        client.set_base_url(base_url);
        Ok(Self { client })
    }

    /// 获取完整 OKX `SWAP` 产品规格响应及本次 HTTP/provider 限频证据。
    ///
    /// SDK 不重试、不筛选空集合，也不把十进制文本转换成业务 Decimal；恢复与完整性判断归
    /// 上层 Market owner。
    pub async fn list_swap(&self) -> Result<OkxPublicResponse<Vec<OkxPublicInstrument>>, Error> {
        self.client
            .send_public_request_with_evidence(Method::GET, OKX_SWAP_INSTRUMENTS_PATH, "")
            .await
    }
}
