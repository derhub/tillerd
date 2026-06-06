//! Year-sharded archive. Evicted chunks live in `archive-YYYY.db` files
//! recorded in `archive-index.json`. Sealed shards are opened read-only and
//! searched newest-first on demand.

use crate::embed::{cosine, decode_vec, Embedder};
use crate::RecallHit;
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMeta {
    pub file: String,
    pub year: i64,
    pub sealed: bool,
}

pub struct ArchiveRouter {
    dir: PathBuf,
}

impl ArchiveRouter {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    fn index_path(&self) -> PathBuf {
        self.dir.join("archive-index.json")
    }

    pub fn load(&self) -> anyhow::Result<Vec<ShardMeta>> {
        let p = self.index_path();
        if !p.exists() {
            return Ok(vec![]);
        }
        Ok(serde_json::from_str(&std::fs::read_to_string(p)?)?)
    }

    fn save(&self, shards: &[ShardMeta]) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(self.index_path(), serde_json::to_string_pretty(shards)?)?;
        Ok(())
    }

    /// Path of the shard for `year`, registering it (and sealing older shards)
    /// if it does not yet exist.
    pub fn ensure_shard(&self, year: i64) -> anyhow::Result<PathBuf> {
        let mut shards = self.load()?;
        if let Some(s) = shards.iter().find(|s| s.year == year) {
            return Ok(self.dir.join(&s.file));
        }
        // Seal every shard for an earlier year; new writes go to `year`.
        for s in shards.iter_mut() {
            if s.year < year {
                s.sealed = true;
            }
        }
        let file = format!("archive-{year}.db");
        shards.push(ShardMeta {
            file: file.clone(),
            year,
            sealed: false,
        });
        self.save(&shards)?;
        Ok(self.dir.join(file))
    }

    /// Shards ordered newest year first.
    pub fn shards_newest_first(&self) -> anyhow::Result<Vec<ShardMeta>> {
        let mut shards = self.load()?;
        shards.sort_by_key(|s| std::cmp::Reverse(s.year));
        Ok(shards)
    }

    /// Vector search over a single sealed/unsealed shard, opened read-only.
    pub fn search_shard(
        &self,
        shard: &ShardMeta,
        embedder: &dyn Embedder,
        query: &str,
        k: usize,
    ) -> anyhow::Result<Vec<RecallHit>> {
        let path = self.dir.join(&shard.file);
        if !path.exists() {
            return Ok(vec![]);
        }
        let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let qv = embedder.embed(query);

        let mut stmt = conn.prepare(
            "SELECT c.id, c.title, c.content, e.vec
             FROM embeddings e JOIN chunks c ON CAST(c.id AS TEXT) = e.observation_id
             WHERE e.model = ?1",
        )?;
        let rows = stmt.query_map(params![embedder.model_id()], |r| {
            let id: i64 = r.get(0)?;
            let title: Option<String> = r.get(1)?;
            let content: String = r.get(2)?;
            let bytes: Vec<u8> = r.get(3)?;
            Ok((id, title, content, bytes))
        })?;

        let mut hits: Vec<RecallHit> = Vec::new();
        for row in rows {
            let (id, title, content, bytes) = row?;
            let score = cosine(&qv, &decode_vec(&bytes));
            let snippet: String = content.chars().take(160).collect();
            hits.push(RecallHit {
                id,
                title: crate::title_or_prefix(title, &content),
                snippet,
                score,
            });
        }
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(k);
        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_shard_names_the_file_by_year() {
        let dir = tempfile::tempdir().unwrap();
        let r = ArchiveRouter::new(dir.path());
        assert!(r.ensure_shard(2025).unwrap().ends_with("archive-2025.db"));
    }

    #[test]
    fn opening_a_newer_shard_seals_older_ones() {
        let dir = tempfile::tempdir().unwrap();
        let r = ArchiveRouter::new(dir.path());
        r.ensure_shard(2025).unwrap();
        r.ensure_shard(2026).unwrap();
        let sealed: Vec<(i64, bool)> = r
            .shards_newest_first()
            .unwrap()
            .iter()
            .map(|s| (s.year, s.sealed))
            .collect();
        assert_eq!(sealed, vec![(2026, false), (2025, true)]);
    }

    #[test]
    fn ensure_shard_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let r = ArchiveRouter::new(dir.path());
        let a = r.ensure_shard(2026).unwrap();
        let b = r.ensure_shard(2026).unwrap();
        assert_eq!(a, b);
        assert_eq!(r.load().unwrap().len(), 1);
    }
}
