//! Surface status transitions push to every window : persist the status,
//! then emit `{surfaceId, sessionId, workspaceId, status}` to every open
//! surface-status channel so windows invalidate the activity read-model without
//! polling. Emission is best-effort and happens after the write commits, so a
//! receiver's re-query always reads the post-transition row.

use serde::Serialize;

use crate::context::Ctx;
use crate::entities::surface::{SurfaceId, SurfaceStatus};
use crate::infra::SurfaceRepo;
use crate::shared::domain_channel::{
    CloseDomainChannel, DomainChannelEvent, DomainChannelSink, OpenDomainChannel,
};
use crate::shared::Result;

/// Registry key prefix; each window registers its own `surface-status://{channelId}`.
pub const SURFACE_STATUS_PREFIX: &str = "surface-status://";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SurfaceStatusChanged<'a> {
    surface_id: &'a str,
    session_id: String,
    workspace_id: String,
    status: &'static str,
}

/// Persist a surface status transition, then push it to every subscribed window.
/// The single write path for app-layer status transitions.
pub async fn update_status_and_emit(cx: &Ctx, id: &SurfaceId, status: SurfaceStatus) -> Result<()> {
    SurfaceRepo::update_status(cx.db(), id, status).await?;
    emit_status(cx, id, status).await;
    Ok(())
}

/// Confirm a fresh spawn: `pending -> live` only. An immediately-dying PTY's
/// exit frame can land before this confirmation; the conditional write loses to
/// that terminal record instead of resurrecting a dead surface as live.
pub async fn confirm_spawn_and_emit(cx: &Ctx, id: &SurfaceId) -> Result<()> {
    let transitioned =
        SurfaceRepo::update_status_from(cx.db(), id, SurfaceStatus::Pending, SurfaceStatus::Live)
            .await?;
    if transitioned {
        emit_status(cx, id, SurfaceStatus::Live).await;
    }
    Ok(())
}

/// Best-effort push: a lookup or serialize failure must never fail the command
/// that performed the transition.
async fn emit_status(cx: &Ctx, id: &SurfaceId, status: SurfaceStatus) {
    let Ok(Some((session_id, workspace_id))) = owner_ids(cx, id).await else {
        return;
    };
    let wire = SurfaceStatusChanged {
        surface_id: id.as_str(),
        session_id,
        workspace_id,
        status: status.as_str(),
    };
    let Ok(json) = serde_json::to_vec(&wire) else {
        return;
    };
    let event = DomainChannelEvent::Bytes(&json);
    cx.domain_channel_sinks()
        .dispatch_prefix(SURFACE_STATUS_PREFIX, |sink| sink.emit(&event));
}

async fn owner_ids(cx: &Ctx, id: &SurfaceId) -> Result<Option<(String, String)>> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT s.id, p.workspace_id
         FROM surface sf
         JOIN session s ON s.id = sf.session_id
         JOIN project p ON p.id = s.project_id
         WHERE sf.id = ?",
    )
    .bind(id.as_str())
    .fetch_optional(cx.db())
    .await?;
    Ok(row)
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OpenSurfaceStatusChannel {
    pub channel_id: String,
}

impl OpenDomainChannel<Ctx> for OpenSurfaceStatusChannel {
    async fn handle(&self, cx: &Ctx, sink: std::sync::Arc<dyn DomainChannelSink>) -> Result<()> {
        cx.domain_channel_sinks()
            .register(&format!("{SURFACE_STATUS_PREFIX}{}", self.channel_id), sink);
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CloseSurfaceStatusChannel {
    pub channel_id: String,
}

impl CloseDomainChannel<Ctx> for CloseSurfaceStatusChannel {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        cx.domain_channel_sinks()
            .remove_key(&format!("{SURFACE_STATUS_PREFIX}{}", self.channel_id));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::app::workspace::test_util::{ctx, insert_workspace};
    use crate::entities::session::{Session, SessionId, SessionStatus, TitleSource};
    use crate::entities::surface::SurfaceKind;
    use crate::infra::session::SessionRepo;

    struct RecordingSink(Mutex<Vec<Vec<u8>>>);

    impl DomainChannelSink for RecordingSink {
        fn emit(&self, event: &DomainChannelEvent<'_>) {
            if let DomainChannelEvent::Bytes(bytes) = event {
                self.0.lock().unwrap().push(bytes.to_vec());
            }
        }
    }

    async fn seed_surface(cx: &Ctx) -> SurfaceId {
        insert_workspace(cx, "ws-ev-1", "W").await;
        sqlx::query("INSERT INTO project (id, workspace_id, name) VALUES (?, ?, ?)")
            .bind("p-ev-1")
            .bind("ws-ev-1")
            .bind("P")
            .execute(cx.db())
            .await
            .unwrap();
        let session = Session {
            id: SessionId::from_string("s-ev-1"),
            project_id: crate::entities::project::ProjectId::new("p-ev-1"),
            title: "S".to_owned(),
            title_source: TitleSource::AgentTitle,
            created_at: "2026-01-01T00:00:00.000Z".to_owned(),
            spec_version: None,
            spec_json: None,
            sort_order: 0,
            pinned: false,
            status: SessionStatus::Active,
        };
        SessionRepo::create(cx.db(), &session).await.unwrap();
        let surface = SurfaceRepo::create(
            cx.db(),
            None,
            &SessionId::from_string("s-ev-1"),
            SurfaceKind::Terminal,
            None,
            None,
            SurfaceStatus::Pending,
        )
        .await
        .unwrap();
        surface.id
    }

    // Scenario: A status transition dispatches exactly one event carrying the
    // post-transition status and the owning session/workspace ids.
    #[tokio::test]
    async fn transition_pushes_one_event_with_post_transition_status() {
        let cx = ctx().await;
        let surface_id = seed_surface(&cx).await;

        let sink = Arc::new(RecordingSink(Mutex::new(Vec::new())));
        cx.domain_channel_sinks()
            .register("surface-status://test-window", sink.clone());

        update_status_and_emit(&cx, &surface_id, SurfaceStatus::Live)
            .await
            .unwrap();

        let events = sink.0.lock().unwrap();
        assert_eq!(events.len(), 1, "exactly one event per transition");
        let wire: serde_json::Value = serde_json::from_slice(&events[0]).unwrap();
        assert_eq!(wire["surfaceId"], surface_id.as_str());
        assert_eq!(wire["sessionId"], "s-ev-1");
        assert_eq!(wire["workspaceId"], "ws-ev-1");
        assert_eq!(wire["status"], "live");
    }

    // Scenario: The event fires after the status write commits -- a receiver's
    // re-query reads the new status.
    #[tokio::test]
    async fn status_is_already_persisted_when_the_event_fires() {
        let cx = ctx().await;
        let surface_id = seed_surface(&cx).await;

        cx.domain_channel_sinks().register(
            "surface-status://probe",
            Arc::new(RecordingSink(Mutex::new(Vec::new()))),
        );

        update_status_and_emit(&cx, &surface_id, SurfaceStatus::Failed)
            .await
            .unwrap();

        let (status,): (String,) = sqlx::query_as("SELECT status FROM surface WHERE id = ?")
            .bind(surface_id.as_str())
            .fetch_one(cx.db())
            .await
            .unwrap();
        assert_eq!(status, "failed");
    }

    // Scenario: A spawn confirmation loses to an exit frame that already landed --
    // a dead surface is never resurrected as live.
    #[tokio::test]
    async fn confirm_spawn_does_not_overwrite_a_terminal_exit() {
        let cx = ctx().await;
        let surface_id = seed_surface(&cx).await;

        // The PTY died immediately: the pump recorded failed before the spawn
        // command's confirmation ran.
        update_status_and_emit(&cx, &surface_id, SurfaceStatus::Failed)
            .await
            .unwrap();

        let sink = Arc::new(RecordingSink(Mutex::new(Vec::new())));
        cx.domain_channel_sinks()
            .register("surface-status://confirm-race", sink.clone());

        confirm_spawn_and_emit(&cx, &surface_id).await.unwrap();

        let (status,): (String,) = sqlx::query_as("SELECT status FROM surface WHERE id = ?")
            .bind(surface_id.as_str())
            .fetch_one(cx.db())
            .await
            .unwrap();
        assert_eq!(status, "failed", "the terminal exit record must stand");
        assert!(
            sink.0.lock().unwrap().is_empty(),
            "no live push for a confirmation that did not transition"
        );
    }

    // Scenario: No subscriber, no error -- push is additive over the query.
    #[tokio::test]
    async fn transition_without_subscribers_still_persists() {
        let cx = ctx().await;
        let surface_id = seed_surface(&cx).await;

        update_status_and_emit(&cx, &surface_id, SurfaceStatus::Idle)
            .await
            .unwrap();

        let (status,): (String,) = sqlx::query_as("SELECT status FROM surface WHERE id = ?")
            .bind(surface_id.as_str())
            .fetch_one(cx.db())
            .await
            .unwrap();
        assert_eq!(status, "idle");
    }
}
