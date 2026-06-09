//! Search: hybrid brute-force cosine + FTS5. No ANN index (sub-millisecond).

use crate::embed::{cosine, Embedder};
use crate::store::Store;
use rusqlite::params;

/// A scored chunk id from one retriever.
#[derive(Debug, Clone)]
pub struct Scored {
    pub id: i64,
    pub score: f32,
}

/// Brute-force vector search over non-archived chunk embeddings. Returns the
/// top `k` chunk ids by cosine similarity to the embedded query.
pub fn vector_search(
    store: &Store,
    embedder: &dyn Embedder,
    query: &str,
    k: usize,
) -> anyhow::Result<Vec<Scored>> {
    let qv = embedder.embed(query);
    let mut scored: Vec<Scored> = store
        .load_chunk_vectors(embedder.model_id())?
        .into_iter()
        .map(|(id, v)| Scored {
            id,
            score: cosine(&qv, &v),
        })
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(k);
    Ok(scored)
}

/// Lexical BM25 search over the chunk full-text index. Returns the top `k`
/// chunk ids; scores are normalized so higher is better.
pub fn lexical_search(store: &Store, query: &str, k: usize) -> anyhow::Result<Vec<Scored>> {
    let m = sanitize_match(query);
    if m.is_empty() {
        return Ok(vec![]);
    }
    let conn = store.conn();
    let mut stmt = conn.prepare(
        "SELECT c.id, bm25(chunks_fts) AS score
         FROM chunks_fts
         JOIN chunks c ON c.id = chunks_fts.rowid
         WHERE chunks_fts MATCH ?1 AND c.archived = 0
         ORDER BY score ASC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![m, k as i64], |r| {
        let id: i64 = r.get(0)?;
        let bm25: f64 = r.get(1)?;
        Ok(Scored {
            id,
            // bm25 is "lower is better"; flip the sign so higher = better.
            score: -bm25 as f32,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Fuse vector and lexical rankings with Reciprocal Rank Fusion (equal weight).
pub fn rrf_fuse(vector: &[Scored], lexical: &[Scored], k_const: f32) -> Vec<Scored> {
    rrf_fuse_weighted(vector, lexical, k_const, 1.0, 1.0)
}

/// Weighted Reciprocal Rank Fusion. `vec_w`/`lex_w` bias the two retrievers.
pub fn rrf_fuse_weighted(
    vector: &[Scored],
    lexical: &[Scored],
    k_const: f32,
    vec_w: f32,
    lex_w: f32,
) -> Vec<Scored> {
    use std::collections::HashMap;
    let mut acc: HashMap<i64, f32> = HashMap::new();
    for (list, w) in [(vector, vec_w), (lexical, lex_w)] {
        for (rank, s) in list.iter().enumerate() {
            *acc.entry(s.id).or_insert(0.0) += w / (k_const + rank as f32 + 1.0);
        }
    }
    let mut fused: Vec<Scored> = acc
        .into_iter()
        .map(|(id, score)| Scored { id, score })
        .collect();
    fused.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    fused
}

/// Whether a query looks like a code symbol rather than prose, so lexical
/// matching should be weighted higher. Heuristic: few tokens and identifier-ish
/// characters (`::`, `_`, camelCase, `.`).
pub fn is_symbol_like(query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return false;
    }
    let tokens = q.split_whitespace().count();
    let has_sym = q.contains("::") || q.contains('_') || q.contains('.');
    let has_camel = q
        .chars()
        .zip(q.chars().skip(1))
        .any(|(a, b)| a.is_ascii_lowercase() && b.is_ascii_uppercase());
    tokens <= 2 && (has_sym || has_camel)
}

/// Recency multiplier in [1, 2]: recent items get up to a 2x boost, old items ~1x.
pub fn recency_boost(now: i64, ts: i64) -> f32 {
    let days = ((now - ts).max(0) as f32) / 86_400.0;
    1.0 + 1.0 / (1.0 + days)
}

/// Escape FTS5 query terms: wrap each bare token in quotes (so arbitrary user
/// input can't trigger an FTS5 syntax error) and OR them together, so recall is
/// forgiving — any matching term contributes rather than requiring all of them.
pub fn sanitize_match(q: &str) -> String {
    q.split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::StubEmbedder;
    use crate::{ChunkKind, NewChunk};

    fn seed(store: &Store, id_session: &str, turn: i64, content: &str) {
        store
            .conn()
            .execute(
                "INSERT OR IGNORE INTO sessions(id, ide, started_at) VALUES (?1,'test',0)",
                params![id_session],
            )
            .unwrap();
        store
            .insert_chunk(&NewChunk {
                session_id: Some(id_session.to_string()),
                kind: ChunkKind::Chunk,
                content: content.to_string(),
                title: None,
                file_path: None,
                turn_index: Some(turn),
                ts: turn,
            })
            .unwrap();
    }

    #[test]
    fn sanitize_quotes_and_ors_terms() {
        assert_eq!(sanitize_match("auth token"), "\"auth\" OR \"token\"");
        assert_eq!(sanitize_match("   "), "");
    }

    #[test]
    fn vector_search_ranks_relevant_first() {
        let s = Store::open_in_memory().unwrap();
        let e = StubEmbedder::default();
        seed(&s, "s1", 0, "database connection pool tuning");
        seed(&s, "s1", 1, "shadow volume rendering technique");
        crate::embed::embed_pending(&s, &e, 100).unwrap();
        let hits = vector_search(&s, &e, "database pool", 5).unwrap();
        assert!(!hits.is_empty());
        let top = hits[0].id;
        let (_, content) = s.chunk_brief(top).unwrap().unwrap();
        assert!(content.contains("database"));
    }

    #[test]
    fn lexical_search_finds_token() {
        let s = Store::open_in_memory().unwrap();
        seed(&s, "s1", 0, "the auth middleware throws a 401");
        seed(&s, "s1", 1, "unrelated content about colors");
        let hits = lexical_search(&s, "middleware", 5).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn rrf_prefers_items_ranked_by_both() {
        let v = vec![Scored { id: 1, score: 0.9 }, Scored { id: 2, score: 0.8 }];
        let l = vec![Scored { id: 2, score: 0.7 }, Scored { id: 3, score: 0.6 }];
        let fused = rrf_fuse(&v, &l, 60.0);
        assert_eq!(
            fused[0].id, 2,
            "item present in both lists should rank first"
        );
    }

    #[test]
    fn detects_symbol_like_queries() {
        assert!(is_symbol_like("Foo::bar"));
        assert!(is_symbol_like("getUserById"));
        assert!(is_symbol_like("config_parser"));
        assert!(!is_symbol_like("how is auth handled"));
        assert!(!is_symbol_like(""));
    }

    #[test]
    fn recency_boost_favors_recent() {
        let now = 100 * 86_400;
        assert!(recency_boost(now, now) > recency_boost(now, 0));
    }
}
