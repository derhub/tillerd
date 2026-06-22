use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::{Error, Result};

/// Move a session to another project by updating its `project_id`.
pub struct MoveSession {
    pub id: SessionId,
    pub target_project_id: crate::entities::project::ProjectId,
}

impl Command<Ctx> for MoveSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        use crate::infra::ProjectRepo;
        let mut s = SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;
        ProjectRepo::get(cx.db(), &self.target_project_id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.target_project_id.as_str().to_owned()))?;
        s.project_id = self.target_project_id.clone();
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::list_sessions_by_project::ListSessionsByProject;
    use crate::app::session::test_util::{create_one, ctx, unfiled};
    use crate::shared::pagination::Page;

    // Scenario: A move reparents by update
    #[tokio::test]
    async fn move_session_reparents_to_target_project() {
        let (bus, pool) = ctx().await;
        let id = create_one(&bus).await;

        sqlx::query("INSERT INTO project (id, workspace_id, name) VALUES (?, ?, ?)")
            .bind("proj-b")
            .bind("00000000-0000-0000-0000-000000000001")
            .bind("Project B")
            .execute(&pool)
            .await
            .unwrap();

        let proj_b = crate::entities::project::ProjectId::new("proj-b");
        bus.execute(MoveSession {
            id: id.clone(),
            target_project_id: proj_b.clone(),
        })
        .await
        .unwrap();

        let s = bus.query(GetSessionById { id }).await.unwrap().unwrap();
        assert_eq!(s.project_id.as_str(), "proj-b");

        let unfiled_list = bus
            .query(ListSessionsByProject {
                project_id: unfiled(),
                page: Page::All,
            })
            .await
            .unwrap();
        assert!(unfiled_list.items.is_empty());

        let proj_b_list = bus
            .query(ListSessionsByProject {
                project_id: proj_b,
                page: Page::All,
            })
            .await
            .unwrap();
        assert_eq!(proj_b_list.items.len(), 1);
    }
}
