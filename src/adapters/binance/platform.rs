use super::BinanceAdapter;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::platform::{PlatformEvent, PlatformEventQuery};
use binance_rs::api::announcements::AnnouncementListRequest;
use serde_json::Value;

impl BinanceAdapter {
    pub(crate) async fn platform_system_status(
        &self,
        _query: PlatformEventQuery,
    ) -> Result<Vec<PlatformEvent>> {
        Err(Error::Unsupported {
            exchange: ExchangeId::Binance,
            capability: "platform system status",
        })
    }

    pub(crate) async fn platform_announcements(
        &self,
        query: PlatformEventQuery,
    ) -> Result<Vec<PlatformEvent>> {
        ensure_announcements_query(&query)?;
        let raw = self
            .announcements
            .get_announcements(announcement_request(&query))
            .await
            .map_err(Error::from_binance)?;
        Ok(announcement_events(raw))
    }
}

pub(crate) fn ensure_announcements_query(query: &PlatformEventQuery) -> Result<()> {
    if query.locale.is_some()
        || query.event_type.is_some()
        || query.tag.is_some()
        || query.id.is_some()
        || query.state.is_some()
    {
        return Err(Error::Unsupported {
            exchange: ExchangeId::Binance,
            capability: "platform announcements filters",
        });
    }

    Ok(())
}

pub(crate) fn announcement_request(query: &PlatformEventQuery) -> AnnouncementListRequest {
    let mut request = AnnouncementListRequest::latest();
    if let Some(page) = query.page {
        request = request.with_page(page);
    }
    if let Some(limit) = query.limit {
        request = request.with_page_size(limit);
    }
    request
}

pub(crate) fn announcement_events(raw: Value) -> Vec<PlatformEvent> {
    raw.get("data")
        .and_then(|data| data.get("articles"))
        .or_else(|| raw.get("articles"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(announcement_event)
        .collect()
}

fn announcement_event(raw: Value) -> PlatformEvent {
    PlatformEvent {
        exchange: ExchangeId::Binance,
        event_type: "announcement".to_string(),
        event_id: first_string_field(&raw, &["id", "articleId"]),
        title: string_field(&raw, "title"),
        status: None,
        url: first_string_field(&raw, &["url", "articleUrl"]),
        start_time: None,
        end_time: None,
        published_at: first_u64_field(&raw, &["releaseDate", "publishDate", "publishedAt"]),
        raw,
    }
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(non_empty_value)
}

fn first_string_field(value: &Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| string_field(value, field))
}

fn u64_field(value: &Value, field: &str) -> Option<u64> {
    value.get(field).and_then(|value| match value {
        Value::Number(value) => value.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    })
}

fn first_u64_field(value: &Value, fields: &[&str]) -> Option<u64> {
    fields.iter().find_map(|field| u64_field(value, field))
}

fn non_empty_value(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}
