#![cfg(test)]

use std::path::PathBuf;
use std::sync::Arc;

use crate::context::Ctx;
use crate::entities::project::{Project, ProjectId, SourceKind};
use crate::entities::workspace::WorkspaceId;
use crate::infra::migrate;
use crate::infra::project::ProjectRepo;
use crate::infra::runtime::FakeRuntime;
use crate::shared::kv::SqliteKv;

pub(crate) async fn ctx() -> (Ctx, crate::shared::Bus<Ctx>) {
    let pool = migrate::open_memory().await.expect("in-memory pool");
    let kv = SqliteKv::in_memory().await.expect("in-memory kv");
    let ctx = Ctx::new(
        pool,
        kv,
        PathBuf::from("/tmp/tillerd-test"),
        Arc::new(FakeRuntime::new()),
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
