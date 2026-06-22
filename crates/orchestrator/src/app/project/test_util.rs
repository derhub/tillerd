#![cfg(test)]

use std::path::PathBuf;
use std::sync::Arc;

use crate::context::Ctx;
use crate::entities::project::{Project, ProjectId, SourceKind};
use crate::entities::workspace::WorkspaceId;
use crate::infra::daemon_pty_api::{FakeRuntime, Runtime};
use crate::infra::migrate;
use crate::infra::project::ProjectRepo;
use crate::shared::kv::SqliteKv;

pub(crate) async fn ctx() -> (Ctx, crate::shared::Bus<Ctx>) {
    let pool = migrate::open_memory().await.expect("in-memory pool");
    let kv = SqliteKv::in_memory().await.expect("in-memory kv");
    let ctx = Ctx::new(
        pool,
        kv,
        PathBuf::from("/tmp/tillerd-test"),
        Runtime::Fake(Arc::new(FakeRuntime::new())),
    );
    let bus = crate::shared::Bus::new(ctx.clone());
    (ctx, bus)
}

pub(crate) fn default_ws() -> WorkspaceId {
    WorkspaceId::default_id()
}

pub(crate) fn unfiled_project_id() -> ProjectId {
    ProjectId::unfiled()
}

// Create a project directly via the repo (for test setup).
pub(crate) async fn seed_project(
    pool: &sqlx::SqlitePool,
    id: &str,
    name: &str,
    ws: &WorkspaceId,
) -> Project {
    ProjectRepo::create(pool, id, ws, name, SourceKind::Blank, None, 0)
        .await
        .expect("seed project")
}

// Create a project with an explicit sort_order (for ordering/pagination tests).
pub(crate) async fn seed_project_full(
    pool: &sqlx::SqlitePool,
    id: &str,
    name: &str,
    ws: &WorkspaceId,
    sort_order: u32,
) -> Project {
    ProjectRepo::create(pool, id, ws, name, SourceKind::Blank, None, sort_order)
        .await
        .expect("seed project")
}
