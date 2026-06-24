use crate::api::API_PUBLIC_PATH;
use crate::client::BitgetClient;
use crate::error::Error;
use reqwest::Method;
use serde_json::Value;

#[derive(Clone)]
pub struct BitgetAnnouncements {
    client: BitgetClient,
}

impl BitgetAnnouncements {
    pub fn new(client: BitgetClient) -> Self {
        Self { client }
    }

    pub fn new_public() -> Result<Self, Error> {
        Ok(Self::new(BitgetClient::new_public()?))
    }

    pub async fn get_announcements(
        &self,
        request: AnnouncementListRequest,
    ) -> Result<Value, Error> {
        let path = format!("{API_PUBLIC_PATH}/annoucements");
        self.client
            .send_public_request(Method::GET, &path, &request.to_params())
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnouncementListRequest {
    pub language: String,
    pub ann_type: Option<String>,
    pub ann_sub_type: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

impl AnnouncementListRequest {
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            ann_type: None,
            ann_sub_type: None,
            cursor: None,
            limit: None,
        }
    }

    pub fn with_ann_type(mut self, value: impl Into<String>) -> Self {
        self.ann_type = Some(value.into());
        self
    }

    pub fn with_ann_sub_type(mut self, value: impl Into<String>) -> Self {
        self.ann_sub_type = Some(value.into());
        self
    }

    pub fn with_cursor(mut self, value: impl Into<String>) -> Self {
        self.cursor = Some(value.into());
        self
    }

    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![("language", self.language.clone())];
        push_opt_string(&mut params, "annType", self.ann_type.clone());
        push_opt_string(&mut params, "annSubType", self.ann_sub_type.clone());
        push_opt_string(&mut params, "cursor", self.cursor.clone());
        push_opt_string(
            &mut params,
            "limit",
            self.limit.map(|value| value.to_string()),
        );
        params
    }
}

fn push_opt_string(
    params: &mut Vec<(&'static str, String)>,
    key: &'static str,
    value: Option<String>,
) {
    if let Some(value) = value {
        params.push((key, value));
    }
}
