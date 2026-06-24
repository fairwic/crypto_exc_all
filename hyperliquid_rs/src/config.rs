#[derive(Clone, Debug)]
pub struct Config {
    pub api_url: String,
    pub api_timeout_ms: u64,
    pub proxy_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let _ = dotenv::dotenv();
        Self {
            api_url: std::env::var("HYPERLIQUID_API_URL")
                .unwrap_or_else(|_| "https://api.hyperliquid.xyz".to_string()),
            api_timeout_ms: std::env::var("HYPERLIQUID_API_TIMEOUT_MS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(5_000),
            proxy_url: std::env::var("HYPERLIQUID_PROXY_URL").ok(),
        }
    }
}
