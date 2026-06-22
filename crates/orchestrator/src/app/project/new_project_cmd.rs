use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::SourceKind;
use crate::entities::workspace::WorkspaceId;
use crate::infra::project::ProjectRepo;
use crate::shared::{Command, Result};

use super::common::{infer_name, new_id};

/// Create a new project in a workspace (defaulting to the Default workspace).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProjectCmd {
    pub source_kind: String,
    pub root_path: Option<String>,
    pub name: Option<String>,
    pub workspace_id: Option<String>,
}

impl Command<Ctx> for NewProjectCmd {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = new_id();
        let workspace_id = self
            .workspace_id
            .clone()
            .map(WorkspaceId::new)
            .unwrap_or_else(WorkspaceId::default_id);
        let source_kind = match self.source_kind.as_str() {
            "local_dir" => SourceKind::LocalDir,
            "git_repo" => SourceKind::GitRepo,
            _ => SourceKind::Blank,
        };
        let name = infer_name(self.name.as_deref(), self.root_path.as_deref());
        ProjectRepo::create(
            cx.db(),
            &id,
            &workspace_id,
            &name,
            source_kind,
            self.root_path.as_deref(),
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

    use super::super::list_projects_by_workspace::ListProjectsByWorkspace;

    #[tokio::test]
    async fn new_project_creates_project_in_workspace() {
        let (_ctx, bus) = ctx().await;
        bus.execute(NewProjectCmd {
            source_kind: "blank".to_owned(),
            root_path: None,
            name: Some("Alpha".to_owned()),
            workspace_id: Some(default_ws().as_str().to_owned()),
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

        let found = listing.items.iter().any(|p| p.name == "Alpha");
        assert!(found, "new project must appear in workspace listing");
    }

    #[tokio::test]
    async fn new_project_returns_nothing() {
        let (_ctx, bus) = ctx().await;
        let result = bus
            .execute(NewProjectCmd {
                source_kind: "blank".to_owned(),
                root_path: None,
                name: Some("Beta".to_owned()),
                workspace_id: None,
            })
            .await;
        // Command::handle returns Result<()>, not data.
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn new_project_trims_name_whitespace() {
        let (_ctx, bus) = ctx().await;
        bus.execute(NewProjectCmd {
            source_kind: "blank".to_owned(),
            root_path: None,
            name: Some("  Gamma  ".to_owned()),
            workspace_id: Some(default_ws().as_str().to_owned()),
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

        let found = listing.items.iter().any(|p| p.name == "Gamma");
        assert!(found, "create must store and return the trimmed name");
    }
}
