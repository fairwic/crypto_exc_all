use super::OkxAdapter;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::platform::{PlatformEvent, PlatformEventQuery};
use okx_rs::api::announcements::announcements_api::{AnnouncementDetail, AnnouncementPage};
use okx_rs::dto::public_data_dto::SystemStatus;
use okx_rs::enums::language_enums::Language;

impl OkxAdapter {
    pub(crate) async fn platform_system_status(
        &self,
        query: PlatformEventQuery,
    ) -> Result<Vec<PlatformEvent>> {
        ensure_system_status_query(&query)?;
        let raw = self
            .public_data
            .get_status()
            .await
            .map_err(Error::from_okx)?;
        system_status_events(raw)
    }

    pub(crate) async fn platform_announcements(
        &self,
        query: PlatformEventQuery,
    ) -> Result<Vec<PlatformEvent>> {
        ensure_announcements_query(&query)?;
        let language = announcement_language(&query)?;
        let raw = self
            .announcements
            .get_announcements(
                query.event_type.clone(),
                query.page.map(|value| value.to_string()),
                language,
            )
            .await
            .map_err(Error::from_okx)?;
        announcement_events(raw)
    }
}

pub(crate) fn ensure_system_status_query(query: &PlatformEventQuery) -> Result<()> {
    if query.locale.is_some()
        || query.event_type.is_some()
        || query.tag.is_some()
        || query.page.is_some()
        || query.limit.is_some()
        || query.id.is_some()
        || query.state.is_some()
    {
        return Err(Error::Unsupported {
            exchange: ExchangeId::Okx,
            capability: "platform system status filters",
        });
    }

    Ok(())
}

pub(crate) fn announcement_language(query: &PlatformEventQuery) -> Result<Option<Language>> {
    match query.locale.as_deref() {
        None => Ok(None),
        Some("en-US") | Some("en") => Ok(Some(Language::EnUs)),
        Some("zh-CN") | Some("zh") => Ok(Some(Language::ZhCn)),
        Some(_) => Err(Error::Unsupported {
            exchange: ExchangeId::Okx,
            capability: "platform announcements locale",
        }),
    }
}

pub(crate) fn ensure_announcements_query(query: &PlatformEventQuery) -> Result<()> {
    if query.tag.is_some() || query.limit.is_some() || query.id.is_some() || query.state.is_some() {
        return Err(Error::Unsupported {
            exchange: ExchangeId::Okx,
            capability: "platform announcements filters",
        });
    }

    Ok(())
}

pub(crate) fn system_status_events(statuses: Vec<SystemStatus>) -> Result<Vec<PlatformEvent>> {
    statuses
        .into_iter()
        .map(system_status_event)
        .collect::<Result<Vec<_>>>()
}

pub(crate) fn announcement_events(pages: Vec<AnnouncementPage>) -> Result<Vec<PlatformEvent>> {
    pages
        .into_iter()
        .flat_map(|page| page.details)
        .map(announcement_event)
        .collect::<Result<Vec<_>>>()
}

fn system_status_event(status: SystemStatus) -> Result<PlatformEvent> {
    let raw = serde_json::to_value(&status)?;
    Ok(PlatformEvent {
        exchange: ExchangeId::Okx,
        event_type: "system_status".to_string(),
        event_id: status.system.and_then(non_empty),
        title: non_empty(status.title),
        status: non_empty(status.state),
        url: status.href.and_then(non_empty),
        start_time: status.begin.as_deref().and_then(parse_u64_string),
        end_time: status.end.as_deref().and_then(parse_u64_string),
        published_at: None,
        raw,
    })
}

fn announcement_event(detail: AnnouncementDetail) -> Result<PlatformEvent> {
    let raw = serde_json::to_value(&detail)?;
    Ok(PlatformEvent {
        exchange: ExchangeId::Okx,
        event_type: "announcement".to_string(),
        event_id: None,
        title: non_empty(detail.title),
        status: non_empty(detail.ann_type),
        url: non_empty(detail.url),
        start_time: None,
        end_time: None,
        published_at: parse_u64_string(&detail.p_time),
        raw,
    })
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn parse_u64_string(value: &str) -> Option<u64> {
    if value.is_empty() {
        None
    } else {
        value.parse::<u64>().ok()
    }
}
