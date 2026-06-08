//! The durable embedding work queue (`capture_queue`). Ingest enqueues one
//! request per committed chunk; the background worker drains it. Operations run
//! over the shared [`Store`] connection, so the caller serializes access.

use rusqlite::{params, OptionalExtension};

use crate::store::Store;

/// Enqueue an embedding request for a committed chunk. Idempotent: a second
/// enqueue for the same chunk is ignored.
pub fn enqueue(store: &Store, chunk_id: i64, created_at: i64) -> anyhow::Result<()> {
    store.conn().execute(
        "INSERT OR IGNORE INTO capture_queue(chunk_id, status, created_at)
         VALUES (?1, 'pending', ?2)",
        params![chunk_id, created_at],
    )?;
    Ok(())
}

/// Claim up to `limit` pending requests, oldest first, marking them `processing`
/// so an abandoned run can be reclaimed. Returns each claimed `(chunk_id,
/// content)` ready to embed.
pub fn drain_batch(store: &Store, limit: i64) -> anyhow::Result<Vec<(i64, String)>> {
    let conn = store.conn();
    let tx = conn.unchecked_transaction()?;
    let ids: Vec<i64> = {
        let mut stmt = tx.prepare(
            "SELECT chunk_id FROM capture_queue
             WHERE status = 'pending'
             ORDER BY created_at, chunk_id
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| r.get(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mut claimed = Vec::with_capacity(ids.len());
    {
        let mut mark =
            tx.prepare("UPDATE capture_queue SET status = 'processing' WHERE chunk_id = ?1")?;
        let mut content = tx.prepare("SELECT content FROM chunks WHERE id = ?1")?;
        for id in ids {
            mark.execute(params![id])?;
            if let Some(text) = content
                .query_row(params![id], |r| r.get::<_, String>(0))
                .optional()?
            {
                claimed.push((id, text));
            }
        }
    }
    tx.commit()?;
    Ok(claimed)
}

/// Remove a request after its chunk is embedded.
pub fn mark_embedded(store: &Store, chunk_id: i64) -> anyhow::Result<()> {
    store.conn().execute(
        "DELETE FROM capture_queue WHERE chunk_id = ?1",
        params![chunk_id],
    )?;
    Ok(())
}

/// Record a failed embedding attempt and reschedule it (back to `pending`).
pub fn mark_failed(store: &Store, chunk_id: i64, error: &str) -> anyhow::Result<()> {
    store.conn().execute(
        "UPDATE capture_queue
         SET status = 'pending', attempts = attempts + 1, last_error = ?2
         WHERE chunk_id = ?1",
        params![chunk_id, error],
    )?;
    Ok(())
}

/// Reset requests left `processing` by an abandoned run back to `pending`. Run
/// on worker startup. Returns the number reclaimed.
pub fn reclaim_stale(store: &Store) -> anyhow::Result<usize> {
    let n = store.conn().execute(
        "UPDATE capture_queue SET status = 'pending' WHERE status = 'processing'",
        [],
    )?;
    Ok(n)
}

/// The number of requests still pending.
pub fn pending_count(store: &Store) -> anyhow::Result<i64> {
    let n: i64 = store.conn().query_row(
        "SELECT COUNT(*) FROM capture_queue WHERE status = 'pending'",
        [],
        |r| r.get(0),
    )?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChunkKind, NewChunk};

    fn store_with_session() -> Store {
        let store = Store::open_in_memory().unwrap();
        store.ensure_session("s1", "test", None, 0).unwrap();
        store
    }

    fn insert_chunk(store: &Store, turn: i64, content: &str) -> i64 {
        store
            .insert_chunk(&NewChunk {
                session_id: Some("s1".into()),
                kind: ChunkKind::Chunk,
                content: content.into(),
                title: None,
                file_path: None,
                turn_index: Some(turn),
                ts: 1_000 + turn,
            })
            .unwrap()
            .unwrap()
    }

    #[test]
    fn enqueue_creates_pending_row_for_committed_chunk() {
        let store = store_with_session();
        let id = insert_chunk(&store, 0, "alpha");
        enqueue(&store, id, 1).unwrap();
        assert_eq!(pending_count(&store).unwrap(), 1);
    }

    #[test]
    fn enqueue_is_idempotent_for_same_chunk() {
        let store = store_with_session();
        let id = insert_chunk(&store, 0, "alpha");
        enqueue(&store, id, 1).unwrap();
        enqueue(&store, id, 2).unwrap();
        assert_eq!(pending_count(&store).unwrap(), 1);
    }

    #[test]
    fn drain_batch_returns_pending_oldest_first() {
        let store = store_with_session();
        let first = insert_chunk(&store, 0, "oldest");
        let second = insert_chunk(&store, 1, "middle");
        let third = insert_chunk(&store, 2, "newest");
        enqueue(&store, first, 10).unwrap();
        enqueue(&store, second, 20).unwrap();
        enqueue(&store, third, 30).unwrap();

        let drained = drain_batch(&store, 2).unwrap();

        assert_eq!(
            drained,
            vec![
                (first, "oldest".to_string()),
                (second, "middle".to_string())
            ]
        );
    }

    #[test]
    fn mark_embedded_removes_row() {
        let store = store_with_session();
        let id = insert_chunk(&store, 0, "alpha");
        enqueue(&store, id, 1).unwrap();
        mark_embedded(&store, id).unwrap();
        assert_eq!(pending_count(&store).unwrap(), 0);
    }

    #[test]
    fn mark_failed_increments_attempts_and_records_last_error() {
        let store = store_with_session();
        let id = insert_chunk(&store, 0, "alpha");
        enqueue(&store, id, 1).unwrap();

        mark_failed(&store, id, "embed exploded").unwrap();

        let (attempts, last_error): (i64, String) = store
            .conn()
            .query_row(
                "SELECT attempts, last_error FROM capture_queue WHERE chunk_id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(attempts, 1);
        assert_eq!(last_error, "embed exploded");
    }

    #[test]
    fn reclaim_stale_resets_abandoned_rows_on_startup() {
        let store = store_with_session();
        let id = insert_chunk(&store, 0, "alpha");
        enqueue(&store, id, 1).unwrap();
        // Drain marks it 'processing'; never marking it embedded simulates a crash.
        drain_batch(&store, 10).unwrap();
        assert_eq!(pending_count(&store).unwrap(), 0);

        let reclaimed = reclaim_stale(&store).unwrap();

        assert_eq!(reclaimed, 1);
        assert_eq!(pending_count(&store).unwrap(), 1);
    }

    #[test]
    fn pending_requests_survive_db_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("memorya.db");
        {
            let store = Store::open(&path).unwrap();
            store.ensure_session("s1", "test", None, 0).unwrap();
            let id = insert_chunk(&store, 0, "durable");
            enqueue(&store, id, 1).unwrap();
        }
        let reopened = Store::open(&path).unwrap();
        assert_eq!(pending_count(&reopened).unwrap(), 1);
        let drained = drain_batch(&reopened, 10).unwrap();
        assert_eq!(drained, vec![(1, "durable".to_string())]);
    }
}
