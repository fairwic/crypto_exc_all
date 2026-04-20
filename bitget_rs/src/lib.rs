pub mod api;
pub mod client;
pub mod config;
pub mod error;
pub mod utils;

pub use api::{
    account::BitgetAccount, announcements::BitgetAnnouncements, asset::BitgetAsset,
    market::BitgetMarket, trade::BitgetTrade,
};
pub use client::BitgetClient;
pub use error::Error;
