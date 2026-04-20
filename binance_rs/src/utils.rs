use crate::error::Error;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use url::form_urlencoded::Serializer;

type HmacSha256 = Hmac<Sha256>;

pub fn generate_signature(secret: &str, payload: &str) -> Result<String, Error> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|err| Error::SignatureError(err.to_string()))?;
    mac.update(payload.as_bytes());
    let result = mac.finalize().into_bytes();
    Ok(result.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn build_query_string<K, V>(params: &[(K, V)]) -> String
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    let mut serializer = Serializer::new(String::new());
    for (key, value) in params {
        serializer.append_pair(key.as_ref(), value.as_ref());
    }
    serializer.finish()
}

pub fn current_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
