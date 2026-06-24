mod api;
pub(crate) mod client;
mod config;
mod dto;
mod error;

pub use client::{CancelOrderRequest, GateClient, OrderRequest};
pub use config::{Config, Credentials};
pub use dto::account::GateAccountBookRequest;
pub use error::Error;
