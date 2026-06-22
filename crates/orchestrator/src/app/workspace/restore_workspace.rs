use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::workspace::{WorkspaceId, WorkspaceStatus};
use crate::infra::WorkspaceRepo;
use crate::shared::message::Command;
use crate::shared::{Error, Result};

/// Restore an archived workspace. Rejected if it is not archived.
/// Single-write (workspace row only; children are not auto-restored).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreWorkspace {
    pub id: String,
}

impl Command<Ctx> for RestoreWorkspace {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = WorkspaceId::new(&self.id);
        let mut ws = WorkspaceRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::WorkspaceNotFound(self.id.clone()))?;
        ws.guard_archived()?;
        ws.status = WorkspaceStatus::Active;
        WorkspaceRepo::update(cx.db(), &ws).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::workspace::archive_workspace::ArchiveWorkspace;
    use crate::app::workspace::test_util::*;
    use crate::infra::WorkspaceRepo;

    // Scenario: An archived entity is restored.
    #[tokio::test]
    async fn restore_workspace_makes_it_active_again() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-restore-1", "Restorable").await;
        ArchiveWorkspace {
            id: "ws-restore-1".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();

        RestoreWorkspace {
            id: "ws-restore-1".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();

        let ws = WorkspaceRepo::get(cx.db(), &ws_id("ws-restore-1"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ws.status, WorkspaceStatus::Active);
    }

    // Scenario: Restore targets only archived entities.
    #[tokio::test]
    async fn restore_active_workspace_is_rejected() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-restore-active", "Active").await;
        let err = RestoreWorkspace {
            id: "ws-restore-active".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap_err();
        assert_eq!(err.code(), "workspace.not_archived");
    }
}
