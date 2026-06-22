use std::sync::Arc;

use sqlx::SqlitePool;

use crate::app::surface::SurfaceView;
use crate::context::Ctx;
use crate::infra::migrate;
use crate::infra::runtime::FakeRuntime;
use crate::shared::kv::SqliteKv;
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

/// Seed a session row and return its id as the primitive the surface DTOs carry.
pub(crate) async fn seed_session(pool: &SqlitePool, id: &str) -> String {
    sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
        .bind(id)
        .bind("00000000-0000-0000-0000-000000000000")
        .bind("test")
        .execute(pool)
        .await
        .expect("seed session");
    id.to_owned()
}

pub(crate) fn spawn(session: &str) -> SpawnSurface {
    SpawnSurface {
        session: session.to_owned(),
        kind: "terminal".to_owned(),
        cwd: Some("/work".to_owned()),
        placement: Some("main".to_owned()),
        cols: None,
        rows: None,
    }
}

pub(crate) async fn one_surface(h: &Harness, session: &str) -> SurfaceView {
    h.bus
        .query(ListSurfacesBySession {
            session: session.to_owned(),
            limit: None,
            offset: None,
            after: None,
        })
        .await
        .unwrap()
        .items
        .into_iter()
        .next()
        .expect("a surface")
}
