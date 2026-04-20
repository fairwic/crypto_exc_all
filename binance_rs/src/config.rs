use crate::error::Error;
use std::env;
use std::path::PathBuf;
use std::sync::Once;

pub const DEFAULT_API_URL: &str = "https://fapi.binance.com";
pub const DEFAULT_SAPI_API_URL: &str = "https://api.binance.com";
pub const DEFAULT_WEB_API_URL: &str = "https://www.binance.com";
pub const DEFAULT_WS_STREAM_URL: &str = "wss://fstream.binance.com";
pub const DEFAULT_RECV_WINDOW_MS: u64 = 5_000;
pub const DEFAULT_API_TIMEOUT_MS: u64 = 5_000;

static INIT_ENV: Once = Once::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub api_url: String,
    pub sapi_api_url: String,
    pub web_api_url: String,
    pub ws_stream_url: String,
    pub api_timeout_ms: u64,
    pub recv_window_ms: u64,
    pub proxy_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_API_URL.to_string(),
            sapi_api_url: DEFAULT_SAPI_API_URL.to_string(),
            web_api_url: DEFAULT_WEB_API_URL.to_string(),
            ws_stream_url: DEFAULT_WS_STREAM_URL.to_string(),
            api_timeout_ms: DEFAULT_API_TIMEOUT_MS,
            recv_window_ms: DEFAULT_RECV_WINDOW_MS,
            proxy_url: None,
        }
    }
}

impl Config {
    pub fn from_env() -> Self {
        init_env();

        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup<F>(lookup: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut config = Self::default();
        if let Some(api_url) = env_any_with(&lookup, &["BINANCE_API_URL", "binance_api_url"]) {
            config.api_url = api_url;
        }
        if let Some(sapi_api_url) =
            env_any_with(&lookup, &["BINANCE_SAPI_API_URL", "binance_sapi_api_url"])
        {
            config.sapi_api_url = sapi_api_url;
        }
        if let Some(web_api_url) =
            env_any_with(&lookup, &["BINANCE_WEB_API_URL", "binance_web_api_url"])
        {
            config.web_api_url = web_api_url;
        }
        if let Some(ws_stream_url) =
            env_any_with(&lookup, &["BINANCE_WS_STREAM_URL", "binance_ws_stream_url"])
        {
            config.ws_stream_url = ws_stream_url;
        }
        if let Some(timeout) = env_any_with(
            &lookup,
            &["BINANCE_API_TIMEOUT_MS", "binance_api_timeout_ms"],
        )
        .and_then(|value| value.parse::<u64>().ok())
        {
            config.api_timeout_ms = timeout;
        }
        if let Some(recv_window) = env_any_with(
            &lookup,
            &["BINANCE_RECV_WINDOW_MS", "binance_recv_window_ms"],
        )
        .and_then(|value| value.parse::<u64>().ok())
        {
            config.recv_window_ms = recv_window;
        }
        config.proxy_url = env_any_with(
            &lookup,
            &[
                "BINANCE_PROXY_URL",
                "binance_proxy_url",
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
}

impl Credentials {
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
        }
    }

    pub fn from_env() -> Result<Self, Error> {
        init_env();

        let api_key = env_any(&["BINANCE_API_KEY", "binance_api_key"])
            .ok_or_else(|| Error::ConfigError("缺少环境变量: BINANCE_API_KEY".to_string()))?;
        let api_secret = env_any(&["BINANCE_API_SECRET", "binance_api_secret"])
            .ok_or_else(|| Error::ConfigError("缺少环境变量: BINANCE_API_SECRET".to_string()))?;

        Ok(Self::new(api_key, api_secret))
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

fn env_any(names: &[&str]) -> Option<String> {
    env_any_with(&|name| env::var(name).ok(), names)
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
    fn config_reads_explicit_proxy_before_global_proxy() {
        let config = Config::from_lookup(|key| match key {
            "BINANCE_PROXY_URL" => Some("socks5://127.0.0.1:7897".to_string()),
            "ALL_PROXY" => Some("http://proxy.example:8080".to_string()),
            _ => None,
        });

        assert_eq!(
            config.proxy_url,
            Some("socks5h://127.0.0.1:7897".to_string())
        );
    }

    #[test]
    fn config_falls_back_to_global_proxy() {
        let config = Config::from_lookup(|key| match key {
            "all_proxy" => Some("socks5://127.0.0.1:7897".to_string()),
            _ => None,
        });

        assert_eq!(
            config.proxy_url,
            Some("socks5h://127.0.0.1:7897".to_string())
        );
    }
}
