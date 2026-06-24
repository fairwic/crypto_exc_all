use crate::client::{push_optional_str, push_optional_u32};
use crate::{BybitClient, Error};
use serde_json::Value;

impl BybitClient {
    pub async fn announcements(
        &self,
        locale: &str,
        announcement_type: Option<&str>,
        tag: Option<&str>,
        page: Option<u32>,
        limit: Option<u32>,
    ) -> Result<Value, Error> {
        let mut params = vec![("locale", locale.to_string())];
        push_optional_str(&mut params, "type", announcement_type);
        push_optional_str(&mut params, "tag", tag);
        push_optional_u32(&mut params, "page", page);
        push_optional_u32(&mut params, "limit", limit);
        self.send_public("/v5/announcements/index", &params).await
    }
}
