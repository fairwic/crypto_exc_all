use crate::error::Error;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

pub fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis() as u64
}

pub fn generate_signature(secret: &str, payload: &str) -> Result<String, Error> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|err| Error::SignatureError(err.to_string()))?;
    mac.update(payload.as_bytes());
    Ok(STANDARD.encode(mac.finalize().into_bytes()))
}

pub fn build_query_string(params: &[(&str, String)]) -> String {
    let mut sorted = params.to_vec();
    sorted.sort_by(|left, right| left.0.cmp(right.0));
    sorted
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}
