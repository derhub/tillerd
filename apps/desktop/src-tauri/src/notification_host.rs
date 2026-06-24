//! User-facing notification feed (roadmap 0.0.10). Derives notifications from the lifecycle
//! signals the desktop host already receives, persists them in the orchestrator store,
//! and pushes them to the renderer over [`NOTIFICATION_EVENT`]. Additive: no
//! orchestrator-core boundary changes. The builders are pure so the derivation is unit-tested
//! without a running app.

use std::time::{SystemTime, UNIX_EPOCH};

use orchestrator::app::notification::{
    ListNotifications, NotificationView, PruneNotifications, RecordNotification,
};
use orchestrator::shared::Bus;
use orchestrator::Ctx;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

/// Renderer event carrying one notification. Mirrors the SDK `NOTIFICATION_EVENT`.
pub const NOTIFICATION_EVENT: &str = "notification://event";

/// Durable-history retention: keep the most recent N, prune older on each insert.
const MAX_HISTORY: u32 = 500;

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
    fn new(category: &str, severity: &str, title: &str, message: String, ts: i64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            category: category.to_string(),
            severity: severity.to_string(),
            title: Some(title.to_string()),
            message,
            detail: None,
            ts,
            session_id: None,
            surface_id: None,
        }
    }

    fn with_session(mut self, session_id: Option<String>) -> Self {
        self.session_id = session_id;
        self
    }

    fn with_surface(mut self, surface_id: &str) -> Self {
        self.surface_id = Some(surface_id.to_string());
        self
    }

    fn to_record_cmd(&self) -> RecordNotification {
        RecordNotification {
            id: self.id.clone(),
            category: self.category.clone(),
            severity: self.severity.clone(),
            title: self.title.clone(),
            message: self.message.clone(),
            detail: self.detail.clone(),
            ts: self.ts,
            session_id: self.session_id.clone(),
            surface_id: self.surface_id.clone(),
            actions_json: None,
            read: false,
            snooze_until: None,
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

pub fn surface_started(surface_id: &str, session_id: &str, ts: i64) -> NotificationWire {
    NotificationWire::new(
        "surface-started",
        "info",
        "Terminal started",
        "A terminal started".to_string(),
        ts,
    )
    .with_surface(surface_id)
    .with_session(Some(session_id.to_string()))
}

/// An orchestrator-status notification for the user-relevant terminal states only
/// (ready / failed); intermediate boot phases are not surfaced.
pub fn orchestrator_status(ready: bool, reason: Option<&str>, ts: i64) -> NotificationWire {
    if ready {
        NotificationWire::new(
            "orchestrator-status",
            "info",
            "Ready",
            "All services are ready".to_string(),
            ts,
        )
    } else {
        NotificationWire::new(
            "orchestrator-status",
            "error",
            "Startup failed",
            reason
                .map(|r| format!("Startup failed: {r}"))
                .unwrap_or_else(|| "Startup failed".to_string()),
            ts,
        )
    }
}

/// Persist a notification (pruning to [`MAX_HISTORY`]) and push it to the renderer.
/// Best-effort: a store or emit error never blocks the originating lifecycle event.
async fn record<R: tauri::Runtime>(app: &AppHandle<R>, bus: &Bus<Ctx>, wire: NotificationWire) {
    let _ = bus.execute(wire.to_record_cmd()).await;
    let _ = bus.execute(PruneNotifications { keep: MAX_HISTORY }).await;
    let _ = app.emit(NOTIFICATION_EVENT, wire);
}

/// Fire-and-forget handle to the bootstrap recorder task. Cloneable; lives in managed state.
#[derive(Clone)]
pub struct NotificationRecorder {
    tx: tokio::sync::mpsc::UnboundedSender<NotificationWire>,
}

impl NotificationRecorder {
    /// Queue a notification for persist + emit. Drops on a closed channel; never blocks
    /// or awaits the bus on the caller's path.
    pub fn notify(&self, wire: NotificationWire) {
        let _ = self.tx.send(wire);
    }
}

/// Spawn the single long-lived recorder task on the current runtime, draining queued
/// notifications through [`record`] off the producers' path. Returns the send handle.
pub fn spawn_recorder(app: AppHandle, bus: Bus<Ctx>) -> NotificationRecorder {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<NotificationWire>();
    tokio::spawn(async move {
        while let Some(wire) = rx.recv().await {
            record(&app, &bus, wire).await;
        }
    });
    NotificationRecorder { tx }
}

/// Push a notification to the renderer without persisting it. For the boot-failure case,
/// where the store may be unavailable; the live session still sees it.
pub fn emit_only<R: tauri::Runtime>(app: &AppHandle<R>, wire: NotificationWire) {
    let _ = app.emit(NOTIFICATION_EVENT, wire);
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
    fn surface_started_carries_session_and_surface() {
        let n = surface_started("surf-1", "sess-1", 7);
        assert_eq!(n.category, "surface-started");
        assert_eq!(n.severity, "info");
        assert_eq!(n.session_id.as_deref(), Some("sess-1"));
        assert_eq!(n.surface_id.as_deref(), Some("surf-1"));
        assert_eq!(n.ts, 7);
    }

    #[test]
    fn orchestrator_status_ready_is_info() {
        let n = orchestrator_status(true, None, 1);
        assert_eq!(n.category, "orchestrator-status");
        assert_eq!(n.severity, "info");
    }

    #[test]
    fn orchestrator_status_failure_carries_reason() {
        let n = orchestrator_status(false, Some("boom"), 1);
        assert_eq!(n.severity, "error");
        assert!(n.message.contains("boom"));
    }
}
