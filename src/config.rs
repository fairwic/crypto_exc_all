use crate::exchange::ExchangeId;
use std::env;
use std::path::PathBuf;
use std::sync::Once;

static INIT_ENV: Once = Once::new();

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SdkConfig {
    pub okx: Option<OkxExchangeConfig>,
    pub binance: Option<BinanceExchangeConfig>,
    pub bitget: Option<BitgetExchangeConfig>,
    pub bybit: Option<BybitExchangeConfig>,
    pub gate: Option<GateExchangeConfig>,
    pub hyperliquid: Option<HyperliquidExchangeConfig>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitgetExchangeConfig {
    pub api_key: String,
    pub api_secret: String,
    pub passphrase: String,
    pub api_url: Option<String>,
    pub api_timeout_ms: Option<u64>,
    pub proxy_url: Option<String>,
    pub product_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BybitExchangeConfig {
    pub api_key: String,
    pub api_secret: String,
    pub api_url: Option<String>,
    pub api_timeout_ms: Option<u64>,
    pub recv_window_ms: Option<u64>,
    pub proxy_url: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateExchangeConfig {
    pub api_key: String,
    pub api_secret: String,
    pub api_url: Option<String>,
    pub api_timeout_ms: Option<u64>,
    pub proxy_url: Option<String>,
    pub settle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyperliquidExchangeConfig {
    pub api_url: Option<String>,
    pub api_timeout_ms: Option<u64>,
    pub proxy_url: Option<String>,
    pub user_address: Option<String>,
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
            bitget: read_bitget_config(&lookup),
            bybit: read_bybit_config(&lookup),
            gate: read_gate_config(&lookup),
            hyperliquid: read_hyperliquid_config(&lookup),
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
        if self.bitget.is_some() {
            exchanges.push(ExchangeId::Bitget);
        }
        if self.bybit.is_some() {
            exchanges.push(ExchangeId::Bybit);
        }
        if self.gate.is_some() {
            exchanges.push(ExchangeId::Gate);
        }
        if self.hyperliquid.is_some() {
            exchanges.push(ExchangeId::Hyperliquid);
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
        proxy_url: env_any_with(lookup, &["BINANCE_PROXY_URL", "binance_proxy_url"]),
    })
}

fn read_bitget_config<F>(lookup: &F) -> Option<BitgetExchangeConfig>
where
    F: Fn(&str) -> Option<String>,
{
    Some(BitgetExchangeConfig {
        api_key: env_any_with(lookup, &["BITGET_API_KEY", "bitget_api_key"])?,
        api_secret: env_any_with(lookup, &["BITGET_API_SECRET", "bitget_api_secret"])?,
        passphrase: env_any_with(
            lookup,
            &[
                "BITGET_PASSPHRASE",
                "BITGET_API_PASSPHRASE",
                "bitget_PASSPHRASE",
                "bitget_passphrase",
                "bitget_api_passphrase",
            ],
        )?,
        api_url: env_any_with(lookup, &["BITGET_API_URL", "bitget_api_url"]),
        api_timeout_ms: env_any_with(lookup, &["BITGET_API_TIMEOUT_MS", "bitget_api_timeout_ms"])
            .and_then(|value| value.parse::<u64>().ok()),
        proxy_url: None,
        product_type: env_any_with(lookup, &["BITGET_PRODUCT_TYPE", "bitget_product_type"]),
    })
}

fn read_bybit_config<F>(lookup: &F) -> Option<BybitExchangeConfig>
where
    F: Fn(&str) -> Option<String>,
{
    Some(BybitExchangeConfig {
        api_key: env_any_with(lookup, &["BYBIT_API_KEY", "bybit_api_key"])?,
        api_secret: env_any_with(lookup, &["BYBIT_API_SECRET", "bybit_api_secret"])?,
        api_url: env_any_with(lookup, &["BYBIT_API_URL", "bybit_api_url"]),
        api_timeout_ms: env_any_with(lookup, &["BYBIT_API_TIMEOUT_MS", "bybit_api_timeout_ms"])
            .and_then(|value| value.parse::<u64>().ok()),
        recv_window_ms: env_any_with(lookup, &["BYBIT_RECV_WINDOW_MS", "bybit_recv_window_ms"])
            .and_then(|value| value.parse::<u64>().ok()),
        proxy_url: env_any_with(lookup, &["BYBIT_PROXY_URL", "bybit_proxy_url"]),
        category: env_any_with(lookup, &["BYBIT_CATEGORY", "bybit_category"]),
    })
}

fn read_gate_config<F>(lookup: &F) -> Option<GateExchangeConfig>
where
    F: Fn(&str) -> Option<String>,
{
    Some(GateExchangeConfig {
        api_key: env_any_with(lookup, &["GATE_API_KEY", "gate_api_key"])?,
        api_secret: env_any_with(lookup, &["GATE_API_SECRET", "gate_api_secret"])?,
        api_url: env_any_with(lookup, &["GATE_API_URL", "gate_api_url"]),
        api_timeout_ms: env_any_with(lookup, &["GATE_API_TIMEOUT_MS", "gate_api_timeout_ms"])
            .and_then(|value| value.parse::<u64>().ok()),
        proxy_url: env_any_with(lookup, &["GATE_PROXY_URL", "gate_proxy_url"]),
        settle: env_any_with(lookup, &["GATE_SETTLE", "gate_settle"]),
    })
}

fn read_hyperliquid_config<F>(lookup: &F) -> Option<HyperliquidExchangeConfig>
where
    F: Fn(&str) -> Option<String>,
{
    let enabled = env_any_with(lookup, &["HYPERLIQUID_ENABLED", "hyperliquid_enabled"])
        .map(|value| parse_boolish(&value))
        .unwrap_or(false);
    let api_url = env_any_with(lookup, &["HYPERLIQUID_API_URL", "hyperliquid_api_url"]);
    let user_address = env_any_with(
        lookup,
        &["HYPERLIQUID_USER_ADDRESS", "hyperliquid_user_address"],
    );

    if !enabled && api_url.is_none() && user_address.is_none() {
        return None;
    }

    Some(HyperliquidExchangeConfig {
        api_url,
        api_timeout_ms: env_any_with(
            lookup,
            &["HYPERLIQUID_API_TIMEOUT_MS", "hyperliquid_api_timeout_ms"],
        )
        .and_then(|value| value.parse::<u64>().ok()),
        proxy_url: env_any_with(lookup, &["HYPERLIQUID_PROXY_URL", "hyperliquid_proxy_url"]),
        user_address,
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
            "BINANCE_PROXY_URL" => Some("socks5h://127.0.0.1:7897".to_string()),
            "BITGET_API_KEY" => Some("bitget-key".to_string()),
            "BITGET_API_SECRET" => Some("bitget-secret".to_string()),
            "bitget_PASSPHRASE" => Some("bitget-pass".to_string()),
            "BITGET_PRODUCT_TYPE" => Some("USDT-FUTURES".to_string()),
            "BYBIT_API_KEY" => Some("bybit-key".to_string()),
            "BYBIT_API_SECRET" => Some("bybit-secret".to_string()),
            "BYBIT_CATEGORY" => Some("linear".to_string()),
            "GATE_API_KEY" => Some("gate-key".to_string()),
            "GATE_API_SECRET" => Some("gate-secret".to_string()),
            "GATE_SETTLE" => Some("usdt".to_string()),
            _ => None,
        });

        assert_eq!(
            config.configured_exchanges(),
            vec![
                ExchangeId::Okx,
                ExchangeId::Binance,
                ExchangeId::Bitget,
                ExchangeId::Bybit,
                ExchangeId::Gate
            ]
        );
        assert_eq!(
            config.binance.unwrap().proxy_url.as_deref(),
            Some("socks5h://127.0.0.1:7897")
        );
        let bitget = config.bitget.unwrap();
        assert_eq!(bitget.passphrase, "bitget-pass");
        assert_eq!(bitget.product_type.as_deref(), Some("USDT-FUTURES"));
        assert_eq!(bitget.proxy_url, None);
        assert_eq!(config.bybit.unwrap().category.as_deref(), Some("linear"));
        assert_eq!(config.gate.unwrap().settle.as_deref(), Some("usdt"));
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
