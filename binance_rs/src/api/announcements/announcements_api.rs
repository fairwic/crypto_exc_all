use crate::api::api_trait::BinanceApiTrait;
use crate::client::BinanceClient;
use crate::config::Config;
use crate::error::Error;
use reqwest::Method;

const ANNOUNCEMENT_LIST_PATH: &str = "/bapi/composite/v1/public/cms/article/list/query";

#[derive(Clone)]
pub struct BinanceAnnouncements {
    client: BinanceClient,
}

impl BinanceApiTrait for BinanceAnnouncements {
    fn new(client: BinanceClient) -> Self {
        Self { client }
    }

    fn from_env() -> Result<Self, Error> {
        let mut config = Config::from_env();
        config.api_url = config.web_api_url.clone();
        let client = BinanceClient::with_config(None, config)?;
        Ok(Self::new(client))
    }

    fn client(&self) -> &BinanceClient {
        &self.client
    }
}

impl BinanceAnnouncements {
    pub fn new(client: BinanceClient) -> Self {
        <Self as BinanceApiTrait>::new(client)
    }

    pub fn from_env() -> Result<Self, Error> {
        <Self as BinanceApiTrait>::from_env()
    }

    pub async fn get_announcements(
        &self,
        request: AnnouncementListRequest,
    ) -> Result<serde_json::Value, Error> {
        self.client
            .send_public_request(Method::GET, ANNOUNCEMENT_LIST_PATH, &request.to_params())
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnouncementListRequest {
    pub article_type: Option<u32>,
    pub catalog_id: Option<u64>,
    pub page_no: Option<u32>,
    pub page_size: Option<u32>,
}

impl AnnouncementListRequest {
    pub fn new() -> Self {
        Self {
            article_type: Some(1),
            catalog_id: None,
            page_no: Some(1),
            page_size: Some(20),
        }
    }

    pub fn latest() -> Self {
        Self::new().with_catalog_id(48)
    }

    pub fn with_article_type(mut self, article_type: u32) -> Self {
        self.article_type = Some(article_type);
        self
    }

    pub fn with_catalog_id(mut self, catalog_id: u64) -> Self {
        self.catalog_id = Some(catalog_id);
        self
    }

    pub fn with_page(mut self, page_no: u32) -> Self {
        self.page_no = Some(page_no);
        self
    }

    pub fn with_page_size(mut self, page_size: u32) -> Self {
        self.page_size = Some(page_size);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();
        push_optional(&mut params, "type", self.article_type);
        push_optional(&mut params, "catalogId", self.catalog_id);
        push_optional(&mut params, "pageNo", self.page_no);
        push_optional(&mut params, "pageSize", self.page_size);
        params
    }
}

impl Default for AnnouncementListRequest {
    fn default() -> Self {
        Self::new()
    }
}

fn push_optional<T: ToString>(
    params: &mut Vec<(&'static str, String)>,
    key: &'static str,
    value: Option<T>,
) {
    if let Some(value) = value {
        params.push((key, value.to_string()));
    }
}
