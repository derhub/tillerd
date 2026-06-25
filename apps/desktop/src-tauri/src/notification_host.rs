//! User-facing notification feed (roadmap 0.0.10). Lifecycle signals are recorded
//! by the orchestrator's notification-recording layer (the single recording
//! point); this host forwards each recorded notification to the renderer over
//! [`NOTIFICATION_EVENT`] and serves the durable history for boot hydration.

use std::time::{SystemTime, UNIX_EPOCH};

use orchestrator::app::notification::{
    ListNotifications, NotificationSink, NotificationView, RecordNotification,
};
use orchestrator::shared::Bus;
use orchestrator::Ctx;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime, State};

/// Renderer event carrying one notification. Mirrors the SDK `NOTIFICATION_EVENT`.
pub const NOTIFICATION_EVENT: &str = "notification://event";

/// How many notifications the renderer hydrates on boot (most recent first).
const HISTORY_LOAD: u32 = 200;

/// One notification on the wire. Field names match the SDK `NotificationEvent`.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "notification://event")]
#[serde(rename_all = "camelCase")]
// Optionals serialize as null (not omitted) so serialize/deserialize stay symmetric and specta
// emits a single type rather than _Serialize/_Deserialize variants.
pub struct NotificationWire {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    pub ts: i64,
    pub session_id: Option<String>,
    pub surface_id: Option<String>,
}

impl NotificationWire {
    fn from_record(n: &RecordNotification) -> Self {
        Self {
            id: n.id.clone(),
            category: n.category.clone(),
            severity: n.severity.clone(),
            title: n.title.clone(),
            message: n.message.clone(),
            detail: n.detail.clone(),
            ts: n.ts,
            session_id: n.session_id.clone(),
            surface_id: n.surface_id.clone(),
        }
    }

    fn from_view(view: NotificationView) -> Self {
        Self {
            id: view.id,
            category: view.category,
            severity: view.severity,
            title: view.title,
            message: view.message,
            detail: view.detail,
            ts: view.ts,
            session_id: view.session_id,
            surface_id: view.surface_id,
        }
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Forwards each notification the recording layer persists to the renderer. The
/// orchestrator records (off an `AppHandle`); this sink only emits the live push,
/// so a notification is never recorded twice.
pub struct NotificationForwarder<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> NotificationForwarder<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> NotificationSink for NotificationForwarder<R> {
    fn emit(&self, notification: &RecordNotification) {
        let _ = self
            .app
            .emit(NOTIFICATION_EVENT, NotificationWire::from_record(notification));
    }
}

/// Durable notification history (most recent first) for the renderer to hydrate on boot.
#[tauri::command]
#[specta::specta]
pub async fn notifications_list(bus: State<'_, Bus<Ctx>>) -> Result<Vec<NotificationWire>, String> {
    let listing = bus
        .query(ListNotifications {
            limit: Some(HISTORY_LOAD),
            offset: Some(0),
            after: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(listing
        .items
        .into_iter()
        .map(NotificationWire::from_view)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_record_carries_session_and_surface() {
        let record = RecordNotification {
            id: "n1".to_owned(),
            category: "surface-started".to_owned(),
            severity: "info".to_owned(),
            title: Some("Terminal started".to_owned()),
            message: "A terminal started".to_owned(),
            detail: None,
            ts: 7,
            session_id: Some("sess-1".to_owned()),
            surface_id: Some("surf-1".to_owned()),
            actions_json: None,
            read: false,
            snooze_until: None,
        };

        let wire = NotificationWire::from_record(&record);

        assert_eq!(wire.category, "surface-started");
        assert_eq!(wire.severity, "info");
        assert_eq!(wire.session_id.as_deref(), Some("sess-1"));
        assert_eq!(wire.surface_id.as_deref(), Some("surf-1"));
        assert_eq!(wire.ts, 7);
    }
}
