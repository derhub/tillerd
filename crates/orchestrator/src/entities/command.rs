//! Command library entity: a named CLI invocation (prebuilt or custom).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::shared::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct CommandId(String);

impl CommandId {
    pub fn mint() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum CommandOrigin {
    Prebuilt,
    Custom,
}

impl CommandOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            CommandOrigin::Prebuilt => "prebuilt",
            CommandOrigin::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Command {
    pub id: CommandId,
    pub name: String,
    pub origin: CommandOrigin,
    pub cli: String,
    #[sqlx(json)]
    pub args: Vec<String>,
    #[sqlx(json)]
    pub env: HashMap<String, String>,
    pub pinned: bool,
}

impl Command {
    /// Guard: reject rename, edit, or discard on a Prebuilt command.
    pub fn guard_not_prebuilt(&self) -> Result<()> {
        if self.origin == CommandOrigin::Prebuilt {
            Err(Error::PrebuiltImmutable { kind: "command" })
        } else {
            Ok(())
        }
    }

    /// Rename the command. Trims whitespace.
    pub fn rename(&mut self, name: &str) {
        self.name = name.trim().to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom_command(id: &str) -> Command {
        Command {
            id: CommandId::from_string(id),
            name: "my-cmd".to_owned(),
            origin: CommandOrigin::Custom,
            cli: "/bin/bash".to_owned(),
            args: vec![],
            env: HashMap::new(),
            pinned: false,
        }
    }

    fn prebuilt_command(id: &str) -> Command {
        Command {
            origin: CommandOrigin::Prebuilt,
            ..custom_command(id)
        }
    }

    #[test]
    fn guard_not_prebuilt_allows_custom_command() {
        let cmd = custom_command("cmd-1");
        assert!(cmd.guard_not_prebuilt().is_ok());
    }

    #[test]
    fn guard_not_prebuilt_rejects_prebuilt_command() {
        let cmd = prebuilt_command("cmd-1");
        assert!(cmd.guard_not_prebuilt().is_err());
    }

    #[test]
    fn rename_trims_whitespace() {
        let mut cmd = custom_command("cmd-1");
        cmd.rename("  new name  ");
        assert_eq!(cmd.name, "new name");
    }
}
