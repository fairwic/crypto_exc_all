use crate::error::Error;

#[derive(Clone, Debug)]
pub struct Config {
    pub api_url: String,
    pub api_timeout_ms: u64,
    pub proxy_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Credentials {
    pub api_key: String,
    pub api_secret: String,
}

impl Config {
    pub fn from_env() -> Self {
        let _ = dotenv::dotenv();
        Self {
            api_url: std::env::var("GATE_API_URL")
                .unwrap_or_else(|_| "https://api.gateio.ws/api/v4".to_string()),
            api_timeout_ms: std::env::var("GATE_API_TIMEOUT_MS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(5_000),
            proxy_url: std::env::var("GATE_PROXY_URL").ok(),
        }
    }
}

impl Credentials {
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
        }
    }

    pub fn from_env() -> Result<Self, Error> {
        let _ = dotenv::dotenv();
        Ok(Self {
            api_key: std::env::var("GATE_API_KEY")
                .map_err(|_| Error::ConfigError("GATE_API_KEY is required".to_string()))?,
            api_secret: std::env::var("GATE_API_SECRET")
                .map_err(|_| Error::ConfigError("GATE_API_SECRET is required".to_string()))?,
        })
    }
}
