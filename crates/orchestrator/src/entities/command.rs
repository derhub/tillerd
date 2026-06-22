//! Command library entity: a named CLI invocation (prebuilt or custom).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

    #[test]
    fn rename_trims_whitespace() {
        let mut cmd = custom_command("cmd-1");
        cmd.rename("  new name  ");
        assert_eq!(cmd.name, "new name");
    }
}
