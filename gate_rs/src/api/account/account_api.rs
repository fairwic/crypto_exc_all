use crate::client::{push_optional_str, push_optional_u32, push_optional_u64};
use crate::{Error, GateAccountBookRequest, GateClient};
use serde_json::Value;

impl GateClient {
    pub async fn account_book(
        &self,
        settle: &str,
        request: GateAccountBookRequest,
    ) -> Result<Value, Error> {
        let mut params = Vec::new();
        push_optional_u64(&mut params, "from", request.from);
        push_optional_u32(&mut params, "limit", request.limit);
        push_optional_u64(&mut params, "to", request.to);
        push_optional_str(&mut params, "type", request.book_type.as_deref());
        self.send_signed_get(&format!("/futures/{settle}/account_book"), &params)
            .await
    }
}
