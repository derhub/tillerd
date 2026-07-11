use std::collections::HashMap;

use orchestrator::app::command::{
    CommandView, DiscardCommand, DuplicateCommand, EditCommand, GetCommandById, ListCommands,
    NewCommand as NewCommandCmd, PinCommand, RenameCommand, SeedCommands, UnpinCommand,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::transport::macros::{transport_command, transport_create, transport_query};

transport_command!(command_rename(id: String, name: String) => RenameCommand { id, name });

transport_command!(
    command_edit(id: String, cli: String, args: Vec<String>, env: HashMap<String, String>)
        => EditCommand { id, cli, args, env }
);

transport_command!(command_pin(id: String) => PinCommand { id });

transport_command!(command_unpin(id: String) => UnpinCommand { id });

transport_command!(command_duplicate(id: String, name: String) => DuplicateCommand { id, name });

transport_command!(command_seed() => SeedCommands);

transport_query!(
    command_get(id: String) -> Option<CommandView>
        => GetCommandById { id },
        |cmd| cmd
);

transport_command!(command_delete(id: String) => DiscardCommand { id });

transport_query!(
    command_list() -> Vec<CommandView>
        => ListCommands { origin: None, limit: None, offset: None, after: None },
        |listing| listing.items
);

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommandRequest {
    pub name: String,
    pub cli: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

transport_create!(
    command_create(req: CreateCommandRequest) -> CommandView {
        let id = Uuid::new_v4().to_string();
        execute: NewCommandCmd {
            id: id.clone(),
            name: req.name,
            cli: req.cli,
            args: req.args,
            env: req.env,
        },
        read_back: GetCommandById { id: id.clone() },
        map: |cmd| cmd,
        missing: "command vanished after create",
    }
);

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
    fn command_response_matches_sdk_command_shape() {
        let c = CommandView {
            id: "c".into(),
            name: "c".into(),
            origin: "custom".into(),
            cli: "/c".into(),
            args: vec![],
            env: Default::default(),
            pinned: false,
        };
        assert_keys(
            &serde_json::to_value(c).unwrap(),
            &["id", "name", "origin", "cli", "args", "env", "pinned"],
        );
    }
}
