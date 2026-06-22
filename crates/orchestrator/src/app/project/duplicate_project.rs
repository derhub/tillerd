use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::{Project, ProjectId};
use crate::infra::project::ProjectRepo;
use crate::infra::session::SessionRepo;
use crate::shared::pagination::Page;
use crate::shared::{Command, Error, Result};

/// Clone a project (sessions + launch specs). The copy is independent of the source.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateProject {
    pub source_id: String,
    /// Name for the duplicate; defaults to `"Copy of <source name>"`.
    pub name: Option<String>,
}

impl Command<Ctx> for DuplicateProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        use crate::entities::session::{Session, SessionStatus, TitleSource};

        let source_id = ProjectId::new(&self.source_id);
        let source = ProjectRepo::get(cx.db(), &source_id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.source_id.clone()))?;

        let new_name = self
            .name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_owned())
            .unwrap_or_else(|| format!("Copy of {}", source.name));

        let new_project_id = ProjectId::new(uuid::Uuid::new_v4().to_string());

        cx.transaction(async |tx| {
            let new_project = Project::new(
                new_project_id.clone(),
                source.workspace_id.clone(),
                &new_name,
                source.source_kind,
                source.root_path.clone(),
                source.sort_order,
            );
            ProjectRepo::create(&mut **tx, &new_project).await?;

            // Clone sessions (with their spec blobs).
            let sessions = SessionRepo::list(&mut **tx, &source_id, Page::All).await?;
            for s in sessions.items {
                let new_session = Session {
                    id: crate::entities::session::SessionId::mint(),
                    project_id: new_project_id.clone(),
                    title: s.title.clone(),
                    title_source: TitleSource::Custom,
                    created_at: crate::shared::datetime::now_iso8601(),
                    spec_version: s.spec_version,
                    spec_json: s.spec_json.clone(),
                    sort_order: s.sort_order,
                    pinned: false,
                    status: SessionStatus::Active,
                };
                SessionRepo::create(&mut **tx, &new_session).await?;
            }
            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    use super::super::get_project_by_id::GetProjectById;
    use super::super::list_projects_by_workspace::ListProjectsByWorkspace;
    use super::super::rename_project::RenameProject;

    #[tokio::test]
    async fn duplicate_project_creates_independent_copy() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-src", "Source", &default_ws()).await;

        // Seed a session under the source project.
        sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
            .bind("s-src-1")
            .bind("p-src")
            .bind("Session A")
            .execute(ctx.db())
            .await
            .unwrap();

        bus.execute(DuplicateProject {
            source_id: "p-src".to_owned(),
            name: Some("Copy".to_owned()),
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();

        let copy = listing.items.iter().find(|p| p.name == "Copy");
        assert!(copy.is_some(), "duplicate project must appear in listing");

        // The copy must be distinct from the source.
        let copy = copy.unwrap();
        assert_ne!(copy.id, "p-src");
    }

    #[tokio::test]
    async fn mutating_copy_does_not_affect_source() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-orig", "Original", &default_ws()).await;

        bus.execute(DuplicateProject {
            source_id: "p-orig".to_owned(),
            name: Some("Clone".to_owned()),
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();

        let clone_id = listing
            .items
            .iter()
            .find(|p| p.name == "Clone")
            .map(|p| p.id.clone())
            .unwrap();

        bus.execute(RenameProject {
            id: clone_id,
            name: "Renamed Clone".to_owned(),
        })
        .await
        .unwrap();

        let source = bus
            .query(GetProjectById {
                id: "p-orig".to_owned(),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(source.name, "Original", "source must not be affected");
    }
}
