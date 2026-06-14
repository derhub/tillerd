//! User-facing notification feed (roadmap 0.0.10). Derives notifications from the lifecycle
//! signals the desktop host already receives, persists them in the orchestrator store
//! (ADR-0031), and pushes them to the renderer over [`NOTIFICATION_EVENT`]. Additive: no
//! orchestrator-core seam changes. The builders are pure so the derivation is unit-tested
//! without a running app.

use std::time::{SystemTime, UNIX_EPOCH};

use orchestrator::persistence::{NotificationRecord, Store};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::orchestrator_host::{OrchestratorState, ServiceHealthWire, ServiceStateWire};

/// Renderer event carrying one notification. Mirrors the SDK `NOTIFICATION_EVENT`.
pub const NOTIFICATION_EVENT: &str = "notification://event";

/// Durable-history retention: keep the most recent N, prune older on each insert.
const MAX_HISTORY: u32 = 500;

/// How many notifications the renderer hydrates on boot (most recent first).
const HISTORY_LOAD: u32 = 200;

/// One notification on the wire. Field names match the SDK `NotificationEvent`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationWire {
    pub id: String,
    pub category: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub ts: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

    fn to_record(&self) -> NotificationRecord {
        NotificationRecord {
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
        }
    }

    fn from_record(rec: NotificationRecord) -> Self {
        Self {
            id: rec.id,
            category: rec.category,
            severity: rec.severity,
            title: rec.title,
            message: rec.message,
            detail: rec.detail,
            ts: rec.ts,
            session_id: rec.session_id,
            surface_id: rec.surface_id,
        }
    }
}

/// Current wall-clock time in epoch milliseconds (the notification timestamp).
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Strip the `tillerd-` prefix for a compact service label.
fn short_service(name: &str) -> &str {
    name.strip_prefix("tillerd-").unwrap_or(name)
}

fn qualifier_severity(qualifier: &str) -> &'static str {
    match qualifier {
        "ok" | "stopped-by-request" => "info",
        _ => "warning",
    }
}

// ── builders (pure) ───────────────────────────────────────────────────────

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

pub fn surface_stopped(
    surface_id: &str,
    session_id: Option<String>,
    qualifier: &str,
    ts: i64,
) -> NotificationWire {
    NotificationWire::new(
        "surface-stopped",
        qualifier_severity(qualifier),
        "Terminal stopped",
        format!("A terminal stopped ({qualifier})"),
        ts,
    )
    .with_surface(surface_id)
    .with_session(session_id)
}

pub fn surface_error(
    surface_id: &str,
    session_id: Option<String>,
    reason: &str,
    ts: i64,
) -> NotificationWire {
    NotificationWire::new(
        "surface-error",
        "error",
        "Terminal error",
        format!("Terminal error: {reason}"),
        ts,
    )
    .with_surface(surface_id)
    .with_session(session_id)
}

fn service_down(name: &str, ts: i64) -> NotificationWire {
    NotificationWire::new(
        "service-down",
        "error",
        "Service down",
        format!("{} is unavailable", short_service(name)),
        ts,
    )
}

fn service_up(name: &str, ts: i64) -> NotificationWire {
    NotificationWire::new(
        "service-up",
        "info",
        "Service up",
        format!("{} is available", short_service(name)),
        ts,
    )
}

fn is_down(state: ServiceStateWire) -> bool {
    matches!(state, ServiceStateWire::Unavailable)
}

/// Diff two health snapshots into up/down notifications. Only a service present in BOTH
/// snapshots that crosses the available/unavailable boundary yields a notification — a
/// first-seen service (boot) or an unchanged state yields nothing.
pub fn health_change_notifications(
    prev: &[ServiceHealthWire],
    next: &[ServiceHealthWire],
    ts: i64,
) -> Vec<NotificationWire> {
    let mut out = Vec::new();
    for n in next {
        let Some(p) = prev.iter().find(|p| p.name == n.name) else {
            continue;
        };
        match (is_down(p.state), is_down(n.state)) {
            (false, true) => out.push(service_down(&n.name, ts)),
            (true, false) => out.push(service_up(&n.name, ts)),
            _ => {}
        }
    }
    out
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

// ── sink ──────────────────────────────────────────────────────────────────

/// Persist a notification (pruning to [`MAX_HISTORY`]) and push it to the renderer.
/// Best-effort: a store or emit error never blocks the originating lifecycle event.
pub fn record<R: tauri::Runtime>(app: &AppHandle<R>, store: &dyn Store, wire: NotificationWire) {
    let _ = store.insert_notification(&wire.to_record());
    let _ = store.prune_notifications(MAX_HISTORY);
    let _ = app.emit(NOTIFICATION_EVENT, wire);
}

/// Push a notification to the renderer without persisting it. For the boot-failure case,
/// where the store may be unavailable; the live session still sees it.
pub fn emit_only<R: tauri::Runtime>(app: &AppHandle<R>, wire: NotificationWire) {
    let _ = app.emit(NOTIFICATION_EVENT, wire);
}

/// Durable notification history (most recent first) for the renderer to hydrate on boot.
/// Empty until the orchestrator has booted (the store is unavailable before then).
#[tauri::command]
pub fn notifications_list(state: State<'_, OrchestratorState>) -> Vec<NotificationWire> {
    let Some(store) = state.store_arc() else {
        return Vec::new();
    };
    store
        .list_notifications(HISTORY_LOAD)
        .unwrap_or_default()
        .into_iter()
        .map(NotificationWire::from_record)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn health(name: &str, state: ServiceStateWire) -> ServiceHealthWire {
        ServiceHealthWire {
            name: name.to_string(),
            version: None,
            state,
        }
    }

    #[test]
    fn service_going_unavailable_yields_a_down_notification() {
        let prev = vec![health("tillerd-gate", ServiceStateWire::Ready)];
        let next = vec![health("tillerd-gate", ServiceStateWire::Unavailable)];
        let out = health_change_notifications(&prev, &next, 1);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].category, "service-down");
        assert_eq!(out[0].severity, "error");
        assert!(out[0].message.contains("gate"));
    }

    #[test]
    fn service_recovering_yields_an_up_notification() {
        let prev = vec![health("tillerd-daemon", ServiceStateWire::Unavailable)];
        let next = vec![health("tillerd-daemon", ServiceStateWire::Ready)];
        let out = health_change_notifications(&prev, &next, 1);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].category, "service-up");
    }

    #[test]
    fn unchanged_snapshot_yields_nothing() {
        let prev = vec![health("tillerd-gate", ServiceStateWire::Ready)];
        let next = vec![health("tillerd-gate", ServiceStateWire::Ready)];
        assert!(health_change_notifications(&prev, &next, 1).is_empty());
    }

    #[test]
    fn first_seen_service_yields_nothing() {
        let prev: Vec<ServiceHealthWire> = vec![];
        let next = vec![health("tillerd-gate", ServiceStateWire::Unavailable)];
        assert!(health_change_notifications(&prev, &next, 1).is_empty());
    }

    #[test]
    fn surface_stopped_carries_session_and_qualifier_severity() {
        let n = surface_stopped("surf-1", Some("sess-1".to_string()), "faulted", 7);
        assert_eq!(n.category, "surface-stopped");
        assert_eq!(n.severity, "warning");
        assert_eq!(n.session_id.as_deref(), Some("sess-1"));
        assert_eq!(n.surface_id.as_deref(), Some("surf-1"));
        assert_eq!(n.ts, 7);
    }
}
