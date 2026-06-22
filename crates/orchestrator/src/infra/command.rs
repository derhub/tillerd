use sqlx::SqliteExecutor;

use crate::entities::command::{Command, CommandId};
use crate::shared::Result;

// -- CommandRepo ----------------------------------------------------------------

/// Per-entity repository for the command library table.
///
/// All methods take an executor (`impl SqliteExecutor`) so the same code serves
/// a direct pool call and a multi-repo transaction without any structural change.
pub struct CommandRepo;

impl CommandRepo {
    /// Insert a command row from a fully-formed entity (caller mints the id).
    pub async fn create<'e>(exec: impl SqliteExecutor<'e>, cmd: &Command) -> Result<()> {
        let args_json = serde_json::to_string(&cmd.args)?;
        let env_json = serde_json::to_string(&cmd.env)?;
        let origin = cmd.origin.as_str();
        sqlx::query(
            "INSERT INTO command (id, name, origin, cli, args_json, env_json, pinned)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(cmd.id.as_str())
        .bind(&cmd.name)
        .bind(origin)
        .bind(&cmd.cli)
        .bind(&args_json)
        .bind(&env_json)
        .bind(cmd.pinned as i64)
        .execute(exec)
        .await?;
        Ok(())
    }

    /// Fetch a single command by id. Returns `None` if not found or soft-deleted.
    ///
    /// The `args`/`env` JSON columns are aliased to the entity's `#[sqlx(json)]`
    /// field names so the row decodes straight into the typed `Command`.
    pub async fn get<'e>(exec: impl SqliteExecutor<'e>, id: &CommandId) -> Result<Option<Command>> {
        Ok(sqlx::query_as::<_, Command>(
            "SELECT id, name, origin, cli, args_json AS args, env_json AS env, pinned
             FROM command
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id.as_str())
        .fetch_optional(exec)
        .await?)
    }

    /// Persist changes to an existing command (name, cli, args, env, pinned).
    pub async fn update<'e>(exec: impl SqliteExecutor<'e>, cmd: &Command) -> Result<()> {
        let args_json = serde_json::to_string(&cmd.args)?;
        let env_json = serde_json::to_string(&cmd.env)?;
        sqlx::query(
            "UPDATE command
             SET name = ?, cli = ?, args_json = ?, env_json = ?, pinned = ?
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(&cmd.name)
        .bind(&cmd.cli)
        .bind(&args_json)
        .bind(&env_json)
        .bind(cmd.pinned as i64)
        .bind(cmd.id.as_str())
        .execute(exec)
        .await?;
        Ok(())
    }

    /// Soft-delete a command by setting `deleted_at` to now.
    pub async fn delete<'e>(exec: impl SqliteExecutor<'e>, id: &CommandId) -> Result<()> {
        sqlx::query(
            "UPDATE command
             SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id.as_str())
        .execute(exec)
        .await?;
        Ok(())
    }

    /// Set the `pinned` flag for a command.
    pub async fn set_pinned<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &CommandId,
        pinned: bool,
    ) -> Result<()> {
        sqlx::query("UPDATE command SET pinned = ? WHERE id = ? AND deleted_at IS NULL")
            .bind(pinned as i64)
            .bind(id.as_str())
            .execute(exec)
            .await?;
        Ok(())
    }
}

// -- tests ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::entities::command::CommandOrigin;
    use crate::infra::migrate;

    fn custom(name: &str) -> Command {
        Command {
            id: CommandId::mint(),
            name: name.to_owned(),
            origin: CommandOrigin::Custom,
            cli: "/bin/bash".to_owned(),
            args: vec!["-c".to_owned()],
            env: HashMap::new(),
            pinned: false,
        }
    }

    // -- Scenario: round-trip --------------------------------------------------

    #[tokio::test]
    async fn create_then_get_returns_the_command() {
        let pool = migrate::open_memory().await.unwrap();
        let nc = custom("my-cmd");
        let id = nc.id.clone();
        CommandRepo::create(&pool, &nc).await.unwrap();
        let cmd = CommandRepo::get(&pool, &id).await.unwrap().unwrap();
        assert_eq!(cmd.name, "my-cmd");
        assert_eq!(cmd.origin, CommandOrigin::Custom);
        assert_eq!(cmd.cli, "/bin/bash");
        assert_eq!(cmd.args, vec!["-c"]);
        assert!(!cmd.pinned);
    }

    #[tokio::test]
    async fn get_absent_id_returns_none() {
        let pool = migrate::open_memory().await.unwrap();
        let missing = CommandId::from_string("does-not-exist");
        let result = CommandRepo::get(&pool, &missing).await.unwrap();
        assert!(result.is_none());
    }

    // -- Scenario: update ------------------------------------------------------

    #[tokio::test]
    async fn update_persists_name_and_cli_changes() {
        let pool = migrate::open_memory().await.unwrap();
        let nc = custom("orig");
        let id = nc.id.clone();
        CommandRepo::create(&pool, &nc).await.unwrap();
        let mut cmd = CommandRepo::get(&pool, &id).await.unwrap().unwrap();
        cmd.rename("updated");
        cmd.cli = "/usr/bin/fish".to_owned();
        CommandRepo::update(&pool, &cmd).await.unwrap();
        let reloaded = CommandRepo::get(&pool, &id).await.unwrap().unwrap();
        assert_eq!(reloaded.name, "updated");
        assert_eq!(reloaded.cli, "/usr/bin/fish");
    }

    // -- Scenario: delete (soft) -----------------------------------------------

    #[tokio::test]
    async fn delete_hides_the_command_from_get() {
        let pool = migrate::open_memory().await.unwrap();
        let nc = custom("gone");
        let id = nc.id.clone();
        CommandRepo::create(&pool, &nc).await.unwrap();
        CommandRepo::delete(&pool, &id).await.unwrap();
        let result = CommandRepo::get(&pool, &id).await.unwrap();
        assert!(result.is_none());
    }

    // -- Scenario: pinned flag round-trips -------------------------------------

    #[tokio::test]
    async fn set_pinned_updates_the_flag() {
        let pool = migrate::open_memory().await.unwrap();
        let nc = custom("pin-me");
        let id = nc.id.clone();
        CommandRepo::create(&pool, &nc).await.unwrap();
        CommandRepo::set_pinned(&pool, &id, true).await.unwrap();
        let cmd = CommandRepo::get(&pool, &id).await.unwrap().unwrap();
        assert!(cmd.pinned);
    }

    // -- Scenario: multi-repo call on one transaction is atomic ----------------

    #[tokio::test]
    async fn two_creates_on_one_transaction_are_both_committed_or_neither() {
        let pool = migrate::open_memory().await.unwrap();
        let a = custom("tx-a");
        let b = custom("tx-b");
        let id_a = a.id.clone();
        let id_b = b.id.clone();
        {
            let mut tx = pool.begin().await.unwrap();
            CommandRepo::create(&mut *tx, &a).await.unwrap();
            CommandRepo::create(&mut *tx, &b).await.unwrap();
            tx.commit().await.unwrap();
        }
        assert!(CommandRepo::get(&pool, &id_a).await.unwrap().is_some());
        assert!(CommandRepo::get(&pool, &id_b).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn transaction_rollback_leaves_no_rows() {
        let pool = migrate::open_memory().await.unwrap();
        let rolled = custom("rolled-back");
        let id = rolled.id.clone();
        {
            let mut tx = pool.begin().await.unwrap();
            CommandRepo::create(&mut *tx, &rolled).await.unwrap();
            tx.rollback().await.unwrap();
        }
        assert!(CommandRepo::get(&pool, &id).await.unwrap().is_none());
    }
}
