//! memorya: local-first memory with capture, recall, consolidation, year-sharded archive.
//! Fact-graph schema ready; population deferred.

pub mod archive;
pub mod capture;
pub mod coverage;
pub mod dual_mode;
pub mod embed;
pub mod entity;
pub mod eval;
pub mod fact;
pub mod hook_source;
pub mod indexer;
pub mod jobs;
pub mod mcp;
pub mod queue;
pub mod search;
pub mod server;
pub mod store;
pub mod tool_use;
pub mod worker;

use serde::{Deserialize, Serialize};

/// A fact in the temporal knowledge graph.
#[derive(Debug, Clone, Serialize)]
pub struct Fact {
    pub id: String,
    pub entity_id: String,
    pub predicate: String,
    pub value: String,
    pub valid_from: i64,
    pub valid_until: Option<i64>,
}

/// Kind of a stored chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkKind {
    /// A conversation turn (user prompt or assistant content).
    Chunk,
    /// A captured tool execution.
    Tool,
    /// A project document section.
    Doc,
}

impl ChunkKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChunkKind::Chunk => "chunk",
            ChunkKind::Tool => "tool",
            ChunkKind::Doc => "doc",
        }
    }
}

/// A chunk to ingest.
#[derive(Debug, Clone)]
pub struct NewChunk {
    pub session_id: Option<String>,
    pub kind: ChunkKind,
    pub content: String,
    pub title: Option<String>,
    pub file_path: Option<String>,
    pub turn_index: Option<i64>,
    pub ts: i64,
}

/// A single recall hit.
#[derive(Debug, Clone, Serialize)]
pub struct RecallHit {
    pub id: i64,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

/// Result of a recall query.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum RecallResult {
    Found { hits: Vec<RecallHit> },
    Uncertain { offer_archive: bool },
}

/// A search result carrying full content (the one-shot recall+expand).
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub id: i64,
    pub title: String,
    pub score: f32,
    pub content: String,
}

/// Session-start injection context.
#[derive(Debug, Clone, Serialize)]
pub struct InjectContext {
    pub recent_digests: Vec<String>,
    pub projects: Vec<String>,
}

/// A title, or a short prefix of the content when no title is set.
pub(crate) fn title_or_prefix(title: Option<String>, content: &str) -> String {
    title.unwrap_or_else(|| content.chars().take(60).collect())
}

/// Default static embedding model (downloaded on first use, cached thereafter).
pub const DEFAULT_MODEL_REPO: &str = "minishlab/potion-retrieval-32M";

/// Recall ranking and uncertainty thresholds.
const RECALL_K: usize = 10;
const CONFIDENCE_FLOOR: f32 = 0.30;
/// Max chunks moved to archive per eviction run.
const EVICT_BATCH: usize = 500;

/// The single writer. No other component may write to or open the database.
pub struct Engram {
    store: store::Store,
    embedder: Box<dyn embed::Embedder>,
    dir: std::path::PathBuf,
}

impl Engram {
    /// Open (creating + migrating if needed) the active database at `path`,
    /// using the default static embedding model (downloaded on first use and
    /// cached). Tests use a deterministic in-crate stub at the same boundary.
    pub fn open(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        #[cfg(test)]
        let embedder: Box<dyn embed::Embedder> = Box::new(embed::StubEmbedder::default());
        #[cfg(not(test))]
        let embedder: Box<dyn embed::Embedder> =
            Box::new(embed::Model2VecEmbedder::from_repo(DEFAULT_MODEL_REPO)?);
        Self::open_with(path, embedder)
    }

    /// Open with a specific embedder.
    pub fn open_with(
        path: impl AsRef<std::path::Path>,
        embedder: Box<dyn embed::Embedder>,
    ) -> anyhow::Result<Self> {
        let dir = path
            .as_ref()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let store = store::Store::open(path)?;
        // Vectors from a different embedding model are not comparable; drop them
        // so recall only uses the active model. Backfill happens out-of-band.
        store.drop_embeddings_for_other_models(embedder.model_id())?;
        Ok(Self {
            store,
            embedder,
            dir,
        })
    }

    /// The archive router for this instance's data directory.
    pub fn archive(&self) -> archive::ArchiveRouter {
        archive::ArchiveRouter::new(&self.dir)
    }

    /// Search the archive shards newest-first for up to `k` hits.
    pub fn archive_recall(&self, query: &str, k: usize) -> anyhow::Result<Vec<RecallHit>> {
        let router = self.archive();
        let mut hits = Vec::new();
        for shard in router.shards_newest_first()? {
            let mut shard_hits = router.search_shard(&shard, self.embedder.as_ref(), query, k)?;
            hits.append(&mut shard_hits);
            if hits.len() >= k {
                break;
            }
        }
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(k);
        Ok(hits)
    }

    /// Ingest a chunk as-is. Same-session duplicate fires are suppressed. The
    /// chunk is committed first, then an embedding request is enqueued, so a
    /// crash never enqueues a request for an uncommitted chunk.
    pub fn ingest(&self, chunk: NewChunk) -> anyhow::Result<Option<i64>> {
        let ts = chunk.ts;
        let id = self.store.insert_chunk(&chunk)?;
        if let Some(id) = id {
            queue::enqueue(&self.store, id, ts)?;
        }
        Ok(id)
    }

    /// Capture a user prompt as a conversation chunk, redacting sensitive
    /// content before any write.
    pub fn capture_prompt(
        &self,
        session_id: Option<&str>,
        content: &str,
        turn_index: Option<i64>,
        ts: i64,
    ) -> anyhow::Result<Option<i64>> {
        self.ingest(NewChunk {
            session_id: session_id.map(String::from),
            kind: ChunkKind::Chunk,
            content: redact::redact(content),
            title: None,
            file_path: None,
            turn_index,
            ts,
        })
    }

    /// Capture a post-tool event: skip low-value tools, derive a title, redact
    /// the title and response, and ingest as a `tool` chunk. Returns the new
    /// chunk id, or `None` if the tool was skipped or the fire was a duplicate.
    pub fn capture_tool(
        &self,
        session_id: &str,
        tool_name: &str,
        tool_input: &serde_json::Value,
        tool_response: &str,
        turn_index: i64,
        ts: i64,
    ) -> anyhow::Result<Option<i64>> {
        if tool_use::should_skip(tool_name) {
            return Ok(None);
        }
        let title = redact::redact(&tool_use::auto_title(tool_name, tool_input));
        let body = redact::redact(tool_response);
        self.ingest(NewChunk {
            session_id: Some(session_id.to_string()),
            kind: ChunkKind::Tool,
            content: format!("{title}\n{body}"),
            title: Some(title),
            file_path: None,
            turn_index: Some(turn_index),
            ts,
        })
    }

    /// Re-index the markdown documents under `cwd` into `doc` chunks.
    pub fn index_project(
        &self,
        cwd: impl AsRef<std::path::Path>,
        ts: i64,
    ) -> anyhow::Result<usize> {
        indexer::index_project(self, cwd.as_ref(), ts)
    }

    /// Delete every indexed document chunk. Returns the number removed.
    pub fn prune_docs(&self) -> anyhow::Result<usize> {
        self.store.prune_docs()
    }

    /// Wipe all memory in the active database. Archive shards are left untouched.
    pub fn prune_all(&self) -> anyhow::Result<()> {
        self.store.prune_all()
    }

    /// Ensure a session row exists (idempotent across resume/clear/compact).
    pub fn ensure_session(
        &self,
        id: &str,
        ide: &str,
        cwd: Option<&str>,
        ts: i64,
    ) -> anyhow::Result<()> {
        self.store.ensure_session(id, ide, cwd, ts)
    }

    /// Most recent active chunks (for the viewer): `(id, title, content)`.
    pub fn recent_chunks(&self, limit: i64) -> anyhow::Result<Vec<(i64, Option<String>, String)>> {
        self.store.recent_chunks(limit)
    }

    /// Current four-digit year from the store clock.
    pub fn current_year(&self) -> anyhow::Result<i64> {
        self.store.current_year()
    }

    /// Roll a session's uncovered chunks into a session digest.
    pub fn consolidate_session(&self, session_id: &str, ts: i64) -> anyhow::Result<Option<i64>> {
        jobs::session_end(&self.store, session_id, ts)
    }

    /// Roll the level below `scope` into a `scope`-scoped digest.
    pub fn consolidate(&self, scope: jobs::Scope, ts: i64) -> anyhow::Result<Option<i64>> {
        jobs::consolidate(&self.store, scope, ts)
    }

    /// Sessions carrying enough aged, uncovered chunks to warrant summarization.
    pub fn coverage_debt(&self, before_ts: i64) -> anyhow::Result<Vec<String>> {
        jobs::coverage_debt(&self.store, before_ts)
    }

    /// Run eviction into the current year's archive shard (creating it if
    /// needed). Returns the number of chunks archived.
    pub fn run_eviction(&self, now_ts: i64, threshold: f32) -> anyhow::Result<usize> {
        let year = self.store.current_year()?;
        let shard = self.archive().ensure_shard(year)?;
        let path = shard.to_string_lossy().to_string();
        self.evict(now_ts, threshold, &path)
    }

    /// Score active chunks and move those above `threshold` into the archive at
    /// `archive_path`, in batches. Returns the number archived. Nothing is
    /// permanently deleted -- chunks are moved, not dropped.
    pub fn evict(&self, now_ts: i64, threshold: f32, archive_path: &str) -> anyhow::Result<usize> {
        let mut scored: Vec<(i64, f32)> = self
            .store
            .evictable_chunk_stats()?
            .iter()
            .map(|c| (c.id, coverage::eviction_score(c, now_ts)))
            .filter(|(_, s)| *s > threshold)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(EVICT_BATCH);
        let ids: Vec<i64> = scored.into_iter().map(|(id, _)| id).collect();
        self.store.move_chunks_to_archive(archive_path, &ids)
    }

    /// Number of currently active (non-archived) chunks.
    pub fn active_chunk_count(&self) -> anyhow::Result<i64> {
        self.store.active_chunk_count()
    }

    /// Embed any chunks still missing an embedding for the current model.
    /// Runs out-of-band of ingestion; returns the number embedded.
    pub fn embed_pending(&self, limit: i64) -> anyhow::Result<usize> {
        embed::embed_pending(&self.store, self.embedder.as_ref(), limit)
    }

    /// Reset embedding requests left in-flight by an abandoned run back to
    /// pending. Run on worker startup. Returns the number reclaimed.
    pub fn reclaim_stale_captures(&self) -> anyhow::Result<usize> {
        queue::reclaim_stale(&self.store)
    }

    /// Drain up to `batch_size` queued requests and embed each chunk. A panic
    /// while embedding one chunk is caught and that request rescheduled, so a
    /// single bad input never kills the worker. Returns the number embedded.
    pub fn drain_and_embed_captures(&self, batch_size: i64) -> anyhow::Result<usize> {
        let batch = queue::drain_batch(&self.store, batch_size)?;
        let model = self.embedder.model_id().to_string();
        let mut embedded = 0;
        for (chunk_id, content) in batch {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.embedder.embed(&content)
            }));
            match outcome {
                Ok(vec) => {
                    self.store
                        .put_embedding(&chunk_id.to_string(), "chunk", &model, &vec)?;
                    queue::mark_embedded(&self.store, chunk_id)?;
                    embedded += 1;
                }
                Err(_) => queue::mark_failed(&self.store, chunk_id, "embedding panicked")?,
            }
        }
        Ok(embedded)
    }

    /// The number of embedding requests still pending.
    pub fn pending_capture_count(&self) -> anyhow::Result<i64> {
        queue::pending_count(&self.store)
    }

    /// Fuse vector + lexical results with query-adaptive weighting and a recency
    /// rerank. Returns the ranked scores and the best vector cosine (the
    /// confidence signal).
    fn ranked(&self, query: &str, now: i64) -> anyhow::Result<(Vec<search::Scored>, f32, bool)> {
        let vec_hits = search::vector_search(&self.store, self.embedder.as_ref(), query, RECALL_K)?;
        let lex_hits = search::lexical_search(&self.store, query, RECALL_K)?;
        let best_cos = vec_hits.first().map(|s| s.score).unwrap_or(0.0);
        let lex_empty = lex_hits.is_empty();

        let (vec_w, lex_w) = if search::is_symbol_like(query) {
            (0.7, 1.5)
        } else {
            (1.0, 1.0)
        };
        let mut reranked: Vec<search::Scored> =
            search::rrf_fuse_weighted(&vec_hits, &lex_hits, 60.0, vec_w, lex_w)
                .into_iter()
                .map(|mut s| {
                    let ts = self.store.chunk_ts(s.id).ok().flatten().unwrap_or(0);
                    s.score *= search::recency_boost(now, ts);
                    s
                })
                .collect();
        reranked.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok((reranked, best_cos, lex_empty))
    }

    /// Ranked chunk ids for a query, ignoring the uncertainty gate. Used by
    /// evaluation to measure pure ranking quality.
    pub fn rank(&self, query: &str, now: i64, k: usize) -> anyhow::Result<Vec<i64>> {
        let (ranked, _, _) = self.ranked(query, now)?;
        Ok(ranked.into_iter().take(k).map(|s| s.id).collect())
    }

    /// Hybrid recall over conversation and doc chunks. Returns ranked hits, or
    /// an uncertain result offering an archive search when confidence is low.
    pub fn recall(&self, query: &str, now: i64) -> anyhow::Result<RecallResult> {
        let (reranked, best_cos, lex_empty) = self.ranked(query, now)?;
        if reranked.is_empty() || (best_cos < CONFIDENCE_FLOOR && lex_empty) {
            return Ok(RecallResult::Uncertain {
                offer_archive: true,
            });
        }
        let mut hits = Vec::new();
        for s in reranked.into_iter().take(RECALL_K) {
            if let Some((title, content)) = self.store.chunk_brief(s.id)? {
                self.store.touch_chunk(s.id, now)?;
                let snippet: String = content.chars().take(160).collect();
                hits.push(RecallHit {
                    id: s.id,
                    title,
                    snippet,
                    score: s.score,
                });
            }
        }
        Ok(RecallResult::Found { hits })
    }

    /// Assemble session-start context: recent daily digests (subject to a
    /// staleness check) and the list of known project directories.
    pub fn context(&self, recent_n: i64) -> anyhow::Result<InjectContext> {
        let last_chunk = self.store.last_chunk_ts()?.unwrap_or(i64::MIN);
        let recent_digests = self
            .store
            .recent_digests("daily", recent_n)?
            .into_iter()
            // Staleness: include a digest only when no newer captured content
            // exists after it.
            .filter(|(_, ts)| *ts >= last_chunk)
            .map(|(content, _)| content)
            .collect();
        Ok(InjectContext {
            recent_digests,
            projects: self.store.project_cwds()?,
        })
    }

    /// One-shot search: recall plus full content of the top `k` hits, so callers
    /// don't need a second `expand` round-trip. Returns an empty vec when recall
    /// is uncertain.
    pub fn search(&self, query: &str, now: i64, k: usize) -> anyhow::Result<Vec<SearchResult>> {
        let hits = match self.recall(query, now)? {
            RecallResult::Found { hits } => hits,
            RecallResult::Uncertain { .. } => return Ok(vec![]),
        };
        let mut out = Vec::new();
        for h in hits.into_iter().take(k) {
            if let Some((_, content)) = self.store.chunk_brief(h.id)? {
                out.push(SearchResult {
                    id: h.id,
                    title: h.title,
                    score: h.score,
                    content,
                });
            }
        }
        Ok(out)
    }

    /// Full content of a chunk by id (the expansion layer).
    pub fn expand(&self, id: i64) -> anyhow::Result<Option<String>> {
        Ok(self.store.chunk_brief(id)?.map(|(_, content)| content))
    }

    /// Full content for several chunk ids at once, in request order. Missing
    /// ids are skipped.
    pub fn expand_many(&self, ids: &[i64]) -> anyhow::Result<Vec<(i64, String)>> {
        self.store.chunk_contents(ids)
    }

    /// Record a fact, superseding any currently-valid fact about the same
    /// entity + predicate. Returns the new fact id.
    pub fn learn(
        &self,
        entity_name: &str,
        predicate: &str,
        value: &str,
        ts: i64,
    ) -> anyhow::Result<String> {
        fact::learn(&self.store, entity_name, predicate, value, ts)
    }

    /// Soft-remove the currently-valid fact about an entity + predicate by
    /// setting its validity end. History is preserved.
    pub fn forget(&self, entity_name: &str, predicate: &str, ts: i64) -> anyhow::Result<()> {
        fact::remove(&self.store, entity_name, predicate, ts)
    }

    /// All currently-valid facts for a named entity.
    pub fn entity(&self, name: &str) -> anyhow::Result<Vec<Fact>> {
        fact::currently_valid_for_entity(&self.store, name)
    }

    pub(crate) fn store(&self) -> &store::Store {
        &self.store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(content: &str) -> NewChunk {
        NewChunk {
            session_id: None,
            kind: ChunkKind::Doc,
            content: content.to_string(),
            title: Some("doc".into()),
            file_path: Some("/proj/readme.md".into()),
            turn_index: None,
            ts: 0,
        }
    }

    #[test]
    fn recall_finds_ingested_content_after_embedding() {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        e.ingest(doc("the project uses postgres for the main datastore"))
            .unwrap();
        e.ingest(doc("the renderer draws shadow volumes")).unwrap();
        e.embed_pending(100).unwrap();

        match e.recall("which database does the project use", 10).unwrap() {
            RecallResult::Found { hits } => {
                assert!(!hits.is_empty());
                assert!(hits[0].snippet.contains("postgres"));
            }
            RecallResult::Uncertain { .. } => panic!("expected a confident hit"),
        }
    }

    #[test]
    fn search_returns_full_content_in_one_call() {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        e.ingest(doc(
            "the project uses postgres as the main datastore for everything",
        ))
        .unwrap();
        e.embed_pending(100).unwrap();

        let results = e.search("postgres datastore", 10, 5).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0]
            .content
            .contains("postgres as the main datastore"));
    }

    #[test]
    fn search_is_empty_when_recall_is_uncertain() {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        assert!(e.search("anything", 0, 5).unwrap().is_empty());
    }

    #[test]
    fn opening_with_a_new_model_drops_stale_vectors_and_backfills() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("memorya.db");

        {
            let e = Engram::open_with(&db, Box::new(embed::StubEmbedder::with_id(256, "model-a")))
                .unwrap();
            e.ingest(doc("the cache uses an LRU eviction policy"))
                .unwrap();
            e.embed_pending(100).unwrap();
            let n: i64 = e
                .store()
                .conn()
                .query_row(
                    "SELECT COUNT(*) FROM embeddings WHERE model='model-a'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1);
        }

        let e =
            Engram::open_with(&db, Box::new(embed::StubEmbedder::with_id(256, "model-b"))).unwrap();
        let stale: i64 = e
            .store()
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM embeddings WHERE model='model-a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stale, 0, "vectors from the previous model are dropped");

        e.embed_pending(100).unwrap();
        assert!(matches!(
            e.recall("LRU eviction cache", 0).unwrap(),
            RecallResult::Found { .. }
        ));
    }

    #[test]
    fn recall_on_empty_store_is_uncertain() {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        assert!(matches!(
            e.recall("anything", 0).unwrap(),
            RecallResult::Uncertain {
                offer_archive: true
            }
        ));
    }

    #[test]
    fn evict_then_archive_recall_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("memorya.db");
        let e = Engram::open(&db).unwrap();
        e.store()
            .conn()
            .execute(
                "INSERT INTO sessions(id, ide, started_at) VALUES ('s1','test',0)",
                [],
            )
            .unwrap();
        e.ingest(NewChunk {
            session_id: Some("s1".into()),
            kind: ChunkKind::Chunk,
            content: "the redis cache eviction policy was tuned".into(),
            title: None,
            file_path: None,
            turn_index: Some(0),
            ts: 0,
        })
        .unwrap();
        e.embed_pending(100).unwrap();
        jobs::session_end(e.store(), "s1", 1).unwrap();

        let shard = e.archive().ensure_shard(2026).unwrap();
        let now = 60 * 86_400;
        let moved = e.evict(now, 1.0, shard.to_str().unwrap()).unwrap();
        assert_eq!(moved, 1);
        assert_eq!(
            e.active_chunk_count().unwrap(),
            0,
            "chunk left the active set"
        );

        assert!(matches!(
            e.recall("redis cache eviction", now).unwrap(),
            RecallResult::Uncertain { .. }
        ));
        let arch = e.archive_recall("redis cache eviction", 5).unwrap();
        assert!(
            !arch.is_empty(),
            "archive search recovers the evicted chunk"
        );
        assert!(arch[0].snippet.contains("redis"));
    }

    /// A store with one session, one project, and a chunk at ts=100.
    fn memorya_with_recent_chunk() -> (tempfile::TempDir, Engram) {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        e.store()
            .conn()
            .execute(
                "INSERT INTO sessions(id, ide, cwd, started_at) VALUES ('s1','test','/proj',0)",
                [],
            )
            .unwrap();
        e.ingest(NewChunk {
            session_id: Some("s1".into()),
            kind: ChunkKind::Chunk,
            content: "recent work".into(),
            title: None,
            file_path: None,
            turn_index: Some(0),
            ts: 100,
        })
        .unwrap();
        (dir, e)
    }

    #[test]
    fn context_withholds_a_digest_older_than_the_latest_chunk() {
        let (_d, e) = memorya_with_recent_chunk();
        e.store()
            .insert_digest(Some("s1"), "daily", "stale daily summary", 50)
            .unwrap();
        assert!(e.context(5).unwrap().recent_digests.is_empty());
    }

    #[test]
    fn context_includes_a_digest_newer_than_all_chunks() {
        let (_d, e) = memorya_with_recent_chunk();
        e.store()
            .insert_digest(Some("s1"), "daily", "fresh daily summary", 200)
            .unwrap();
        assert_eq!(
            e.context(5).unwrap().recent_digests,
            vec!["fresh daily summary".to_string()]
        );
    }

    #[test]
    fn context_lists_known_projects() {
        let (_d, e) = memorya_with_recent_chunk();
        assert_eq!(e.context(5).unwrap().projects, vec!["/proj".to_string()]);
    }

    #[test]
    fn expand_many_returns_contents_in_request_order_skipping_missing() {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        let a = e.ingest(doc("first")).unwrap().unwrap();
        let b = e.ingest(doc("second")).unwrap().unwrap();

        let got = e.expand_many(&[b, 9999, a]).unwrap();
        assert_eq!(
            got,
            vec![(b, "second".to_string()), (a, "first".to_string())]
        );
    }

    #[test]
    fn ingest_commits_chunk_before_enqueuing_embedding_request() {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();

        e.ingest(doc("a committed chunk")).unwrap().unwrap();

        // The enqueue's foreign key only holds if the chunk was committed first.
        assert_eq!(e.active_chunk_count().unwrap(), 1);
        assert_eq!(e.pending_capture_count().unwrap(), 1);
    }

    #[test]
    fn embedding_request_enqueued_even_when_worker_not_running() {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();

        e.ingest(doc("first")).unwrap();
        e.ingest(doc("second")).unwrap();

        // No worker drains the queue here, so both requests remain.
        assert_eq!(e.pending_capture_count().unwrap(), 2);
    }

    #[test]
    fn content_hash_dedup_suppresses_duplicate_prompt_without_turn_index() {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        e.ensure_session("s1", "test", None, 0).unwrap();

        let first = e
            .capture_prompt(Some("s1"), "a prompt with no turn index", None, 1)
            .unwrap();
        let second = e
            .capture_prompt(Some("s1"), "a prompt with no turn index", None, 2)
            .unwrap();

        assert!(first.is_some());
        assert!(
            second.is_none(),
            "the structural index misses NULL turn_index; content-hash catches it"
        );
        assert_eq!(e.active_chunk_count().unwrap(), 1);
        assert_eq!(e.pending_capture_count().unwrap(), 1);
    }

    #[test]
    fn learn_and_entity_through_public_api() {
        let dir = tempfile::tempdir().unwrap();
        let e = Engram::open(dir.path().join("memorya.db")).unwrap();
        e.learn("user", "prefers_pm", "bun", 1).unwrap();
        e.learn("user", "prefers_pm", "pnpm", 2).unwrap();
        let facts = e.entity("user").unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].value, "pnpm");
    }
}
