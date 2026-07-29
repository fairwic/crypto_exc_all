#[cfg(feature = "full")]
pub mod account;
#[cfg(feature = "full")]
pub mod announcements;
pub mod api_trait;
#[cfg(feature = "full")]
pub mod asset;
#[cfg(feature = "full")]
pub mod big_data;
pub mod market;
#[cfg(feature = "full")]
pub mod public_data;
#[cfg(feature = "full")]
pub mod trade;
#[cfg(feature = "full")]
pub mod websocket;
// 重新导出已移动的模块
#[cfg(feature = "full")]
pub use websocket::OkxWebsocketApi;

// 常量定义
#[cfg(feature = "full")]
pub const API_ACCOUNT_PATH: &str = "/api/v5/account";
#[cfg(feature = "full")]
pub const API_TRADE_PATH: &str = "/api/v5/trade";
pub const API_MARKET_PATH: &str = "/api/v5/market";
#[cfg(feature = "full")]
pub const API_PUBLIC_PATH: &str = "/api/v5/public";
#[cfg(feature = "full")]
pub const API_ASSET_PATH: &str = "/api/v5/asset";
#[cfg(feature = "full")]
pub const API_SYSTEM_PATH: &str = "/api/v5/system";
#[cfg(feature = "full")]
pub const API_BIGDATA_PATH: &str = "/api/v5/rubik";
#[cfg(feature = "full")]
pub const API_ANNOUNCEMENTS_PATH: &str = "/api/v5/support/announcements";
