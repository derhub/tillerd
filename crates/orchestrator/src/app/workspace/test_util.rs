use std::path::PathBuf;
use std::sync::Arc;

use crate::context::Ctx;
use crate::entities::workspace::{Workspace, WorkspaceId, WorkspaceStatus};
use crate::infra::migrate;
use crate::infra::runtime::FakeRuntime;
use crate::infra::WorkspaceRepo;
use crate::shared::kv::SqliteKv;

pub(crate) async fn ctx() -> Ctx {
    let pool = migrate::open_memory().await.unwrap();
    let kv = SqliteKv::in_memory().await.unwrap();
    Ctx::new(
        pool,
        kv,
        PathBuf::from("/tmp/test"),
        Arc::new(FakeRuntime::new()),
    )
}

pub(crate) fn ws_id(s: &str) -> WorkspaceId {
    WorkspaceId::new(s)
}

pub(crate) async fn insert_workspace(cx: &Ctx, id: &str, name: &str) {
    let workspace = Workspace {
        id: WorkspaceId::new(id),
        name: name.to_owned(),
        sort_order: 0,
        pinned: false,
        status: WorkspaceStatus::Active,
    };
    WorkspaceRepo::create(cx.db(), &workspace).await.unwrap();
}
