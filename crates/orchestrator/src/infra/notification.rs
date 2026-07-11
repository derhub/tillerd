//! Per-entity sqlx repository for `NotificationRecord`. Executor-passing:
//! methods take `impl SqliteExecutor`, so the same call works against a pool
//! or a shared transaction. Owns the `notification` table and the
//! `Row -> NotificationRecord` mapping.

use sqlx::SqliteExecutor;

use crate::entities::notification::NotificationRecord;
use crate::shared::{Error, Result};

pub struct NotificationRepo;

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

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

    /// Fetch one notification by id. Returns `None` when absent. Test helper for
    /// read-after-write assertions.
    #[cfg(test)]
    pub async fn get<'e>(
        exec: impl SqliteExecutor<'e>,
        id: &str,
    ) -> Result<Option<NotificationRecord>> {
        Ok(sqlx::query_as::<_, NotificationRecord>(
            "SELECT id, category, severity, title, message, detail, ts,
                    session_id, surface_id, actions_json, read, snooze_until
             FROM notification WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(exec)
        .await?)
    }

    /// Count unread notifications. A notification snoozed into the future does not
    /// count until its `snooze_until` elapses.
    pub async fn count_unread<'e>(exec: impl SqliteExecutor<'e>) -> Result<i64> {
        let now = now_millis();
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM notification
             WHERE read = 0 AND (snooze_until IS NULL OR snooze_until <= ?)",
        )
        .bind(now)
        .fetch_one(exec)
        .await?;
        Ok(row.0)
    }

    /// Update the `read` and `snooze_until` fields of a notification. Test helper.
    #[cfg(test)]
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

    /// Delete all rows except the `keep` most recent (by `ts`).
    /// The caller decides the retention count; this method runs the DELETE only.
    pub async fn prune<'e>(exec: impl SqliteExecutor<'e>, keep: i64) -> Result<()> {
        sqlx::query(
            "DELETE FROM notification
             WHERE id NOT IN (
                 SELECT id FROM notification ORDER BY ts DESC LIMIT ?
             )",
        )
        .bind(keep)
        .execute(exec)
        .await?;
        Ok(())
    }
}

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

        // n3 is now read; n4 is still unread.
        assert!(
            NotificationRepo::get(&pool, "n3")
                .await
                .unwrap()
                .unwrap()
                .read
        );
        assert!(
            !NotificationRepo::get(&pool, "n4")
                .await
                .unwrap()
                .unwrap()
                .read
        );
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
    async fn count_unread_excludes_a_notification_snoozed_into_the_future() {
        let pool = memory_pool().await;
        let now = now_millis();

        let mut future = sample("future");
        future.snooze_until = Some(now + 60_000);
        NotificationRepo::create(&pool, &future).await.unwrap();

        let mut past = sample("past");
        past.snooze_until = Some(now - 60_000);
        NotificationRepo::create(&pool, &past).await.unwrap();

        NotificationRepo::create(&pool, &sample("plain"))
            .await
            .unwrap();

        // future-snoozed is excluded; past-snoozed (elapsed) and plain both count.
        let count = NotificationRepo::count_unread(&pool).await.unwrap();
        assert_eq!(count, 2);
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
        // Every record is gone (all were unread).
        let count = NotificationRepo::count_unread(&pool).await.unwrap();
        assert_eq!(count, 0);
        for id in ["x", "y", "z"] {
            assert!(NotificationRepo::get(&pool, id).await.unwrap().is_none());
        }
    }

    #[tokio::test]
    async fn prune_runs_delete_and_returns_ok() {
        let pool = memory_pool().await;
        for (id, ts) in [("pr1", 10i64), ("pr2", 20), ("pr3", 30)] {
            NotificationRepo::create(&pool, &sample_at(id, ts))
                .await
                .unwrap();
        }
        // keep=2: raw SQL executes without error; rows are removed.
        NotificationRepo::prune(&pool, 2).await.unwrap();
        assert!(NotificationRepo::get(&pool, "pr1").await.unwrap().is_none());
    }

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

    #[tokio::test]
    async fn update_on_absent_id_returns_not_found() {
        let pool = memory_pool().await;
        let mut n = sample("ghost");
        n.read = true;
        let err = NotificationRepo::update(&pool, &n).await.unwrap_err();
        assert_eq!(err.code(), "notification.not_found");
    }
}
