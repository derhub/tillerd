
use orchestrator::app::project::{
    ArchiveProject, DiscardProject, DuplicateProject, GetProjectById, ListProjectsByWorkspace,
    MoveProject, NewProjectCmd, PinProject, ProjectView, RenameProject, ReorderProject,
    RestoreProject, SearchProjects, StopProjectSurfaces, UnpinProject,
};
use uuid::Uuid;

use crate::transport::macros::{transport_command, transport_create, transport_query};

transport_query!(
    project_list(workspace_id: Option<String>) -> Vec<ProjectView>
        => ListProjectsByWorkspace {
            workspace_id: workspace_id.unwrap_or_else(orchestrator::app::workspace::default_workspace_id),
            limit: None,
            offset: None,
            after: None,
        },
        |listing| listing.items
);

transport_command!(project_rename(id: String, name: String) => RenameProject {
    id,
    name,
});

transport_command!(project_archive(id: String) => ArchiveProject { id });

// Hard-delete a project. `DiscardProject` folds archive-then-discard internally
// (archives an active project, cascading its sessions, then purges).
transport_command!(project_delete(id: String) => DiscardProject { id });

transport_command!(project_reorder(id: String, sort_order: u32) => ReorderProject {
    id,
    sort_order,
});

transport_command!(project_move(id: String, workspace_id: String) => MoveProject {
    id,
    workspace_id,
});

transport_create!(
    project_create(name: Option<String>, workspace_id: Option<String>) -> ProjectView {
        let id = Uuid::new_v4().to_string();
        execute: NewProjectCmd {
            id: id.clone(),
            source_kind: "blank".to_string(),
            root_path: None,
            name,
            workspace_id: workspace_id
                .or_else(|| Some(orchestrator::app::workspace::default_workspace_id())),
        },
        read_back: GetProjectById { id: id.clone() },
        map: |project| project,
        missing: "project vanished after create",
    }
);

transport_query!(
    project_get(id: String) -> Option<ProjectView>
        => GetProjectById { id },
        |project| project
);

transport_query!(
    project_search(workspace_id: String, query: String, limit: u32) -> Vec<ProjectView>
        => SearchProjects { workspace_id, query, limit },
        |results| results
);

transport_command!(project_restore(id: String) => RestoreProject { id });

transport_command!(project_duplicate(source_id: String, name: Option<String>) => DuplicateProject {
    source_id,
    name,
});

transport_command!(project_pin(id: String) => PinProject { id });

transport_command!(project_unpin(id: String) => UnpinProject { id });

transport_command!(project_stop_surfaces(id: String) => StopProjectSurfaces { id });

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_keys(value: &serde_json::Value, expected: &[&str]) {
        let obj = value.as_object().expect("response serializes to an object");
        let mut got: Vec<&str> = obj.keys().map(String::as_str).collect();
        got.sort_unstable();
        let mut want = expected.to_vec();
        want.sort_unstable();
        assert_eq!(got, want, "response keys drifted from the SDK contract");
    }

    #[test]
    fn project_response_matches_sdk_project_shape() {
        let p = ProjectView {
            id: "p".into(),
            name: "P".into(),
            source_kind: "blank".into(),
            root_path: None,
            workspace_id: "w".into(),
        };
        assert_keys(
            &serde_json::to_value(p).unwrap(),
            &["id", "name", "sourceKind", "rootPath", "workspaceId"],
        );
    }
}
