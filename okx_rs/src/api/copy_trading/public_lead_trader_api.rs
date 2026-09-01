use crate::{
    client::{OkxClient, OkxPublicResponse},
    dto::copy_trading::OkxPublicLeadTraderPage,
    error::Error,
    public_transport::OkxPublicTransportConfig,
};
use reqwest::Method;

const PUBLIC_LEAD_TRADERS_PATH: &str = "/api/v5/copytrading/public-lead-traders";

/// OKX 匿名带单交易员榜单 capability。
///
/// 该接口只返回公开目录与榜单指标，不持有账户凭证，也不能证明某个来源账户已实际跟单。
#[derive(Debug, Clone)]
pub struct OkxPublicLeadTraders {
    client: OkxClient,
}

impl OkxPublicLeadTraders {
    pub fn new() -> Result<Self, Error> {
        Self::with_transport(OkxPublicTransportConfig::default())
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self, Error> {
        Self::with_transport(OkxPublicTransportConfig {
            api_url: base_url.into(),
            ..OkxPublicTransportConfig::default()
        })
    }

    pub fn with_transport(transport: OkxPublicTransportConfig) -> Result<Self, Error> {
        Ok(Self {
            client: OkxClient::new_public_with_transport(transport)?,
        })
    }

    /// 读取 SWAP 带单交易员的官方综合榜单。
    ///
    /// SDK 固定公开产品范围和排序语义；Web 等业务 owner 决定缓存、筛选、展示权利与订阅资格。
    pub async fn list_swap_overview(
        &self,
        page: u32,
        limit: u8,
    ) -> Result<OkxPublicResponse<Vec<OkxPublicLeadTraderPage>>, Error> {
        if page == 0 {
            return Err(Error::ConfigError(
                "OKX lead trader ranks page 必须大于 0".to_string(),
            ));
        }
        if !(1..=20).contains(&limit) {
            return Err(Error::ConfigError(
                "OKX lead trader ranks limit 必须在 1..=20".to_string(),
            ));
        }
        let path = format!(
            "{PUBLIC_LEAD_TRADERS_PATH}?instType=SWAP&sortType=overview&state=0&page={page}&limit={limit}"
        );
        self.client
            .send_public_request_with_evidence(Method::GET, &path, "")
            .await
    }
}
