use super::BitgetAdapter;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::platform::{PlatformEvent, PlatformEventQuery};
use bitget_rs::api::announcements::AnnouncementListRequest;
use serde_json::Value;

impl BitgetAdapter {
    pub(crate) async fn platform_system_status(
        &self,
        _query: PlatformEventQuery,
    ) -> Result<Vec<PlatformEvent>> {
        Err(Error::Unsupported {
            exchange: ExchangeId::Bitget,
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
            .map_err(Error::from_bitget)?;
        Ok(announcement_events(raw))
    }
}

pub(crate) fn ensure_announcements_query(query: &PlatformEventQuery) -> Result<()> {
    if query.page.is_some() || query.id.is_some() || query.state.is_some() {
        return Err(Error::Unsupported {
            exchange: ExchangeId::Bitget,
            capability: "platform announcements filters",
        });
    }

    Ok(())
}

pub(crate) fn announcement_request(query: &PlatformEventQuery) -> AnnouncementListRequest {
    let mut request = AnnouncementListRequest::new(query.locale.as_deref().unwrap_or("en-US"));
    if let Some(ann_type) = query.event_type.as_deref() {
        request = request.with_ann_type(ann_type);
    }
    if let Some(sub_type) = query.tag.as_deref() {
        request = request.with_ann_sub_type(sub_type);
    }
    if let Some(limit) = query.limit {
        request = request.with_limit(limit);
    }
    request
}

pub(crate) fn announcement_events(raw: Value) -> Vec<PlatformEvent> {
    raw.get("announcements")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(announcement_event)
        .collect()
}

fn announcement_event(raw: Value) -> PlatformEvent {
    PlatformEvent {
        exchange: ExchangeId::Bitget,
        event_type: "announcement".to_string(),
        event_id: first_string_field(&raw, &["annId", "id"]),
        title: first_string_field(&raw, &["annTitle", "title"]),
        status: first_string_field(&raw, &["annType", "type"]),
        url: first_string_field(&raw, &["annUrl", "url"]),
        start_time: None,
        end_time: None,
        published_at: first_u64_field(&raw, &["cTime", "pTime", "publishTime", "publishedAt"]),
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
