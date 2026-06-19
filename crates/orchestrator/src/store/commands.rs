//! Command library store.

use crate::entities::{Command, NewCommand};
use crate::error::Result;
use crate::store::backend::Backend;

/// Operational store for the command library.
#[derive(Clone)]
pub struct Commands {
    backend: Backend,
}

impl Commands {
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }

    pub async fn list(&self) -> Result<Vec<Command>> {
        self.backend.list_commands().await
    }

    pub async fn get(&self, id: String) -> Result<Option<Command>> {
        self.backend.get_command(id).await
    }

    pub async fn create(&self, draft: NewCommand) -> Result<Command> {
        self.backend.create_command(draft).await
    }

    pub async fn delete(&self, id: String) -> Result<()> {
        self.backend.delete_command(id).await
    }

    pub async fn seed(&self) -> Result<()> {
        self.backend.seed_commands().await
    }
}
