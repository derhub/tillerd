use std::path::PathBuf;
use std::sync::Arc;

use crate::context::Ctx;
use crate::entities::session::{NewSession, SessionId, TitleSource};
use crate::infra::migrate;
use crate::infra::runtime::FakeRuntime;
use crate::shared::bus::Bus;
use crate::shared::kv::SqliteKv;
use crate::shared::pagination::Page;

use super::list_sessions_by_project::ListSessionsByProject;
use super::new_session_cmd::NewSessionCmd;

pub(crate) async fn ctx() -> (Bus<Ctx>, sqlx::SqlitePool) {
    let pool = migrate::open_memory().await.unwrap();
    let kv = SqliteKv::in_memory().await.unwrap();
    let cx = Ctx::new(
        pool.clone(),
        kv,
        PathBuf::from("/tmp/session-ops-test"),
        Arc::new(FakeRuntime::new()),
    );
    (Bus::new(cx), pool)
}

pub(crate) fn unfiled() -> crate::entities::project::ProjectId {
    crate::entities::project::ProjectId::new("00000000-0000-0000-0000-000000000000")
}

pub(crate) fn draft(pid: crate::entities::project::ProjectId) -> NewSession {
    NewSession {
        project_id: Some(pid),
        title_source: TitleSource::Custom,
        title: Some("My session".to_owned()),
        template_id: None,
    }
}

pub(crate) async fn create_one(bus: &Bus<Ctx>) -> SessionId {
    bus.execute(NewSessionCmd(draft(unfiled()))).await.unwrap();
    let listing = bus
        .query(ListSessionsByProject {
            project_id: unfiled(),
            page: Page::All,
        })
        .await
        .unwrap();
    listing.items.into_iter().last().unwrap().id
}
