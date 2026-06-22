use std::sync::Arc;

use sqlx::SqlitePool;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::entities::{Surface, SurfaceKind};
use crate::infra::migrate;
use crate::infra::runtime::FakeRuntime;
use crate::shared::kv::SqliteKv;
use crate::shared::pagination::Page;
use crate::shared::Bus;

use super::list_surfaces_by_session::ListSurfacesBySession;
use super::spawn_surface::SpawnSurface;

/// A test context plus a handle to its fake runtime for assertions.
pub(crate) struct Harness {
    pub(crate) bus: Bus<Ctx>,
    pub(crate) runtime: Arc<FakeRuntime>,
    pub(crate) pool: SqlitePool,
}

pub(crate) async fn harness() -> Harness {
    let pool = migrate::open_memory().await.unwrap();
    let kv = SqliteKv::in_memory().await.unwrap();
    let runtime = Arc::new(FakeRuntime::new());
    let cx = Ctx::new(
        pool.clone(),
        kv,
        std::path::PathBuf::from("/tmp/tillerd-test"),
        runtime.clone(),
    );
    Harness {
        bus: Bus::new(cx),
        runtime,
        pool,
    }
}

pub(crate) async fn seed_session(pool: &SqlitePool, id: &str) -> SessionId {
    sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
        .bind(id)
        .bind("00000000-0000-0000-0000-000000000000")
        .bind("test")
        .execute(pool)
        .await
        .expect("seed session");
    SessionId::from_string(id)
}

pub(crate) fn spawn(session: &SessionId) -> SpawnSurface {
    SpawnSurface {
        session: session.clone(),
        kind: SurfaceKind::Terminal,
        cwd: Some("/work".to_owned()),
        placement: Some("main".to_owned()),
        command: None,
        geometry: None,
    }
}

pub(crate) async fn one_surface(h: &Harness, session: &SessionId) -> Surface {
    h.bus
        .query(ListSurfacesBySession {
            session: session.clone(),
            page: Page::All,
        })
        .await
        .unwrap()
        .items
        .into_iter()
        .next()
        .expect("a surface")
}
