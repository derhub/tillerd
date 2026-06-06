//! Storage layer — the only component that opens the database.

use crate::NewChunk;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const SCHEMA_SQL: &str = include_str!("schema.sql");

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open and migrate the active database.
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self { conn })
    }

    /// Open an in-memory database (for tests).
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self { conn })
    }

    pub fn schema_version(&self) -> anyhow::Result<i64> {
        let v: i64 = self
            .conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))?;
        Ok(v)
    }

    /// Insert a chunk. Returns the new row id, or `None` if a same-session
    /// duplicate fire was suppressed by the uniqueness constraint.
    pub fn insert_chunk(&self, c: &NewChunk) -> anyhow::Result<Option<i64>> {
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO chunks
               (session_id, kind, content, title, file_path, turn_index, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                c.session_id,
                c.kind.as_str(),
                c.content,
                c.title,
                c.file_path,
                c.turn_index,
                c.ts,
            ],
        )?;
        if n == 0 {
            return Ok(None);
        }
        Ok(Some(self.conn.last_insert_rowid()))
    }

    /// Delete every indexed document chunk and its embeddings.
    pub fn prune_docs(&self) -> anyhow::Result<usize> {
        self.conn.execute(
            "DELETE FROM embeddings WHERE observation_id IN
               (SELECT CAST(id AS TEXT) FROM chunks WHERE kind = 'doc')",
            [],
        )?;
        let n = self
            .conn
            .execute("DELETE FROM chunks WHERE kind = 'doc'", [])?;
        Ok(n)
    }

    /// Wipe all memory in the active database (chunks, digests, facts, entities,
    /// relations, embeddings, sessions). Archive shards are left untouched.
    pub fn prune_all(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "PRAGMA foreign_keys=OFF;
             DELETE FROM embeddings;
             DELETE FROM chunks;
             DELETE FROM digests;
             DELETE FROM relations;
             DELETE FROM facts;
             DELETE FROM entities;
             DELETE FROM sessions;
             PRAGMA foreign_keys=ON;",
        )?;
        Ok(())
    }

    /// Replace all document chunks for a project path prefix. Used by the
    /// per-session document indexer before re-inserting fresh chunks.
    pub fn delete_doc_chunks_under(&self, cwd_prefix: &str) -> anyhow::Result<usize> {
        let like = format!("{cwd_prefix}/%");
        let n = self.conn.execute(
            "DELETE FROM chunks WHERE kind = 'doc' AND file_path LIKE ?1",
            params![like],
        )?;
        Ok(n)
    }

    pub fn active_chunk_count(&self) -> anyhow::Result<i64> {
        let n: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM chunks WHERE archived = 0", [], |r| {
                    r.get(0)
                })?;
        Ok(n)
    }

    /// Most recent chunk timestamp, if any (for digest staleness checks).
    pub fn last_chunk_ts(&self) -> anyhow::Result<Option<i64>> {
        let t: Option<i64> = self
            .conn
            .query_row("SELECT MAX(ts) FROM chunks", [], |r| r.get(0))
            .optional()?
            .flatten();
        Ok(t)
    }

    /// Record an embedding for a stored item.
    pub fn put_embedding(
        &self,
        observation_id: &str,
        kind: &str,
        model: &str,
        vec: &[f32],
    ) -> anyhow::Result<()> {
        let bytes = crate::embed::encode_vec(vec);
        self.conn.execute(
            "INSERT OR REPLACE INTO embeddings(observation_id, kind, model, dim, vec)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![observation_id, kind, model, vec.len() as i64, bytes],
        )?;
        Ok(())
    }

    /// Drop every embedding not produced by `model`. Run on startup when the
    /// active embedding model changes, so recall never compares vectors across
    /// models; the missing ones are then backfilled out-of-band.
    pub fn drop_embeddings_for_other_models(&self, model: &str) -> anyhow::Result<usize> {
        let n = self
            .conn
            .execute("DELETE FROM embeddings WHERE model != ?1", params![model])?;
        Ok(n)
    }

    /// Chunks that have no embedding for `model` yet, oldest first, capped at
    /// `limit`. Returns `(id, kind, content)`.
    pub fn chunks_missing_embeddings(
        &self,
        model: &str,
        limit: i64,
    ) -> anyhow::Result<Vec<(i64, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.kind, c.content
             FROM chunks c
             LEFT JOIN embeddings e
               ON e.observation_id = CAST(c.id AS TEXT) AND e.model = ?1
             WHERE e.observation_id IS NULL
             ORDER BY c.id
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![model, limit], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Load `(chunk_id, vector)` for all non-archived chunk embeddings of `model`.
    pub fn load_chunk_vectors(&self, model: &str) -> anyhow::Result<Vec<(i64, Vec<f32>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, e.vec
             FROM embeddings e
             JOIN chunks c ON CAST(c.id AS TEXT) = e.observation_id
             WHERE c.archived = 0 AND e.model = ?1",
        )?;
        let rows = stmt.query_map(params![model], |r| {
            let id: i64 = r.get(0)?;
            let bytes: Vec<u8> = r.get(1)?;
            Ok((id, crate::embed::decode_vec(&bytes)))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Title (falling back to a content prefix) and content for a chunk.
    pub fn chunk_brief(&self, id: i64) -> anyhow::Result<Option<(String, String)>> {
        let row = self
            .conn
            .query_row(
                "SELECT title, content FROM chunks WHERE id = ?1",
                params![id],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        Ok(row.map(|(title, content)| {
            let t = crate::title_or_prefix(title, &content);
            (t, content)
        }))
    }

    /// Mark a chunk accessed (bumps recency + frequency for ranking and eviction).
    pub fn touch_chunk(&self, id: i64, ts: i64) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE chunks SET last_accessed = ?1, access_count = access_count + 1 WHERE id = ?2",
            params![ts, id],
        )?;
        Ok(())
    }

    /// Current four-digit year, from SQLite's clock.
    pub fn current_year(&self) -> anyhow::Result<i64> {
        let y: String = self
            .conn
            .query_row("SELECT strftime('%Y','now')", [], |r| r.get(0))?;
        Ok(y.parse().unwrap_or(1970))
    }

    /// Timestamp of a chunk, if present.
    pub fn chunk_ts(&self, id: i64) -> anyhow::Result<Option<i64>> {
        Ok(self
            .conn
            .query_row("SELECT ts FROM chunks WHERE id = ?1", params![id], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Most recent digests of a scope, newest first: `(content, ts)`.
    pub fn recent_digests(&self, scope: &str, limit: i64) -> anyhow::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT content, ts FROM digests WHERE scope = ?1 ORDER BY ts DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![scope, limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Full content for multiple chunk ids, in the order requested. Missing ids
    /// are skipped.
    pub fn chunk_contents(&self, ids: &[i64]) -> anyhow::Result<Vec<(i64, String)>> {
        let mut out = Vec::with_capacity(ids.len());
        let mut stmt = self
            .conn
            .prepare("SELECT content FROM chunks WHERE id = ?1")?;
        for &id in ids {
            if let Some(content) = stmt
                .query_row(params![id], |r| r.get::<_, String>(0))
                .optional()?
            {
                out.push((id, content));
            }
        }
        Ok(out)
    }

    /// Most recent active chunks for the viewer: `(id, title, content)`.
    pub fn recent_chunks(&self, limit: i64) -> anyhow::Result<Vec<(i64, Option<String>, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content FROM chunks WHERE archived = 0 ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Ensure a session row exists.
    pub fn ensure_session(&self, id: &str, ide: &str, cwd: Option<&str>, ts: i64) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO sessions(id, ide, cwd, started_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, ide, cwd, ts],
        )?;
        Ok(())
    }

    /// Distinct project working directories seen across sessions.
    pub fn project_cwds(&self) -> anyhow::Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT cwd FROM sessions WHERE cwd IS NOT NULL ORDER BY cwd")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // --- consolidation ---

    /// Uncovered chunk `(id, content)` for a session, oldest first.
    pub fn session_uncovered_chunks(&self, session_id: &str) -> anyhow::Result<Vec<(i64, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content FROM chunks
             WHERE session_id = ?1 AND covered_by_digest IS NULL
             ORDER BY ts, id",
        )?;
        let rows = stmt.query_map(params![session_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Insert a digest and return its id.
    pub fn insert_digest(
        &self,
        session_id: Option<&str>,
        scope: &str,
        content: &str,
        ts: i64,
    ) -> anyhow::Result<i64> {
        self.conn.execute(
            "INSERT INTO digests(session_id, scope, content, ts) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, scope, content, ts],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Mark a session's currently-uncovered chunks as covered by `digest_id`.
    pub fn cover_session_chunks(&self, session_id: &str, digest_id: i64) -> anyhow::Result<usize> {
        let n = self.conn.execute(
            "UPDATE chunks SET covered_by_digest = ?1
             WHERE session_id = ?2 AND covered_by_digest IS NULL",
            params![digest_id, session_id],
        )?;
        Ok(n)
    }

    /// Uncovered digests `(id, content)` of a given scope, oldest first.
    pub fn uncovered_digests(&self, scope: &str) -> anyhow::Result<Vec<(i64, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content FROM digests
             WHERE scope = ?1 AND covered_by IS NULL
             ORDER BY ts, id",
        )?;
        let rows = stmt.query_map(params![scope], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Mark digests covered by a parent digest.
    pub fn cover_digests(&self, ids: &[i64], parent_id: i64) -> anyhow::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for id in ids {
            tx.execute(
                "UPDATE digests SET covered_by = ?1 WHERE id = ?2",
                params![parent_id, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Sessions with at least `min_uncovered` uncovered chunks older than `before_ts`.
    pub fn sessions_with_coverage_debt(
        &self,
        min_uncovered: i64,
        before_ts: i64,
    ) -> anyhow::Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, COUNT(*) AS n FROM chunks
             WHERE covered_by_digest IS NULL AND session_id IS NOT NULL AND ts < ?1
             GROUP BY session_id HAVING n >= ?2",
        )?;
        let rows = stmt.query_map(params![before_ts, min_uncovered], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // --- eviction / archive ---

    /// Stats for non-archived, non-doc chunks (doc chunks are regenerated each
    /// session, so they are not archived).
    pub fn evictable_chunk_stats(&self) -> anyhow::Result<Vec<crate::coverage::ChunkStat>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, ts, last_accessed, access_count,
                    covered_by_digest IS NOT NULL, covered_by_fact IS NOT NULL
             FROM chunks
             WHERE archived = 0 AND kind != 'doc'",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(crate::coverage::ChunkStat {
                id: r.get(0)?,
                ts: r.get(1)?,
                last_accessed: r.get(2)?,
                access_count: r.get(3)?,
                covered_by_digest: r.get(4)?,
                covered_by_fact: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Move the given chunks (and their embeddings) into the archive database at
    /// `archive_path` in one atomic operation, then remove them from the active
    /// database.
    pub fn move_chunks_to_archive(
        &self,
        archive_path: &str,
        ids: &[i64],
    ) -> anyhow::Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }
        self.conn
            .execute("ATTACH DATABASE ?1 AS arc", params![archive_path])?;
        let result = (|| -> anyhow::Result<usize> {
            self.conn.execute_batch(ARCHIVE_SCHEMA)?;
            let in_list = ids
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let text_list = ids
                .iter()
                .map(|i| format!("'{i}'"))
                .collect::<Vec<_>>()
                .join(",");
            let tx = self.conn.unchecked_transaction()?;
            tx.execute_batch(&format!(
                "INSERT OR REPLACE INTO arc.chunks
                   SELECT id, session_id, kind, content, title, file_path, turn_index, ts,
                          covered_by_digest, covered_by_fact, last_accessed, access_count, 1
                   FROM main.chunks WHERE id IN ({in_list});
                 INSERT OR REPLACE INTO arc.embeddings
                   SELECT * FROM main.embeddings WHERE observation_id IN ({text_list});
                 DELETE FROM main.embeddings WHERE observation_id IN ({text_list});
                 DELETE FROM main.chunks WHERE id IN ({in_list});"
            ))?;
            tx.commit()?;
            Ok(ids.len())
        })();
        // Always detach, even on error.
        let _ = self.conn.execute("DETACH DATABASE arc", []);
        result
    }

    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// Minimal archive schema (storage only — no triggers or FTS; archive search is
/// vector-based). Kept column-compatible with the active `chunks` table.
const ARCHIVE_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS arc.chunks (
  id                INTEGER PRIMARY KEY,
  session_id        TEXT,
  kind              TEXT NOT NULL,
  content           TEXT NOT NULL,
  title             TEXT,
  file_path         TEXT,
  turn_index        INTEGER,
  ts                INTEGER NOT NULL,
  covered_by_digest INTEGER,
  covered_by_fact   TEXT,
  last_accessed     INTEGER,
  access_count      INTEGER DEFAULT 0,
  archived          INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE IF NOT EXISTS arc.embeddings (
  observation_id TEXT PRIMARY KEY,
  kind           TEXT NOT NULL,
  model          TEXT NOT NULL,
  dim            INTEGER NOT NULL,
  vec            BLOB NOT NULL
);
";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChunkKind;

    fn chunk(session: &str, turn: i64, content: &str) -> NewChunk {
        NewChunk {
            session_id: Some(session.to_string()),
            kind: ChunkKind::Chunk,
            content: content.to_string(),
            title: None,
            file_path: None,
            turn_index: Some(turn),
            ts: 1_000 + turn,
        }
    }

    #[test]
    fn migrates_to_version_1() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.schema_version().unwrap(), 1);
    }

    #[test]
    fn suppresses_same_session_duplicate_fire() {
        let s = Store::open_in_memory().unwrap();
        s.conn()
            .execute(
                "INSERT INTO sessions(id, ide, started_at) VALUES ('s1','test',0)",
                [],
            )
            .unwrap();
        let first = s.insert_chunk(&chunk("s1", 0, "hello")).unwrap();
        let dup = s.insert_chunk(&chunk("s1", 0, "hello")).unwrap();
        assert!(first.is_some());
        assert!(dup.is_none(), "duplicate same-session fire must be ignored");
        assert_eq!(s.active_chunk_count().unwrap(), 1);
    }

    #[test]
    fn keeps_identical_content_across_sessions() {
        let s = Store::open_in_memory().unwrap();
        for sid in ["s1", "s2"] {
            s.conn()
                .execute(
                    "INSERT INTO sessions(id, ide, started_at) VALUES (?1,'test',0)",
                    params![sid],
                )
                .unwrap();
            s.insert_chunk(&chunk(sid, 0, "same content")).unwrap();
        }
        assert_eq!(
            s.active_chunk_count().unwrap(),
            2,
            "identical content in distinct sessions must both persist"
        );
    }

    fn doc(path: &str, body: &str) -> NewChunk {
        NewChunk {
            session_id: None,
            kind: ChunkKind::Doc,
            content: body.to_string(),
            title: None,
            file_path: Some(path.to_string()),
            turn_index: None,
            ts: 0,
        }
    }

    #[test]
    fn prune_docs_removes_only_document_chunks() {
        let s = Store::open_in_memory().unwrap();
        s.conn()
            .execute("INSERT INTO sessions(id, ide, started_at) VALUES ('s1','t',0)", [])
            .unwrap();
        s.insert_chunk(&chunk("s1", 0, "conversation turn")).unwrap();
        s.insert_chunk(&doc("/p/a.md", "doc body")).unwrap();
        assert_eq!(s.prune_docs().unwrap(), 1);
        assert_eq!(s.active_chunk_count().unwrap(), 1);
    }

    #[test]
    fn prune_all_wipes_the_active_database() {
        let s = Store::open_in_memory().unwrap();
        s.conn()
            .execute("INSERT INTO sessions(id, ide, started_at) VALUES ('s1','t',0)", [])
            .unwrap();
        s.insert_chunk(&chunk("s1", 0, "x")).unwrap();
        s.insert_digest(Some("s1"), "session", "d", 0).unwrap();
        s.prune_all().unwrap();
        assert_eq!(s.active_chunk_count().unwrap(), 0);
        let digests: i64 = s
            .conn()
            .query_row("SELECT COUNT(*) FROM digests", [], |r| r.get(0))
            .unwrap();
        assert_eq!(digests, 0);
    }

    #[test]
    fn doc_reindex_replaces_prior_chunks() {
        let s = Store::open_in_memory().unwrap();
        let mk = |path: &str, body: &str| NewChunk {
            session_id: None,
            kind: ChunkKind::Doc,
            content: body.to_string(),
            title: None,
            file_path: Some(path.to_string()),
            turn_index: None,
            ts: 0,
        };
        s.insert_chunk(&mk("/proj/a.md", "old")).unwrap();
        assert_eq!(s.active_chunk_count().unwrap(), 1);
        let removed = s.delete_doc_chunks_under("/proj").unwrap();
        assert_eq!(removed, 1);
        s.insert_chunk(&mk("/proj/a.md", "new")).unwrap();
        assert_eq!(s.active_chunk_count().unwrap(), 1);
    }
}
