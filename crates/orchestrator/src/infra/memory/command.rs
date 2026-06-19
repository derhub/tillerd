use super::*;

impl MemoryBackend {
    pub(crate) fn list_commands(&self) -> Result<Vec<Command>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .commands
            .values()
            .filter(|r| !r.deleted)
            .map(|r| r.command.clone())
            .collect())
    }

    pub(crate) fn get_command(&self, id: &str) -> Result<Option<Command>> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .commands
            .get(id)
            .filter(|r| !r.deleted)
            .map(|r| r.command.clone()))
    }

    pub(crate) fn create_command(&self, draft: NewCommand) -> Result<Command> {
        let command = Command {
            id: CommandId::mint(),
            name: draft.name,
            origin: draft.origin,
            cli: draft.cli,
            args: draft.args,
            env: draft.env,
        };
        self.inner.lock().unwrap().commands.insert(
            command.id.as_str().to_string(),
            CommandRecord {
                command: command.clone(),
                deleted: false,
            },
        );
        Ok(command)
    }

    pub(crate) fn delete_command(&self, id: &str) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(r) = inner.commands.get_mut(id) {
            r.deleted = true;
        }
        Ok(())
    }

    pub(crate) fn seed_commands(&self) -> Result<()> {
        let seeds = prebuilt_commands_mem();
        let mut inner = self.inner.lock().unwrap();
        for cmd in seeds {
            inner
                .commands
                .entry(cmd.id.as_str().to_string())
                .or_insert(CommandRecord {
                    command: cmd,
                    deleted: false,
                });
        }
        Ok(())
    }
}
