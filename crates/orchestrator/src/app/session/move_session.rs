use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

/// Move a session to another project by updating its `project_id`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveSession {
    pub id: String,
    pub target_project_id: String,
}

impl Command<Ctx> for MoveSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        use crate::infra::ProjectRepo;
        let id = SessionId::from_string(&self.id);
        let target_project_id = ProjectId::new(&self.target_project_id);
        let mut s = SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;
        ProjectRepo::get(cx.db(), &target_project_id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.target_project_id.clone()))?;
        s.project_id = target_project_id;
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::list_sessions_by_project::ListSessionsByProject;
    use crate::app::session::test_util::{create_one, ctx, unfiled};

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

        bus.execute(MoveSession {
            id: id.clone(),
            target_project_id: "proj-b".to_owned(),
        })
        .await
        .unwrap();

        let s = bus.query(GetSessionById { id }).await.unwrap().unwrap();
        assert_eq!(s.project_id, "proj-b");

        let unfiled_list = bus
            .query(ListSessionsByProject {
                project_id: unfiled().as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert!(unfiled_list.items.is_empty());

        let proj_b_list = bus
            .query(ListSessionsByProject {
                project_id: "proj-b".to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(proj_b_list.items.len(), 1);
    }
}
