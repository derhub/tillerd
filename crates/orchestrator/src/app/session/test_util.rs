use std::path::PathBuf;
use std::sync::Arc;

use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::entities::session::SessionId;
use crate::infra::daemon_pty_api::{FakeRuntime, Runtime};
use crate::infra::migrate;
use crate::shared::bus::Bus;
use crate::shared::kv::SqliteKv;

use super::list_sessions_by_project::ListSessionsByProject;
use super::new_session_cmd::NewSessionCmd;

pub(crate) async fn ctx() -> (Bus<Ctx>, sqlx::SqlitePool) {
    let pool = migrate::open_memory().await.unwrap();
    let kv = SqliteKv::in_memory().await.unwrap();
    let cx = Ctx::new(
        pool.clone(),
        kv,
        PathBuf::from("/tmp/session-ops-test"),
        Runtime::Fake(Arc::new(FakeRuntime::new())),
    );
    (Bus::new(cx), pool)
}

pub(crate) fn unfiled() -> ProjectId {
    ProjectId::new("00000000-0000-0000-0000-000000000000")
}

/// Build a `NewSessionCmd` with a custom-titled session in the given project.
pub(crate) fn draft_cmd(pid: ProjectId) -> NewSessionCmd {
    NewSessionCmd {
        id: SessionId::mint(),
        project_id: Some(pid.as_str().to_owned()),
        title_source: "custom".to_owned(),
        title: Some("My session".to_owned()),
        template_id: None,
    }
}

/// Create one session and return its id (as the primitive `String` the View carries).
pub(crate) async fn create_one(bus: &Bus<Ctx>) -> String {
    bus.execute(draft_cmd(unfiled())).await.unwrap();
    let listing = bus
        .query(ListSessionsByProject {
            project_id: unfiled().as_str().to_owned(),
            limit: None,
            offset: None,
            after: None,
        })
        .await
        .unwrap();
    listing.items.into_iter().last().unwrap().id
}
