//! Per-entity sqlx repository for `NotificationRecord`. Executor-passing:
//! methods take `impl SqliteExecutor`, so the same call works against a pool
//! or a shared transaction. Owns the `notification` table and the
//! `Row -> NotificationRecord` mapping.

use sqlx::{FromRow, SqliteExecutor};

use crate::entities::notification::NotificationRecord;
use crate::shared::pagination::{Listing, Page};
use crate::shared::{Error, Result};

// ── Row type ─────────────────────────────────────────────────────────────────

/// Raw column values as returned by sqlx. SQLite stores booleans as `INTEGER`
/// (0/1) and optional timestamps as nullable `INTEGER`.
#[derive(FromRow)]
struct NotificationRow {
    id: String,
    category: String,
    severity: String,
    title: Option<String>,
    message: String,
    detail: Option<String>,
    ts: i64,
    session_id: Option<String>,
    surface_id: Option<String>,
    actions_json: Option<String>,
    read: i64,
    snooze_until: Option<i64>,
}

impl From<NotificationRow> for NotificationRecord {
    fn from(r: NotificationRow) -> Self {
        NotificationRecord {
            id: r.id,
            category: r.category,
            severity: r.severity,
            title: r.title,
            message: r.message,
            detail: r.detail,
            ts: r.ts,
            session_id: r.session_id,
            surface_id: r.surface_id,
            actions_json: r.actions_json,
            read: r.read != 0,
            snooze_until: r.snooze_until,
        }
    }
}

// ── Repository ────────────────────────────────────────────────────────────────

pub struct NotificationRepo;

impl NotificationRepo {
    /// Insert a new notification record.
    pub async fn create<'e>(exec: impl SqliteExecutor<'e>, n: &NotificationRecord) -> Result<()> {
        let read = n.read as i64;
        sqlx::query(
            "INSERT INTO notification
             (id, category, severity, title, message, detail, ts,
              session_id, surface_id, actions_json, read, snooze_until)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&n.id)
        .bind(&n.category)
        .bind(&n.severity)
        .bind(&n.title)
        .bind(&n.message)
        .bind(&n.detail)
        .bind(n.ts)
        .bind(&n.session_id)
        .bind(&n.surface_id)
        .bind(&n.actions_json)
        .bind(read)
        .bind(n.snooze_until)
        .execute(exec)
        .await?;
        Ok(())
    }

    /// Fetch one notification by id. Returns `None` when absent.
    pub async fn get<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &str,
    ) -> Result<Option<NotificationRecord>> {
        let row = sqlx::query_as::<_, NotificationRow>(
            "SELECT id, category, severity, title, message, detail, ts,
                    session_id, surface_id, actions_json, read, snooze_until
             FROM notification WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(exec)
        .await?;
        Ok(row.map(Into::into))
    }

    /// List all notifications ordered by `ts DESC`, with optional pagination.
    pub async fn list<'e>(
        exec: impl SqliteExecutor<'e>,
        page: &Page,
    ) -> Result<Listing<NotificationRecord>> {
        match page {
            Page::All => {
                let rows = sqlx::query_as::<_, NotificationRow>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id, actions_json, read, snooze_until
                     FROM notification ORDER BY ts DESC",
                )
                .fetch_all(exec)
                .await?;
                Ok(Listing::new(
                    rows.into_iter().map(Into::into).collect(),
                    None,
                ))
            }

            Page::Offset { limit, offset } => {
                let rows = sqlx::query_as::<_, NotificationRow>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id, actions_json, read, snooze_until
                     FROM notification ORDER BY ts DESC LIMIT ? OFFSET ?",
                )
                .bind(*limit as i64 + 1)
                .bind(*offset as i64)
                .fetch_all(exec)
                .await?;

                let has_more = rows.len() > *limit as usize;
                let items: Vec<NotificationRecord> = rows
                    .into_iter()
                    .take(*limit as usize)
                    .map(Into::into)
                    .collect();
                let next = if has_more {
                    Some((*offset + *limit).to_string())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }

            Page::Cursor { after: None, limit } => {
                let rows = sqlx::query_as::<_, NotificationRow>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id, actions_json, read, snooze_until
                     FROM notification ORDER BY ts DESC, id DESC LIMIT ?",
                )
                .bind(*limit as i64 + 1)
                .fetch_all(exec)
                .await?;

                let has_more = rows.len() > *limit as usize;
                let items: Vec<NotificationRecord> = rows
                    .into_iter()
                    .take(*limit as usize)
                    .map(Into::into)
                    .collect();
                let next = if has_more {
                    items.last().map(|r| r.ts.to_string())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }

            Page::Cursor {
                after: Some(cursor),
                limit,
            } => {
                // Cursor is the `ts` of the last item on the previous page.
                let cursor_ts: i64 = cursor.parse().map_err(|_| Error::Validation {
                    field: "cursor",
                    reason: "not a valid timestamp cursor".to_owned(),
                })?;
                let rows = sqlx::query_as::<_, NotificationRow>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id, actions_json, read, snooze_until
                     FROM notification WHERE ts < ?
                     ORDER BY ts DESC, id DESC LIMIT ?",
                )
                .bind(cursor_ts)
                .bind(*limit as i64 + 1)
                .fetch_all(exec)
                .await?;

                let has_more = rows.len() > *limit as usize;
                let items: Vec<NotificationRecord> = rows
                    .into_iter()
                    .take(*limit as usize)
                    .map(Into::into)
                    .collect();
                let next = if has_more {
                    items.last().map(|r| r.ts.to_string())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }
        }
    }

    /// List only unread notifications (`read = 0`) ordered by `ts DESC`.
    pub async fn list_unread<'e>(
        exec: impl SqliteExecutor<'e>,
        page: &Page,
    ) -> Result<Listing<NotificationRecord>> {
        match page {
            Page::All => {
                let rows = sqlx::query_as::<_, NotificationRow>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id, actions_json, read, snooze_until
                     FROM notification WHERE read = 0 ORDER BY ts DESC",
                )
                .fetch_all(exec)
                .await?;
                Ok(Listing::new(
                    rows.into_iter().map(Into::into).collect(),
                    None,
                ))
            }

            Page::Offset { limit, offset } => {
                let rows = sqlx::query_as::<_, NotificationRow>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id, actions_json, read, snooze_until
                     FROM notification WHERE read = 0 ORDER BY ts DESC LIMIT ? OFFSET ?",
                )
                .bind(*limit as i64 + 1)
                .bind(*offset as i64)
                .fetch_all(exec)
                .await?;

                let has_more = rows.len() > *limit as usize;
                let items: Vec<NotificationRecord> = rows
                    .into_iter()
                    .take(*limit as usize)
                    .map(Into::into)
                    .collect();
                let next = if has_more {
                    Some((*offset + *limit).to_string())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }

            Page::Cursor { after: None, limit } => {
                let rows = sqlx::query_as::<_, NotificationRow>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id, actions_json, read, snooze_until
                     FROM notification WHERE read = 0 ORDER BY ts DESC, id DESC LIMIT ?",
                )
                .bind(*limit as i64 + 1)
                .fetch_all(exec)
                .await?;

                let has_more = rows.len() > *limit as usize;
                let items: Vec<NotificationRecord> = rows
                    .into_iter()
                    .take(*limit as usize)
                    .map(Into::into)
                    .collect();
                let next = if has_more {
                    items.last().map(|r| r.ts.to_string())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }

            Page::Cursor {
                after: Some(cursor),
                limit,
            } => {
                let cursor_ts: i64 = cursor.parse().map_err(|_| Error::Validation {
                    field: "cursor",
                    reason: "not a valid timestamp cursor".to_owned(),
                })?;
                let rows = sqlx::query_as::<_, NotificationRow>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id, actions_json, read, snooze_until
                     FROM notification WHERE read = 0 AND ts < ?
                     ORDER BY ts DESC, id DESC LIMIT ?",
                )
                .bind(cursor_ts)
                .bind(*limit as i64 + 1)
                .fetch_all(exec)
                .await?;

                let has_more = rows.len() > *limit as usize;
                let items: Vec<NotificationRecord> = rows
                    .into_iter()
                    .take(*limit as usize)
                    .map(Into::into)
                    .collect();
                let next = if has_more {
                    items.last().map(|r| r.ts.to_string())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }
        }
    }

    /// Count unread notifications.
    pub async fn count_unread<'e>(exec: impl SqliteExecutor<'e>) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM notification WHERE read = 0")
            .fetch_one(exec)
            .await?;
        Ok(row.0)
    }

    /// Update the `read` and `snooze_until` fields of a notification.
    pub async fn update<'e>(exec: impl SqliteExecutor<'e>, n: &NotificationRecord) -> Result<()> {
        let read = n.read as i64;
        let rows = sqlx::query("UPDATE notification SET read = ?, snooze_until = ? WHERE id = ?")
            .bind(read)
            .bind(n.snooze_until)
            .bind(&n.id)
            .execute(exec)
            .await?
            .rows_affected();

        if rows == 0 {
            return Err(Error::NotificationNotFound(n.id.clone()));
        }
        Ok(())
    }

    /// Mark one notification as read.
    pub async fn mark_read<'e>(exec: impl SqliteExecutor<'e>, id: &str) -> Result<()> {
        let rows = sqlx::query("UPDATE notification SET read = 1 WHERE id = ?")
            .bind(id)
            .execute(exec)
            .await?
            .rows_affected();

        if rows == 0 {
            return Err(Error::NotificationNotFound(id.to_owned()));
        }
        Ok(())
    }

    /// Mark every notification as read.
    pub async fn mark_all_read<'e>(exec: impl SqliteExecutor<'e>) -> Result<()> {
        sqlx::query("UPDATE notification SET read = 1")
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Set `snooze_until` on a notification. Pass `None` to clear the snooze.
    pub async fn snooze<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &str,
        snooze_until: Option<i64>,
    ) -> Result<()> {
        let rows = sqlx::query("UPDATE notification SET snooze_until = ? WHERE id = ?")
            .bind(snooze_until)
            .bind(id)
            .execute(exec)
            .await?
            .rows_affected();

        if rows == 0 {
            return Err(Error::NotificationNotFound(id.to_owned()));
        }
        Ok(())
    }

    /// Delete a single notification by id.
    pub async fn delete<'e>(exec: impl SqliteExecutor<'e>, id: &str) -> Result<()> {
        let rows = sqlx::query("DELETE FROM notification WHERE id = ?")
            .bind(id)
            .execute(exec)
            .await?
            .rows_affected();

        if rows == 0 {
            return Err(Error::NotificationNotFound(id.to_owned()));
        }
        Ok(())
    }

    /// Delete all notifications.
    pub async fn delete_all<'e>(exec: impl SqliteExecutor<'e>) -> Result<()> {
        sqlx::query("DELETE FROM notification")
            .execute(exec)
            .await?;
        Ok(())
    }

    /// Retention cap: keep only the most recent `keep` records; delete the rest.
    pub async fn prune<'e>(exec: impl SqliteExecutor<'e>, keep: u32) -> Result<()> {
        sqlx::query(
            "DELETE FROM notification
             WHERE id NOT IN (
                 SELECT id FROM notification ORDER BY ts DESC LIMIT ?
             )",
        )
        .bind(keep as i64)
        .execute(exec)
        .await?;
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

    async fn memory_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory pool");
        // Apply the domain schema so all tables exist.
        static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("src/infra/migrations");
        MIGRATOR.run(&pool).await.expect("migrations");
        pool
    }

    fn sample(id: &str) -> NotificationRecord {
        NotificationRecord {
            id: id.to_owned(),
            category: "test".to_owned(),
            severity: "info".to_owned(),
            title: Some("T".to_owned()),
            message: "msg".to_owned(),
            detail: None,
            ts: 1_000,
            session_id: None,
            surface_id: None,
            actions_json: None,
            read: false,
            snooze_until: None,
        }
    }

    fn sample_at(id: &str, ts: i64) -> NotificationRecord {
        NotificationRecord {
            id: id.to_owned(),
            ts,
            ..sample(id)
        }
    }

    // ── Scenario: round-trip ──────────────────────────────────────────────────

    #[tokio::test]
    async fn create_and_get_returns_the_same_record() {
        let pool = memory_pool().await;
        let n = sample("n1");
        NotificationRepo::create(&pool, &n).await.unwrap();
        let got = NotificationRepo::get(&pool, "n1").await.unwrap().unwrap();
        assert_eq!(got, n);
    }

    #[tokio::test]
    async fn get_returns_none_for_absent_id() {
        let pool = memory_pool().await;
        let got = NotificationRepo::get(&pool, "missing").await.unwrap();
        assert!(got.is_none());
    }

    // ── Scenario: read and snooze round-trip ──────────────────────────────────

    #[tokio::test]
    async fn read_flag_and_snooze_until_round_trip() {
        let pool = memory_pool().await;
        let mut n = sample("n2");
        NotificationRepo::create(&pool, &n).await.unwrap();

        n.read = true;
        n.snooze_until = Some(9_999_999);
        NotificationRepo::update(&pool, &n).await.unwrap();

        let got = NotificationRepo::get(&pool, "n2").await.unwrap().unwrap();
        assert!(got.read);
        assert_eq!(got.snooze_until, Some(9_999_999));
    }

    // ── Scenario: mark_read and count_unread ──────────────────────────────────

    #[tokio::test]
    async fn mark_read_removes_record_from_unread_listing() {
        let pool = memory_pool().await;
        NotificationRepo::create(&pool, &sample("n3"))
            .await
            .unwrap();
        NotificationRepo::create(&pool, &sample("n4"))
            .await
            .unwrap();

        let before = NotificationRepo::count_unread(&pool).await.unwrap();
        assert_eq!(before, 2);

        NotificationRepo::mark_read(&pool, "n3").await.unwrap();

        let after = NotificationRepo::count_unread(&pool).await.unwrap();
        assert_eq!(after, 1);

        let unread = NotificationRepo::list_unread(&pool, &Page::All)
            .await
            .unwrap();
        assert_eq!(unread.items.len(), 1);
        assert_eq!(unread.items[0].id, "n4");
    }

    #[tokio::test]
    async fn mark_read_on_absent_id_returns_not_found() {
        let pool = memory_pool().await;
        let err = NotificationRepo::mark_read(&pool, "ghost")
            .await
            .unwrap_err();
        assert_eq!(err.code(), "notification.not_found");
    }

    #[tokio::test]
    async fn mark_all_read_clears_the_unread_count() {
        let pool = memory_pool().await;
        for id in ["a", "b", "c"] {
            NotificationRepo::create(&pool, &sample(id)).await.unwrap();
        }
        NotificationRepo::mark_all_read(&pool).await.unwrap();
        let count = NotificationRepo::count_unread(&pool).await.unwrap();
        assert_eq!(count, 0);
    }

    // ── Scenario: snooze ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn snooze_sets_and_clears_snooze_until() {
        let pool = memory_pool().await;
        NotificationRepo::create(&pool, &sample("sn1"))
            .await
            .unwrap();

        NotificationRepo::snooze(&pool, "sn1", Some(5_000))
            .await
            .unwrap();
        let snoozed = NotificationRepo::get(&pool, "sn1").await.unwrap().unwrap();
        assert_eq!(snoozed.snooze_until, Some(5_000));

        NotificationRepo::snooze(&pool, "sn1", None).await.unwrap();
        let cleared = NotificationRepo::get(&pool, "sn1").await.unwrap().unwrap();
        assert_eq!(cleared.snooze_until, None);
    }

    #[tokio::test]
    async fn snooze_on_absent_id_returns_not_found() {
        let pool = memory_pool().await;
        let err = NotificationRepo::snooze(&pool, "ghost", Some(1))
            .await
            .unwrap_err();
        assert_eq!(err.code(), "notification.not_found");
    }

    // ── Scenario: delete ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_removes_the_record() {
        let pool = memory_pool().await;
        NotificationRepo::create(&pool, &sample("d1"))
            .await
            .unwrap();
        NotificationRepo::delete(&pool, "d1").await.unwrap();
        let got = NotificationRepo::get(&pool, "d1").await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn delete_on_absent_id_returns_not_found() {
        let pool = memory_pool().await;
        let err = NotificationRepo::delete(&pool, "ghost").await.unwrap_err();
        assert_eq!(err.code(), "notification.not_found");
    }

    #[tokio::test]
    async fn delete_all_removes_every_record() {
        let pool = memory_pool().await;
        for id in ["x", "y", "z"] {
            NotificationRepo::create(&pool, &sample(id)).await.unwrap();
        }
        NotificationRepo::delete_all(&pool).await.unwrap();
        let listing = NotificationRepo::list(&pool, &Page::All).await.unwrap();
        assert!(listing.items.is_empty());
    }

    // ── Scenario: list (pagination) ───────────────────────────────────────────

    #[tokio::test]
    async fn list_all_returns_records_ordered_by_ts_desc() {
        let pool = memory_pool().await;
        for (id, ts) in [("p1", 100i64), ("p2", 300), ("p3", 200)] {
            NotificationRepo::create(&pool, &sample_at(id, ts))
                .await
                .unwrap();
        }
        let listing = NotificationRepo::list(&pool, &Page::All).await.unwrap();
        let ids: Vec<&str> = listing.items.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["p2", "p3", "p1"]);
    }

    #[tokio::test]
    async fn offset_page_returns_bounded_slice_and_continuation() {
        let pool = memory_pool().await;
        for (id, ts) in [("o1", 10), ("o2", 20), ("o3", 30), ("o4", 40), ("o5", 50)] {
            NotificationRepo::create(&pool, &sample_at(id, ts))
                .await
                .unwrap();
        }
        // ts DESC = [o5, o4, o3, o2, o1]
        let page1 = NotificationRepo::list(&pool, &Page::offset(2, 0))
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.items[0].id, "o5");
        assert_eq!(page1.items[1].id, "o4");
        assert!(page1.has_next());

        let page2 = NotificationRepo::list(&pool, &Page::offset(2, 2))
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 2);
        assert_eq!(page2.items[0].id, "o3");
        assert!(page2.has_next());

        let page3 = NotificationRepo::list(&pool, &Page::offset(2, 4))
            .await
            .unwrap();
        assert_eq!(page3.items.len(), 1);
        assert!(!page3.has_next());
    }

    #[tokio::test]
    async fn cursor_page_from_start_returns_bounded_slice() {
        let pool = memory_pool().await;
        for (id, ts) in [("c1", 10), ("c2", 20), ("c3", 30)] {
            NotificationRepo::create(&pool, &sample_at(id, ts))
                .await
                .unwrap();
        }
        let page1 = NotificationRepo::list(&pool, &Page::cursor_from_start(2))
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.items[0].id, "c3");
        assert_eq!(page1.items[1].id, "c2");
        assert!(page1.has_next());

        let next_cursor = page1.next.unwrap();
        let page2 = NotificationRepo::list(&pool, &Page::cursor_after(&next_cursor, 2))
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 1);
        assert_eq!(page2.items[0].id, "c1");
        assert!(!page2.has_next());
    }

    // ── Scenario: retention cap ───────────────────────────────────────────────

    #[tokio::test]
    async fn prune_keeps_only_the_most_recent_n_records() {
        let pool = memory_pool().await;
        for (id, ts) in [
            ("pr1", 10),
            ("pr2", 20),
            ("pr3", 30),
            ("pr4", 40),
            ("pr5", 50),
        ] {
            NotificationRepo::create(&pool, &sample_at(id, ts))
                .await
                .unwrap();
        }
        NotificationRepo::prune(&pool, 3).await.unwrap();
        let listing = NotificationRepo::list(&pool, &Page::All).await.unwrap();
        assert_eq!(listing.items.len(), 3);
        let ids: Vec<&str> = listing.items.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["pr5", "pr4", "pr3"]);
    }

    #[tokio::test]
    async fn prune_with_keep_larger_than_count_deletes_nothing() {
        let pool = memory_pool().await;
        NotificationRepo::create(&pool, &sample("only"))
            .await
            .unwrap();
        NotificationRepo::prune(&pool, 100).await.unwrap();
        let listing = NotificationRepo::list(&pool, &Page::All).await.unwrap();
        assert_eq!(listing.items.len(), 1);
    }

    // ── Scenario: multi-repo call on one tx is atomic ────────────────────────

    #[tokio::test]
    async fn two_repo_writes_on_one_tx_are_atomic_on_rollback() {
        let pool = memory_pool().await;
        let n1 = sample("tx1");
        let n2 = sample("tx2");

        let mut tx = pool.begin().await.unwrap();
        NotificationRepo::create(&mut *tx, &n1).await.unwrap();
        NotificationRepo::create(&mut *tx, &n2).await.unwrap();
        tx.rollback().await.unwrap();

        assert!(NotificationRepo::get(&pool, "tx1").await.unwrap().is_none());
        assert!(NotificationRepo::get(&pool, "tx2").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn two_repo_writes_on_one_tx_both_commit() {
        let pool = memory_pool().await;
        let n1 = sample("c_tx1");
        let n2 = sample("c_tx2");

        let mut tx = pool.begin().await.unwrap();
        NotificationRepo::create(&mut *tx, &n1).await.unwrap();
        NotificationRepo::create(&mut *tx, &n2).await.unwrap();
        tx.commit().await.unwrap();

        assert!(NotificationRepo::get(&pool, "c_tx1")
            .await
            .unwrap()
            .is_some());
        assert!(NotificationRepo::get(&pool, "c_tx2")
            .await
            .unwrap()
            .is_some());
    }

    // ── Scenario: update on absent id returns not_found ───────────────────────

    #[tokio::test]
    async fn update_on_absent_id_returns_not_found() {
        let pool = memory_pool().await;
        let mut n = sample("ghost");
        n.read = true;
        let err = NotificationRepo::update(&pool, &n).await.unwrap_err();
        assert_eq!(err.code(), "notification.not_found");
    }
}
