use crate::exchange::ExchangeId;
use std::env;
use std::path::PathBuf;
use std::sync::Once;

static INIT_ENV: Once = Once::new();

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SdkConfig {
    pub okx: Option<OkxExchangeConfig>,
    pub binance: Option<BinanceExchangeConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkxExchangeConfig {
    pub api_key: String,
    pub api_secret: String,
    pub passphrase: String,
    pub simulated: bool,
    pub api_url: Option<String>,
    pub request_expiration_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinanceExchangeConfig {
    pub api_key: String,
    pub api_secret: String,
    pub api_url: Option<String>,
    pub sapi_api_url: Option<String>,
    pub web_api_url: Option<String>,
    pub ws_stream_url: Option<String>,
    pub api_timeout_ms: Option<u64>,
    pub recv_window_ms: Option<u64>,
    pub proxy_url: Option<String>,
}

impl SdkConfig {
    pub fn from_env() -> Self {
        init_env();
        Self::from_lookup(|key| env::var(key).ok())
    }

    pub fn from_lookup<F>(lookup: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        Self {
            okx: read_okx_config(&lookup),
            binance: read_binance_config(&lookup),
        }
    }

    pub fn configured_exchanges(&self) -> Vec<ExchangeId> {
        let mut exchanges = Vec::new();
        if self.okx.is_some() {
            exchanges.push(ExchangeId::Okx);
        }
        if self.binance.is_some() {
            exchanges.push(ExchangeId::Binance);
        }
        exchanges
    }
}

fn read_okx_config<F>(lookup: &F) -> Option<OkxExchangeConfig>
where
    F: Fn(&str) -> Option<String>,
{
    let simulated_credentials = env_any_with(
        lookup,
        &[
            "OKX_SIMULATED_API_KEY",
            "OKX_SIMULATED_API_SECRET",
            "OKX_SIMULATED_PASSPHRASE",
        ],
    );
    let use_simulated_credentials = simulated_credentials.is_some()
        && env_any_with(lookup, &["OKX_API_KEY", "OKX_API_SECRET", "OKX_PASSPHRASE"]).is_none();

    let (api_key, api_secret, passphrase, default_simulated) = if use_simulated_credentials {
        (
            lookup("OKX_SIMULATED_API_KEY")?,
            lookup("OKX_SIMULATED_API_SECRET")?,
            lookup("OKX_SIMULATED_PASSPHRASE")?,
            true,
        )
    } else {
        (
            lookup("OKX_API_KEY")?,
            lookup("OKX_API_SECRET")?,
            lookup("OKX_PASSPHRASE")?,
            false,
        )
    };

    let simulated = lookup("OKX_SIMULATED_TRADING")
        .map(|value| parse_boolish(&value))
        .unwrap_or(default_simulated);

    Some(OkxExchangeConfig {
        api_key,
        api_secret,
        passphrase,
        simulated,
        api_url: lookup("OKX_API_URL"),
        request_expiration_ms: lookup("OKX_REQUEST_EXPIRATION_MS")
            .and_then(|value| value.parse::<i64>().ok()),
    })
}

fn read_binance_config<F>(lookup: &F) -> Option<BinanceExchangeConfig>
where
    F: Fn(&str) -> Option<String>,
{
    Some(BinanceExchangeConfig {
        api_key: env_any_with(lookup, &["BINANCE_API_KEY", "binance_api_key"])?,
        api_secret: env_any_with(lookup, &["BINANCE_API_SECRET", "binance_api_secret"])?,
        api_url: env_any_with(lookup, &["BINANCE_API_URL", "binance_api_url"]),
        sapi_api_url: env_any_with(lookup, &["BINANCE_SAPI_API_URL", "binance_sapi_api_url"]),
        web_api_url: env_any_with(lookup, &["BINANCE_WEB_API_URL", "binance_web_api_url"]),
        ws_stream_url: env_any_with(lookup, &["BINANCE_WS_STREAM_URL", "binance_ws_stream_url"]),
        api_timeout_ms: env_any_with(
            lookup,
            &["BINANCE_API_TIMEOUT_MS", "binance_api_timeout_ms"],
        )
        .and_then(|value| value.parse::<u64>().ok()),
        recv_window_ms: env_any_with(
            lookup,
            &["BINANCE_RECV_WINDOW_MS", "binance_recv_window_ms"],
        )
        .and_then(|value| value.parse::<u64>().ok()),
        proxy_url: env_any_with(
            lookup,
            &[
                "BINANCE_PROXY_URL",
                "binance_proxy_url",
                "ALL_PROXY",
                "all_proxy",
                "HTTPS_PROXY",
                "https_proxy",
            ],
        )
        .and_then(normalize_proxy_url),
    })
}

fn env_any_with<F>(lookup: &F, names: &[&str]) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    names.iter().find_map(|name| lookup(name))
}

fn parse_boolish(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "y" | "on"
    )
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
    fn reads_configured_exchanges_from_lookup() {
        let config = SdkConfig::from_lookup(|key| match key {
            "OKX_API_KEY" => Some("okx-key".to_string()),
            "OKX_API_SECRET" => Some("okx-secret".to_string()),
            "OKX_PASSPHRASE" => Some("okx-pass".to_string()),
            "BINANCE_API_KEY" => Some("binance-key".to_string()),
            "BINANCE_API_SECRET" => Some("binance-secret".to_string()),
            "BINANCE_PROXY_URL" => Some("socks5://127.0.0.1:7897".to_string()),
            _ => None,
        });

        assert_eq!(
            config.configured_exchanges(),
            vec![ExchangeId::Okx, ExchangeId::Binance]
        );
        assert_eq!(
            config.binance.unwrap().proxy_url.as_deref(),
            Some("socks5h://127.0.0.1:7897")
        );
    }

    #[test]
    fn reads_okx_simulated_credentials_when_real_key_is_absent() {
        let config = SdkConfig::from_lookup(|key| match key {
            "OKX_SIMULATED_API_KEY" => Some("sim-key".to_string()),
            "OKX_SIMULATED_API_SECRET" => Some("sim-secret".to_string()),
            "OKX_SIMULATED_PASSPHRASE" => Some("sim-pass".to_string()),
            _ => None,
        });

        let okx = config.okx.unwrap();
        assert_eq!(okx.api_key, "sim-key");
        assert!(okx.simulated);
    }
}
