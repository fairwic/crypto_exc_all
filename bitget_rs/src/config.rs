use crate::error::Error;
use std::env;
use std::path::PathBuf;
use std::sync::Once;

pub const DEFAULT_API_URL: &str = "https://api.bitget.com";
pub const DEFAULT_API_TIMEOUT_MS: u64 = 5_000;
pub const DEFAULT_WS_PUBLIC_URL: &str = "wss://ws.bitget.com/v2/ws/public";
pub const DEFAULT_WS_PRIVATE_URL: &str = "wss://ws.bitget.com/v2/ws/private";

static INIT_ENV: Once = Once::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub api_url: String,
    pub api_timeout_ms: u64,
    pub proxy_url: Option<String>,
    pub ws_public_url: String,
    pub ws_private_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_API_URL.to_string(),
            api_timeout_ms: DEFAULT_API_TIMEOUT_MS,
            proxy_url: None,
            ws_public_url: DEFAULT_WS_PUBLIC_URL.to_string(),
            ws_private_url: DEFAULT_WS_PRIVATE_URL.to_string(),
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        init_env();
        Self::from_lookup(|key| env::var(key).ok())
    }

    pub fn from_lookup<F>(lookup: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut config = Self::default();
        if let Some(api_url) = env_any_with(&lookup, &["BITGET_API_URL", "bitget_api_url"]) {
            config.api_url = api_url;
        }
        if let Some(ws_public_url) =
            env_any_with(&lookup, &["BITGET_WS_PUBLIC_URL", "bitget_ws_public_url"])
        {
            config.ws_public_url = ws_public_url;
        }
        if let Some(ws_private_url) =
            env_any_with(&lookup, &["BITGET_WS_PRIVATE_URL", "bitget_ws_private_url"])
        {
            config.ws_private_url = ws_private_url;
        }
        if let Some(timeout) =
            env_any_with(&lookup, &["BITGET_API_TIMEOUT_MS", "bitget_api_timeout_ms"])
                .and_then(|value| value.parse::<u64>().ok())
        {
            config.api_timeout_ms = timeout;
        }
        config.proxy_url = env_any_with(
            &lookup,
            &[
                "BITGET_PROXY_URL",
                "bitget_proxy_url",
                "ALL_PROXY",
                "all_proxy",
                "HTTPS_PROXY",
                "https_proxy",
            ],
        )
        .and_then(normalize_proxy_url);

        config
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    pub api_key: String,
    pub api_secret: String,
    pub passphrase: String,
}

impl Credentials {
    pub fn new(
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
        passphrase: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
            passphrase: passphrase.into(),
        }
    }

    pub fn from_env() -> Result<Self, Error> {
        init_env();

        Self::from_lookup(|key| env::var(key).ok())
    }

    pub fn from_lookup<F>(lookup: F) -> Result<Self, Error>
    where
        F: Fn(&str) -> Option<String>,
    {
        let api_key = env_any_with(&lookup, &["BITGET_API_KEY", "bitget_api_key"])
            .ok_or_else(|| Error::ConfigError("缺少环境变量: BITGET_API_KEY".to_string()))?;
        let api_secret = env_any_with(&lookup, &["BITGET_API_SECRET", "bitget_api_secret"])
            .ok_or_else(|| Error::ConfigError("缺少环境变量: BITGET_API_SECRET".to_string()))?;
        let passphrase = env_any_with(
            &lookup,
            &[
                "BITGET_PASSPHRASE",
                "BITGET_API_PASSPHRASE",
                "bitget_PASSPHRASE",
                "bitget_passphrase",
                "bitget_api_passphrase",
            ],
        )
        .ok_or_else(|| Error::ConfigError("缺少环境变量: BITGET_PASSPHRASE".to_string()))?;

        Ok(Self::new(api_key, api_secret, passphrase))
    }
}

pub fn init_env() {
    INIT_ENV.call_once(|| {
        if dotenv::dotenv().is_ok() {
            return;
        }

        for candidate in env_file_candidates() {
            if candidate.exists() {
                let _ = dotenv::from_path(candidate);
                break;
            }
        }
    });
}

fn env_any_with<F>(lookup: &F, names: &[&str]) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    names.iter().find_map(|name| lookup(name))
}

fn normalize_proxy_url(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("socks5://") {
        return Some(format!("socks5h://{rest}"));
    }

    Some(trimmed.to_string())
}

fn env_file_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(mut dir) = env::current_dir() {
        loop {
            candidates.push(dir.join(".env"));
            if !dir.pop() {
                break;
            }
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_reads_proxy_and_normalizes_socks5() {
        let config = Config::from_lookup(|key| match key {
            "BITGET_PROXY_URL" => Some("socks5://127.0.0.1:7897".to_string()),
            _ => None,
        });

        assert_eq!(
            config.proxy_url,
            Some("socks5h://127.0.0.1:7897".to_string())
        );
    }

    #[test]
    fn credentials_accept_existing_mixed_case_passphrase_name() {
        let credentials = Credentials::from_lookup(|key| match key {
            "bitget_api_key" => Some("key".to_string()),
            "bitget_api_secret" => Some("secret".to_string()),
            "bitget_PASSPHRASE" => Some("pass".to_string()),
            _ => None,
        })
        .unwrap();

        assert_eq!(credentials.passphrase, "pass");
    }
}
