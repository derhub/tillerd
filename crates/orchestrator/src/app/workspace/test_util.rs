use std::path::PathBuf;
use std::sync::Arc;

use crate::context::Ctx;
use crate::entities::workspace::{NewWorkspace, WorkspaceId};
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
    WorkspaceRepo::create(
        cx.db(),
        &NewWorkspace {
            name: name.to_owned(),
        },
        &ws_id(id),
    )
    .await
    .unwrap();
}
