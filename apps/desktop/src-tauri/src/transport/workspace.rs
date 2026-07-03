use orchestrator::app::workspace::{
    ArchiveWorkspace, DiscardWorkspace, GetWorkspaceById, ListWorkspaceActivity, ListWorkspaces,
    NewWorkspaceCmd, PinWorkspace, RenameWorkspace, ReorderWorkspace, RestoreWorkspace,
    StopWorkspaceSurfaces, UnpinWorkspace, WorkspaceActivityView, WorkspaceView,
};
use uuid::Uuid;

use crate::transport::macros::{transport_command, transport_create, transport_query};

transport_create!(
    workspace_create(name: String) -> WorkspaceView {
        let id = Uuid::new_v4().to_string();
        execute: NewWorkspaceCmd { id: id.clone(), name },
        read_back: GetWorkspaceById { id },
        map: |workspace| workspace,
        missing: "workspace vanished after create",
    }
);

transport_query!(
    workspace_list() -> Vec<WorkspaceView>
        => ListWorkspaces { limit: None, offset: None, after: None },
        |listing| listing.items
);

transport_query!(
    workspace_activity() -> Vec<WorkspaceActivityView>
        => ListWorkspaceActivity {},
        |items| items
);

transport_command!(workspace_rename(id: String, name: String) => RenameWorkspace {
    id,
    name,
});

transport_command!(workspace_reorder(id: String, sort_order: u32) => ReorderWorkspace {
    id,
    sort_order,
});

transport_command!(workspace_delete(id: String) => DiscardWorkspace { id });

transport_query!(
    workspace_get(id: String) -> Option<WorkspaceView>
        => GetWorkspaceById { id },
        |workspace| workspace
);

transport_command!(workspace_archive(id: String) => ArchiveWorkspace { id });

transport_command!(workspace_restore(id: String) => RestoreWorkspace { id });

transport_command!(workspace_pin(id: String) => PinWorkspace { id });

transport_command!(workspace_unpin(id: String) => UnpinWorkspace { id });

transport_command!(workspace_stop_surfaces(id: String) => StopWorkspaceSurfaces { id });

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
    fn workspace_response_matches_sdk_workspace_shape() {
        let w = WorkspaceView {
            id: "w".into(),
            name: "W".into(),
        };
        assert_keys(&serde_json::to_value(w).unwrap(), &["id", "name"]);
    }
}
