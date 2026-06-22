use std::collections::HashMap;

use sqlx::{AssertSqlSafe, SqliteExecutor};

use crate::entities::command::{Command, CommandId, CommandOrigin, NewCommand};
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

// ── Row ────────────────────────────────────────────────────────────────────────

struct CommandRow {
    id: String,
    name: String,
    origin: String,
    cli: String,
    args_json: String,
    env_json: String,
    pinned: i64,
}

impl TryFrom<CommandRow> for Command {
    type Error = crate::shared::Error;

    fn try_from(row: CommandRow) -> Result<Self> {
        let origin = match row.origin.as_str() {
            "prebuilt" => CommandOrigin::Prebuilt,
            _ => CommandOrigin::Custom,
        };
        let args: Vec<String> = serde_json::from_str(&row.args_json)?;
        let env: HashMap<String, String> = serde_json::from_str(&row.env_json)?;
        Ok(Command {
            id: CommandId::from_string(row.id),
            name: row.name,
            origin,
            cli: row.cli,
            args,
            env,
            pinned: row.pinned != 0,
        })
    }
}

// ── CommandRepo ────────────────────────────────────────────────────────────────

/// Per-entity repository for the command library table.
///
/// All methods take an executor (`impl SqliteExecutor`) so the same code serves
/// a direct pool call and a multi-repo transaction without any structural change.
pub struct CommandRepo;

impl CommandRepo {
    /// Insert a new command row and return its assigned `CommandId`.
    pub async fn create<'e>(exec: impl SqliteExecutor<'e>, cmd: &NewCommand) -> Result<CommandId> {
        let id = CommandId::mint();
        let args_json = serde_json::to_string(&cmd.args)?;
        let env_json = serde_json::to_string(&cmd.env)?;
        let origin = cmd.origin.as_str();
        sqlx::query(
            "INSERT INTO command (id, name, origin, cli, args_json, env_json)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_str())
        .bind(&cmd.name)
        .bind(origin)
        .bind(&cmd.cli)
        .bind(&args_json)
        .bind(&env_json)
        .execute(exec)
        .await?;
        Ok(id)
    }

    /// Fetch a single command by id. Returns `None` if not found or soft-deleted.
    pub async fn get<'e>(exec: impl SqliteExecutor<'e>, id: &CommandId) -> Result<Option<Command>> {
        let row: Option<CommandRow> = sqlx::query_as(
            "SELECT id, name, origin, cli, args_json, env_json, pinned
             FROM command
             WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id.as_str())
        .fetch_optional(exec)
        .await?;
        row.map(Command::try_from).transpose()
    }

    /// List active (non-deleted) commands, pinned-first, then by sort_order.
    ///
    /// `origin_filter` restricts to `"prebuilt"` or `"custom"` when set.
    pub async fn list<'e>(
        exec: impl SqliteExecutor<'e>,
        origin_filter: Option<&str>,
        page: Page,
    ) -> Result<Listing<Command>> {
        // Build the query dynamically: the ORDER BY is always pinned DESC, sort_order ASC;
        // optional WHERE clause for origin.
        let base = "SELECT id, name, origin, cli, args_json, env_json, pinned
                    FROM command
                    WHERE deleted_at IS NULL";

        match page {
            Page::All => {
                let rows: Vec<CommandRow> = if let Some(origin) = origin_filter {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "{base} AND origin = ? ORDER BY pinned DESC, sort_order ASC"
                    )))
                    .bind(origin)
                    .fetch_all(exec)
                    .await?
                } else {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "{base} ORDER BY pinned DESC, sort_order ASC"
                    )))
                    .fetch_all(exec)
                    .await?
                };
                let items = rows
                    .into_iter()
                    .map(Command::try_from)
                    .collect::<Result<Vec<_>>>()?;
                Ok(Listing::new(items, None))
            }
            Page::Offset { limit, offset } => {
                let rows: Vec<CommandRow> = if let Some(origin) = origin_filter {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "{base} AND origin = ? ORDER BY pinned DESC, sort_order ASC LIMIT ? OFFSET ?"
                    )))
                    .bind(origin)
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(exec)
                    .await?
                } else {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "{base} ORDER BY pinned DESC, sort_order ASC LIMIT ? OFFSET ?"
                    )))
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(exec)
                    .await?
                };
                let items = rows
                    .into_iter()
                    .map(Command::try_from)
                    .collect::<Result<Vec<_>>>()?;
                Ok(Listing::new(items, None))
            }
            Page::Cursor { after, limit } => {
                // Fetch limit+1 to detect whether a next page exists without a
                // COUNT query. If we get more than limit rows back, a next page
                // exists; we truncate to limit before returning.
                let fetch_n = limit as i64 + 1;
                let rows: Vec<CommandRow> = if let Some(cursor) = after {
                    if let Some(origin) = origin_filter {
                        sqlx::query_as(AssertSqlSafe(format!(
                            "WITH anchor AS (
                                 SELECT pinned, sort_order FROM command WHERE id = ?
                             )
                             {base} AND origin = ?
                               AND (pinned < (SELECT pinned FROM anchor)
                                    OR (pinned = (SELECT pinned FROM anchor)
                                        AND sort_order > (SELECT sort_order FROM anchor))
                                    OR (pinned = (SELECT pinned FROM anchor)
                                        AND sort_order = (SELECT sort_order FROM anchor)
                                        AND id > ?))
                             ORDER BY pinned DESC, sort_order ASC, id ASC
                             LIMIT ?"
                        )))
                        .bind(&cursor)
                        .bind(origin)
                        .bind(&cursor)
                        .bind(fetch_n)
                        .fetch_all(exec)
                        .await?
                    } else {
                        sqlx::query_as(AssertSqlSafe(format!(
                            "WITH anchor AS (
                                 SELECT pinned, sort_order FROM command WHERE id = ?
                             )
                             {base}
                               AND (pinned < (SELECT pinned FROM anchor)
                                    OR (pinned = (SELECT pinned FROM anchor)
                                        AND sort_order > (SELECT sort_order FROM anchor))
                                    OR (pinned = (SELECT pinned FROM anchor)
                                        AND sort_order = (SELECT sort_order FROM anchor)
                                        AND id > ?))
                             ORDER BY pinned DESC, sort_order ASC, id ASC
                             LIMIT ?"
                        )))
                        .bind(&cursor)
                        .bind(&cursor)
                        .bind(fetch_n)
                        .fetch_all(exec)
                        .await?
                    }
                } else {
                    // first page
                    if let Some(origin) = origin_filter {
                        sqlx::query_as(AssertSqlSafe(format!(
                            "{base} AND origin = ?
                             ORDER BY pinned DESC, sort_order ASC, id ASC
                             LIMIT ?"
                        )))
                        .bind(origin)
                        .bind(fetch_n)
                        .fetch_all(exec)
                        .await?
                    } else {
                        sqlx::query_as(AssertSqlSafe(format!(
                            "{base} ORDER BY pinned DESC, sort_order ASC, id ASC LIMIT ?"
                        )))
                        .bind(fetch_n)
                        .fetch_all(exec)
                        .await?
                    }
                };
                let has_more = rows.len() > limit as usize;
                let items: Vec<Command> = rows
                    .into_iter()
                    .take(limit as usize)
                    .map(Command::try_from)
                    .collect::<Result<Vec<_>>>()?;
                let next = if has_more {
                    items.last().map(|c| c.id.as_str().to_owned())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }
        }
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

// ── FromRow ────────────────────────────────────────────────────────────────────

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for CommandRow {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(CommandRow {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            origin: row.try_get("origin")?,
            cli: row.try_get("cli")?,
            args_json: row.try_get("args_json")?,
            env_json: row.try_get("env_json")?,
            pinned: row.try_get("pinned")?,
        })
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::migrate;

    fn new_custom(name: &str) -> NewCommand {
        NewCommand {
            name: name.to_owned(),
            origin: CommandOrigin::Custom,
            cli: "/bin/bash".to_owned(),
            args: vec!["-c".to_owned()],
            env: HashMap::new(),
        }
    }

    fn new_prebuilt(name: &str) -> NewCommand {
        NewCommand {
            name: name.to_owned(),
            origin: CommandOrigin::Prebuilt,
            cli: "/bin/sh".to_owned(),
            args: vec![],
            env: HashMap::new(),
        }
    }

    // ── Scenario: round-trip ──────────────────────────────────────────────────

    #[tokio::test]
    async fn create_then_get_returns_the_command() {
        let pool = migrate::open_memory().await.unwrap();
        let nc = new_custom("my-cmd");
        let id = CommandRepo::create(&pool, &nc).await.unwrap();
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

    // ── Scenario: update ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn update_persists_name_and_cli_changes() {
        let pool = migrate::open_memory().await.unwrap();
        let id = CommandRepo::create(&pool, &new_custom("orig"))
            .await
            .unwrap();
        let mut cmd = CommandRepo::get(&pool, &id).await.unwrap().unwrap();
        cmd.rename("updated");
        cmd.cli = "/usr/bin/fish".to_owned();
        CommandRepo::update(&pool, &cmd).await.unwrap();
        let reloaded = CommandRepo::get(&pool, &id).await.unwrap().unwrap();
        assert_eq!(reloaded.name, "updated");
        assert_eq!(reloaded.cli, "/usr/bin/fish");
    }

    // ── Scenario: delete (soft) ───────────────────────────────────────────────

    #[tokio::test]
    async fn delete_hides_the_command_from_get() {
        let pool = migrate::open_memory().await.unwrap();
        let id = CommandRepo::create(&pool, &new_custom("gone"))
            .await
            .unwrap();
        CommandRepo::delete(&pool, &id).await.unwrap();
        let result = CommandRepo::get(&pool, &id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn delete_hides_the_command_from_list() {
        let pool = migrate::open_memory().await.unwrap();
        let id = CommandRepo::create(&pool, &new_custom("gone"))
            .await
            .unwrap();
        CommandRepo::delete(&pool, &id).await.unwrap();
        let listing = CommandRepo::list(&pool, None, Page::All).await.unwrap();
        assert!(!listing.items.iter().any(|c| c.id == id));
    }

    // ── Scenario: pinned-first ordering ──────────────────────────────────────

    #[tokio::test]
    async fn list_returns_pinned_commands_before_unpinned() {
        let pool = migrate::open_memory().await.unwrap();
        let id_a = CommandRepo::create(&pool, &new_custom("alpha"))
            .await
            .unwrap();
        let id_b = CommandRepo::create(&pool, &new_custom("beta"))
            .await
            .unwrap();
        // pin beta — it should sort first
        CommandRepo::set_pinned(&pool, &id_b, true).await.unwrap();
        let listing = CommandRepo::list(&pool, None, Page::All).await.unwrap();
        let ids: Vec<&CommandId> = listing.items.iter().map(|c| &c.id).collect();
        let pos_a = ids.iter().position(|id| **id == id_a).unwrap();
        let pos_b = ids.iter().position(|id| **id == id_b).unwrap();
        assert!(pos_b < pos_a, "pinned beta must precede unpinned alpha");
    }

    // ── Scenario: origin filter ───────────────────────────────────────────────

    #[tokio::test]
    async fn list_filtered_by_origin_returns_only_matching_commands() {
        let pool = migrate::open_memory().await.unwrap();
        CommandRepo::create(&pool, &new_custom("c")).await.unwrap();
        CommandRepo::create(&pool, &new_prebuilt("p"))
            .await
            .unwrap();
        let custom_only = CommandRepo::list(&pool, Some("custom"), Page::All)
            .await
            .unwrap();
        assert!(custom_only
            .items
            .iter()
            .all(|c| c.origin == CommandOrigin::Custom));
        let prebuilt_only = CommandRepo::list(&pool, Some("prebuilt"), Page::All)
            .await
            .unwrap();
        assert!(prebuilt_only
            .items
            .iter()
            .all(|c| c.origin == CommandOrigin::Prebuilt));
    }

    // ── Scenario: pagination (Offset) ─────────────────────────────────────────

    #[tokio::test]
    async fn offset_page_returns_bounded_slice() {
        let pool = migrate::open_memory().await.unwrap();
        for i in 0..5u32 {
            CommandRepo::create(&pool, &new_custom(&format!("cmd-{i}")))
                .await
                .unwrap();
        }
        let page1 = CommandRepo::list(&pool, None, Page::offset(2, 0))
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        let page2 = CommandRepo::list(&pool, None, Page::offset(2, 2))
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 2);
        // no overlap
        let ids1: Vec<&CommandId> = page1.items.iter().map(|c| &c.id).collect();
        let ids2: Vec<&CommandId> = page2.items.iter().map(|c| &c.id).collect();
        assert!(ids1.iter().all(|id| !ids2.contains(id)));
    }

    // ── Scenario: pagination (Cursor) ─────────────────────────────────────────

    #[tokio::test]
    async fn cursor_page_returns_continuation_and_terminates() {
        let pool = migrate::open_memory().await.unwrap();
        // create 4 custom commands (the seeded prebuilt is in the table too; use origin filter)
        for i in 0..4u32 {
            CommandRepo::create(&pool, &new_custom(&format!("cmd-{i}")))
                .await
                .unwrap();
        }
        let p1 = CommandRepo::list(&pool, Some("custom"), Page::cursor_from_start(2))
            .await
            .unwrap();
        assert_eq!(p1.items.len(), 2);
        assert!(
            p1.next.is_some(),
            "cursor page 1 should have a continuation"
        );
        let p2 = CommandRepo::list(
            &pool,
            Some("custom"),
            Page::cursor_after(p1.next.unwrap(), 2),
        )
        .await
        .unwrap();
        assert_eq!(p2.items.len(), 2);
        assert!(p2.next.is_none(), "cursor page 2 should be the last page");
        // no overlap
        let ids1: Vec<&CommandId> = p1.items.iter().map(|c| &c.id).collect();
        let ids2: Vec<&CommandId> = p2.items.iter().map(|c| &c.id).collect();
        assert!(ids1.iter().all(|id| !ids2.contains(id)));
    }

    // ── Scenario: multi-repo call on one transaction is atomic ────────────────

    #[tokio::test]
    async fn two_creates_on_one_transaction_are_both_committed_or_neither() {
        let pool = migrate::open_memory().await.unwrap();
        {
            let mut tx = pool.begin().await.unwrap();
            CommandRepo::create(&mut *tx, &new_custom("tx-a"))
                .await
                .unwrap();
            CommandRepo::create(&mut *tx, &new_custom("tx-b"))
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
        let listing = CommandRepo::list(&pool, Some("custom"), Page::All)
            .await
            .unwrap();
        let names: Vec<&str> = listing.items.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"tx-a"));
        assert!(names.contains(&"tx-b"));
    }

    #[tokio::test]
    async fn transaction_rollback_leaves_no_rows() {
        let pool = migrate::open_memory().await.unwrap();
        let before = CommandRepo::list(&pool, Some("custom"), Page::All)
            .await
            .unwrap()
            .items
            .len();
        {
            let mut tx = pool.begin().await.unwrap();
            CommandRepo::create(&mut *tx, &new_custom("rolled-back"))
                .await
                .unwrap();
            tx.rollback().await.unwrap();
        }
        let after = CommandRepo::list(&pool, Some("custom"), Page::All)
            .await
            .unwrap()
            .items
            .len();
        assert_eq!(after, before, "rollback must leave row count unchanged");
    }
}
