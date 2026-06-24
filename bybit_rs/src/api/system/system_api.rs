use crate::client::push_optional_str;
use crate::{BybitClient, Error};
use serde_json::Value;

impl BybitClient {
    pub async fn system_status(
        &self,
        id: Option<&str>,
        state: Option<&str>,
    ) -> Result<Value, Error> {
        let mut params = Vec::new();
        push_optional_str(&mut params, "id", id);
        push_optional_str(&mut params, "state", state);
        self.send_public("/v5/system/status", &params).await
    }
}
