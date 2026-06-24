use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlatformEvent {
    pub exchange: ExchangeId,
    pub event_type: String,
    pub event_id: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub url: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub published_at: Option<u64>,
    pub raw: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformEventQuery {
    pub locale: Option<String>,
    pub event_type: Option<String>,
    pub tag: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub id: Option<String>,
    pub state: Option<String>,
}

impl PlatformEventQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_locale(mut self, value: impl Into<String>) -> Self {
        self.locale = Some(value.into());
        self
    }

    pub fn with_event_type(mut self, value: impl Into<String>) -> Self {
        self.event_type = Some(value.into());
        self
    }

    pub fn with_tag(mut self, value: impl Into<String>) -> Self {
        self.tag = Some(value.into());
        self
    }

    pub fn with_page(mut self, value: u32) -> Self {
        self.page = Some(value);
        self
    }

    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }

    pub fn with_id(mut self, value: impl Into<String>) -> Self {
        self.id = Some(value.into());
        self
    }

    pub fn with_state(mut self, value: impl Into<String>) -> Self {
        self.state = Some(value.into());
        self
    }
}

pub struct PlatformFacade<'a> {
    pub(crate) client: &'a ExchangeClient,
}

impl<'a> PlatformFacade<'a> {
    pub(crate) fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    pub async fn system_status(&self, query: PlatformEventQuery) -> Result<Vec<PlatformEvent>> {
        self.client.platform_system_status(query).await
    }

    pub async fn announcements(&self, query: PlatformEventQuery) -> Result<Vec<PlatformEvent>> {
        self.client.platform_announcements(query).await
    }
}
