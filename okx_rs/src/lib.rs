// OKX SDK - Rust Client Library
// 提供与OKX交易所API的通信能力

pub mod api;
pub mod client;
#[cfg(feature = "full")]
pub mod config;
#[cfg(feature = "full")]
pub mod debug_helper;
pub mod dto;
pub mod enums;
pub mod error;
#[cfg(feature = "full")]
pub mod utils;
#[cfg(feature = "full")]
pub mod websocket;

/// OKX SDK的版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use api::market::OkxMarket;
#[cfg(feature = "public-market")]
pub use api::public_data::OkxPublicInstruments;
#[cfg(feature = "full")]
pub use api::{
    account::OkxAccount, asset::OkxAsset, big_data::OkxBigData, public_data::OkxPublicData,
    trade::OkxTrade, websocket::OkxWebsocketApi,
};
/// Re-export commonly used modules and functions
pub use client::OkxClient;
#[cfg(feature = "public-market")]
pub use client::{
    OkxPublicFailureEvidence, OkxPublicFailureKind, OkxPublicResponse, OkxPublicResponseEvidence,
};
#[cfg(feature = "public-market")]
pub use dto::public_data::OkxPublicInstrument;
pub use error::Error;
#[cfg(feature = "full")]
pub use websocket::OkxWebsocketClient;
