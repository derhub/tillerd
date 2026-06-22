use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use sqlx::ConnectOptions;

use crate::shared::Result;

/// Migrator that embeds `infra/migrations/` at compile time.
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("src/infra/migrations");

/// Open an in-memory pool suitable for tests and apply all migrations.
///
/// Each call returns a fresh, isolated pool (`:memory:` URI). Pass the result
/// directly to helpers or construct a `Ctx` from it. `max_connections(1)`:
/// each `:memory:` connection is its own database; a pool of more than one
/// would give callers separate, empty databases.
pub async fn open_memory() -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    run(&pool).await?;
    Ok(pool)
}

/// Open a file-backed pool and apply all pending migrations.
pub async fn open_file(path: &std::path::Path) -> Result<SqlitePool> {
    let url = format!("sqlite://{}", path.display());
    let opts = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(5))
        .disable_statement_logging();

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await?;
    run(&pool).await?;
    Ok(pool)
}

/// Apply all pending migrations to an already-open pool.
pub async fn run(pool: &SqlitePool) -> Result<()> {
    MIGRATOR.run(pool).await.map_err(sqlx::Error::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    // Helpers ──────────────────────────────────────────────────────────────────

    async fn memory_pool() -> SqlitePool {
        open_memory().await.expect("in-memory pool failed")
    }

    fn table_list_sql() -> &'static str {
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
    }

    async fn table_names(pool: &SqlitePool) -> Vec<String> {
        sqlx::query(table_list_sql())
            .fetch_all(pool)
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.get::<String, _>("name"))
            .filter(|n| !n.starts_with('_')) // exclude _sqlx_migrations
            .collect()
    }

    // ── schema presence ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn all_domain_tables_exist_after_migration() {
        let pool = memory_pool().await;
        let tables = table_names(&pool).await;
        for expected in [
            "command",
            "kv",
            "launch_template",
            "notification",
            "project",
            "session",
            "surface",
            "workspace",
        ] {
            assert!(
                tables.contains(&expected.to_string()),
                "missing table: {expected}"
            );
        }
    }

    #[tokio::test]
    async fn migration_is_idempotent_on_repeated_run() {
        let pool = memory_pool().await;
        // A second run against an already-migrated pool must not error.
        run(&pool).await.expect("second migration run must succeed");
        let tables = table_names(&pool).await;
        // Tables must still be unique (no duplication).
        let mut sorted = tables.clone();
        sorted.dedup();
        assert_eq!(tables.len(), sorted.len());
    }

    // ── seed data ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn default_workspace_is_seeded() {
        let pool = memory_pool().await;
        let name: String = sqlx::query_scalar(
            "SELECT name FROM workspace WHERE id = '00000000-0000-0000-0000-000000000001'",
        )
        .fetch_one(&pool)
        .await
        .expect("Default workspace must be seeded");
        assert_eq!(name, "Default");
    }

    #[tokio::test]
    async fn unfiled_project_is_seeded_under_default_workspace() {
        let pool = memory_pool().await;
        let row = sqlx::query(
            "SELECT name, workspace_id FROM project WHERE id = '00000000-0000-0000-0000-000000000000'",
        )
        .fetch_one(&pool)
        .await
        .expect("Unfiled project must be seeded");
        assert_eq!(row.get::<String, _>("name"), "Unfiled");
        assert_eq!(
            row.get::<String, _>("workspace_id"),
            "00000000-0000-0000-0000-000000000001"
        );
    }

    #[tokio::test]
    async fn login_shell_command_is_seeded() {
        let pool = memory_pool().await;
        let name: String = sqlx::query_scalar(
            "SELECT name FROM command WHERE id = '00000000-0000-0000-0000-000000000101'",
        )
        .fetch_one(&pool)
        .await
        .expect("login-shell command must be seeded");
        assert_eq!(name, "login-shell");
    }

    // ── column presence: new fields required by task 0b ───────────────────────

    #[tokio::test]
    async fn workspace_has_sort_order_pinned_archived_at() {
        let pool = memory_pool().await;
        // Inserting with the new columns proves they exist; if any column is
        // absent SQLite returns an error and the test fails with a clear message.
        let id = "test-ws-col-check";
        sqlx::query(
            "INSERT INTO workspace (id, name, sort_order, pinned, archived_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind("ColCheck")
        .bind(5i64)
        .bind(1i64)
        .bind("2026-01-01T00:00:00.000Z")
        .execute(&pool)
        .await
        .expect("workspace must accept sort_order, pinned, archived_at");
    }

    #[tokio::test]
    async fn project_has_parent_id_sort_order_pinned_archived_at() {
        let pool = memory_pool().await;
        let default_ws = "00000000-0000-0000-0000-000000000001";
        sqlx::query(
            "INSERT INTO project (id, workspace_id, name, sort_order, pinned, archived_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("test-proj-col-check")
        .bind(default_ws)
        .bind("ColCheck")
        .bind(3i64)
        .bind(1i64)
        .bind("2026-01-01T00:00:00.000Z")
        .execute(&pool)
        .await
        .expect("project must accept workspace_id (parent_id), sort_order, pinned, archived_at");
    }

    #[tokio::test]
    async fn session_has_parent_id_sort_order_pinned_archived_at() {
        let pool = memory_pool().await;
        let unfiled = "00000000-0000-0000-0000-000000000000";
        sqlx::query(
            "INSERT INTO session (id, project_id, title, sort_order, pinned, archived_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("test-sess-col-check")
        .bind(unfiled)
        .bind("ColCheck")
        .bind(2i64)
        .bind(1i64)
        .bind("2026-01-01T00:00:00.000Z")
        .execute(&pool)
        .await
        .expect("session must accept project_id (parent_id), sort_order, pinned, archived_at");
    }

    #[tokio::test]
    async fn surface_has_parent_id_and_status() {
        let pool = memory_pool().await;
        let unfiled = "00000000-0000-0000-0000-000000000000";
        let sid = "test-sess-for-surf";
        sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
            .bind(sid)
            .bind(unfiled)
            .bind("s")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO surface (id, session_id, kind, status) VALUES (?, ?, ?, ?)")
            .bind("test-surf-col-check")
            .bind(sid)
            .bind("terminal")
            .bind("pending")
            .execute(&pool)
            .await
            .expect("surface must accept session_id (parent_id) and status");
    }

    #[tokio::test]
    async fn surface_status_rejects_unknown_values() {
        let pool = memory_pool().await;
        let unfiled = "00000000-0000-0000-0000-000000000000";
        let sid = "test-sess-bad-status";
        sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
            .bind(sid)
            .bind(unfiled)
            .bind("s")
            .execute(&pool)
            .await
            .unwrap();

        let result = sqlx::query("INSERT INTO surface (id, session_id, status) VALUES (?, ?, ?)")
            .bind("surf-bad-status")
            .bind(sid)
            .bind("running") // not in the CHECK constraint
            .execute(&pool)
            .await;

        assert!(
            result.is_err(),
            "surface.status must reject values outside the allowed set"
        );
    }

    #[tokio::test]
    async fn notification_has_read_and_snooze_until() {
        let pool = memory_pool().await;
        sqlx::query(
            "INSERT INTO notification (id, category, severity, message, ts, read, snooze_until)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("test-notif-col-check")
        .bind("surface-stopped")
        .bind("info")
        .bind("test")
        .bind(1_000_000i64)
        .bind(1i64)
        .bind(2_000_000i64)
        .execute(&pool)
        .await
        .expect("notification must accept read and snooze_until");
    }

    #[tokio::test]
    async fn command_has_sort_order_and_pinned() {
        let pool = memory_pool().await;
        sqlx::query(
            "INSERT INTO command (id, name, origin, cli, sort_order, pinned)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("test-cmd-col-check")
        .bind("my-tool")
        .bind("custom")
        .bind("/usr/bin/my-tool")
        .bind(7i64)
        .bind(1i64)
        .execute(&pool)
        .await
        .expect("command must accept sort_order and pinned");
    }

    // ── behavioral contracts ────────────────────────────────────────────────────

    #[tokio::test]
    async fn pinned_items_sort_before_unpinned() {
        let pool = memory_pool().await;
        let default_ws = "00000000-0000-0000-0000-000000000001";

        sqlx::query(
            "INSERT INTO project (id, workspace_id, name, sort_order, pinned) VALUES
             ('p-unpinned', ?, 'Unpinned', 10, 0),
             ('p-pinned',   ?, 'Pinned',   20, 1)",
        )
        .bind(default_ws)
        .bind(default_ws)
        .execute(&pool)
        .await
        .unwrap();

        let names: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM project
             WHERE workspace_id = ?
               AND id NOT IN ('00000000-0000-0000-0000-000000000000')
             ORDER BY pinned DESC, sort_order",
        )
        .bind(default_ws)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(names, vec!["Pinned", "Unpinned"]);
    }

    #[tokio::test]
    async fn surface_placement_is_unique_within_session() {
        let pool = memory_pool().await;
        let unfiled = "00000000-0000-0000-0000-000000000000";
        let sid = "test-sess-placement";
        sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
            .bind(sid)
            .bind(unfiled)
            .bind("s")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO surface (id, session_id, placement) VALUES (?, ?, ?)")
            .bind("surf-a")
            .bind(sid)
            .bind("slot-1")
            .execute(&pool)
            .await
            .expect("first surface with placement must succeed");

        let result =
            sqlx::query("INSERT INTO surface (id, session_id, placement) VALUES (?, ?, ?)")
                .bind("surf-b")
                .bind(sid)
                .bind("slot-1") // duplicate placement in same session
                .execute(&pool)
                .await;

        assert!(
            result.is_err(),
            "duplicate placement within a session must be rejected"
        );
    }

    #[tokio::test]
    async fn null_placement_does_not_violate_uniqueness() {
        let pool = memory_pool().await;
        let unfiled = "00000000-0000-0000-0000-000000000000";
        let sid = "test-sess-null-placement";
        sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
            .bind(sid)
            .bind(unfiled)
            .bind("s")
            .execute(&pool)
            .await
            .unwrap();

        // Two surfaces with NULL placement must not violate the unique index.
        sqlx::query("INSERT INTO surface (id, session_id) VALUES (?, ?)")
            .bind("surf-no-p-1")
            .bind(sid)
            .execute(&pool)
            .await
            .expect("surface without placement must succeed");

        sqlx::query("INSERT INTO surface (id, session_id) VALUES (?, ?)")
            .bind("surf-no-p-2")
            .bind(sid)
            .execute(&pool)
            .await
            .expect("second surface without placement must not violate the unique index");
    }

    #[tokio::test]
    async fn notification_unread_listing_excludes_read_records() {
        let pool = memory_pool().await;

        sqlx::query(
            "INSERT INTO notification (id, category, severity, message, ts, read) VALUES
             ('n-unread', 'cat', 'info', 'msg', 1, 0),
             ('n-read',   'cat', 'info', 'msg', 2, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let unread_ids: Vec<String> =
            sqlx::query_scalar("SELECT id FROM notification WHERE read = 0 ORDER BY ts DESC")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert_eq!(unread_ids, vec!["n-unread"]);
    }

    #[tokio::test]
    async fn foreign_key_prevents_project_without_workspace() {
        let pool = memory_pool().await;
        // Ensure FKs are on (WAL migration sets them; verify they survived).
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();

        let result = sqlx::query("INSERT INTO project (id, workspace_id, name) VALUES (?, ?, ?)")
            .bind("p-orphan")
            .bind("nonexistent-workspace")
            .bind("Orphan")
            .execute(&pool)
            .await;

        assert!(
            result.is_err(),
            "inserting a project with a nonexistent workspace_id must fail"
        );
    }
}
