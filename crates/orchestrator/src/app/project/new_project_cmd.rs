use crate::context::Ctx;
use crate::entities::project::NewProject;
use crate::entities::workspace::WorkspaceId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Result};

use super::common::{infer_name, new_id};

/// Create a new project in a workspace (defaulting to the Default workspace).
pub struct NewProjectCmd {
    pub params: NewProject,
}

impl Command<Ctx> for NewProjectCmd {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = new_id();
        let workspace_id = self
            .params
            .workspace_id
            .clone()
            .unwrap_or_else(WorkspaceId::default_id);
        let name = infer_name(&self.params);
        ProjectRepo::create(
            cx.db(),
            &id,
            &workspace_id,
            &name,
            self.params.source_kind,
            self.params.root_path.as_deref(),
            0,
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;
    use crate::entities::project::SourceKind;
    use crate::shared::pagination::Page;

    use super::super::list_projects_by_workspace::ListProjectsByWorkspace;

    #[tokio::test]
    async fn new_project_creates_project_in_workspace() {
        let (_ctx, bus) = ctx().await;
        bus.execute(NewProjectCmd {
            params: NewProject {
                source_kind: SourceKind::Blank,
                root_path: None,
                name: Some("Alpha".to_owned()),
                workspace_id: Some(default_ws()),
            },
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws(),
                page: Page::All,
            })
            .await
            .unwrap();

        let found = listing.items.iter().any(|p| p.name == "Alpha");
        assert!(found, "new project must appear in workspace listing");
    }

    #[tokio::test]
    async fn new_project_returns_nothing() {
        let (_ctx, bus) = ctx().await;
        let result = bus
            .execute(NewProjectCmd {
                params: NewProject {
                    source_kind: SourceKind::Blank,
                    root_path: None,
                    name: Some("Beta".to_owned()),
                    workspace_id: None,
                },
            })
            .await;
        // Command::handle returns Result<()>, not data.
        assert!(result.is_ok());
    }
}
