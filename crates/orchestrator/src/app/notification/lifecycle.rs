use serde::Deserialize;

use crate::app::notification::RecordNotification;
use crate::context::Ctx;
use crate::shared::bus::Notable;
use crate::shared::message::Command;
use crate::shared::Result;

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceStarted {
    pub surface_id: String,
    pub session_id: String,
    pub ts: i64,
}

impl Command<Ctx> for SurfaceStarted {
    async fn handle(&self, _cx: &Ctx) -> Result<()> {
        Ok(())
    }
}

impl Notable for SurfaceStarted {
    fn notification(&self) -> Option<RecordNotification> {
        Some(RecordNotification {
            id: new_id(),
            category: "surface-started".to_owned(),
            severity: "info".to_owned(),
            title: Some("Terminal started".to_owned()),
            message: "A terminal started".to_owned(),
            detail: None,
            ts: self.ts,
            session_id: Some(self.session_id.clone()),
            surface_id: Some(self.surface_id.clone()),
            actions_json: None,
            read: false,
            snooze_until: None,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestratorStatus {
    pub ready: bool,
    pub reason: Option<String>,
    pub ts: i64,
}

impl Command<Ctx> for OrchestratorStatus {
    async fn handle(&self, _cx: &Ctx) -> Result<()> {
        Ok(())
    }
}

impl Notable for OrchestratorStatus {
    fn notification(&self) -> Option<RecordNotification> {
        let (severity, title, message) = if self.ready {
            ("info", "Ready", "All services are ready".to_owned())
        } else {
            (
                "error",
                "Startup failed",
                self.reason
                    .as_deref()
                    .map(|r| format!("Startup failed: {r}"))
                    .unwrap_or_else(|| "Startup failed".to_owned()),
            )
        };
        Some(RecordNotification {
            id: new_id(),
            category: "orchestrator-status".to_owned(),
            severity: severity.to_owned(),
            title: Some(title.to_owned()),
            message,
            detail: None,
            ts: self.ts,
            session_id: None,
            surface_id: None,
            actions_json: None,
            read: false,
            snooze_until: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_started_notification_carries_session_and_surface() {
        let n = SurfaceStarted {
            surface_id: "surf-1".to_owned(),
            session_id: "sess-1".to_owned(),
            ts: 7,
        }
        .notification()
        .expect("surface start is notification-worthy");

        assert_eq!(n.category, "surface-started");
        assert_eq!(n.severity, "info");
        assert_eq!(n.session_id.as_deref(), Some("sess-1"));
        assert_eq!(n.surface_id.as_deref(), Some("surf-1"));
        assert_eq!(n.ts, 7);
    }

    #[test]
    fn orchestrator_status_ready_is_info() {
        let n = OrchestratorStatus {
            ready: true,
            reason: None,
            ts: 1,
        }
        .notification()
        .expect("status change is notification-worthy");

        assert_eq!(n.category, "orchestrator-status");
        assert_eq!(n.severity, "info");
    }

    #[test]
    fn orchestrator_status_failure_carries_reason() {
        let n = OrchestratorStatus {
            ready: false,
            reason: Some("boom".to_owned()),
            ts: 1,
        }
        .notification()
        .expect("status change is notification-worthy");

        assert_eq!(n.severity, "error");
        assert!(n.message.contains("boom"));
    }
}
