//! Jobs: consolidation ladder by aggregation. No language model.

use crate::store::Store;

const JOIN: &str = "\n---\n";

/// The four digest scopes, lowest to highest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Session,
    Daily,
    Weekly,
    Monthly,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Session => "session",
            Scope::Daily => "daily",
            Scope::Weekly => "weekly",
            Scope::Monthly => "monthly",
        }
    }

    /// The scope one level below this one, if any.
    pub fn below(self) -> Option<Scope> {
        match self {
            Scope::Session => None,
            Scope::Daily => Some(Scope::Session),
            Scope::Weekly => Some(Scope::Daily),
            Scope::Monthly => Some(Scope::Weekly),
        }
    }
}

/// Roll a session's uncovered chunks into a session digest and mark them
/// covered. Returns the new digest id, or `None` if there was nothing to do.
pub fn session_end(store: &Store, session_id: &str, ts: i64) -> anyhow::Result<Option<i64>> {
    let chunks = store.session_uncovered_chunks(session_id)?;
    if chunks.is_empty() {
        return Ok(None);
    }
    let content = chunks
        .iter()
        .map(|(_, c)| c.as_str())
        .collect::<Vec<_>>()
        .join(JOIN);
    let digest_id = store.insert_digest(Some(session_id), Scope::Session.as_str(), &content, ts)?;
    store.cover_session_chunks(session_id, digest_id)?;
    Ok(Some(digest_id))
}

/// Roll the uncovered digests one level below `target` into a `target`-scoped
/// digest. No model call. Returns the new digest id, or `None` if nothing to do.
pub fn consolidate(store: &Store, target: Scope, ts: i64) -> anyhow::Result<Option<i64>> {
    let Some(below) = target.below() else {
        return Ok(None); // session digests are produced by `session_end`
    };
    let sources = store.uncovered_digests(below.as_str())?;
    if sources.is_empty() {
        return Ok(None);
    }
    let content = sources
        .iter()
        .map(|(_, c)| c.as_str())
        .collect::<Vec<_>>()
        .join(JOIN);
    let parent = store.insert_digest(None, target.as_str(), &content, ts)?;
    let ids: Vec<i64> = sources.iter().map(|(id, _)| *id).collect();
    store.cover_digests(&ids, parent)?;
    Ok(Some(parent))
}

/// Sessions carrying enough uncovered, aged chunks to warrant summarization.
pub fn coverage_debt(store: &Store, before_ts: i64) -> anyhow::Result<Vec<String>> {
    store.sessions_with_coverage_debt(5, before_ts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChunkKind, NewChunk};
    use rusqlite::params;

    fn store_with_session(sid: &str) -> Store {
        let s = Store::open_in_memory().unwrap();
        s.conn()
            .execute(
                "INSERT INTO sessions(id, ide, started_at) VALUES (?1,'test',0)",
                params![sid],
            )
            .unwrap();
        s
    }

    fn add(s: &Store, sid: &str, turn: i64, body: &str) {
        s.insert_chunk(&NewChunk {
            session_id: Some(sid.to_string()),
            kind: ChunkKind::Chunk,
            content: body.to_string(),
            title: None,
            file_path: None,
            turn_index: Some(turn),
            ts: turn,
        })
        .unwrap();
    }

    #[test]
    fn session_end_aggregates_chunks_into_one_digest() {
        let s = store_with_session("s1");
        add(&s, "s1", 0, "alpha");
        add(&s, "s1", 1, "beta");
        let id = session_end(&s, "s1", 10).unwrap().unwrap();

        let content: String = s
            .conn()
            .query_row(
                "SELECT content FROM digests WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(content.contains("alpha") && content.contains("beta"));
    }

    #[test]
    fn session_end_is_a_noop_once_all_chunks_are_covered() {
        let s = store_with_session("s1");
        add(&s, "s1", 0, "alpha");
        session_end(&s, "s1", 10).unwrap();
        assert!(session_end(&s, "s1", 11).unwrap().is_none());
    }

    #[test]
    fn consolidating_daily_rolls_up_session_digests() {
        let s = store_with_session("s1");
        add(&s, "s1", 0, "work today");
        session_end(&s, "s1", 10).unwrap();
        let daily = consolidate(&s, Scope::Daily, 20).unwrap().unwrap();

        let scope: String = s
            .conn()
            .query_row(
                "SELECT scope FROM digests WHERE id = ?1",
                params![daily],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(scope, "daily");
    }

    #[test]
    fn consolidating_daily_is_a_noop_once_sources_are_covered() {
        let s = store_with_session("s1");
        add(&s, "s1", 0, "work today");
        session_end(&s, "s1", 10).unwrap();
        consolidate(&s, Scope::Daily, 20).unwrap();
        assert!(consolidate(&s, Scope::Daily, 21).unwrap().is_none());
    }

    #[test]
    fn coverage_debt_lists_sessions_with_enough_aged_uncovered_chunks() {
        let s = store_with_session("s1");
        for t in 0..6 {
            add(&s, "s1", t, "x");
        }
        assert_eq!(coverage_debt(&s, 100).unwrap(), vec!["s1".to_string()]);
    }

    #[test]
    fn coverage_debt_clears_after_a_session_is_consolidated() {
        let s = store_with_session("s1");
        for t in 0..6 {
            add(&s, "s1", t, "x");
        }
        session_end(&s, "s1", 50).unwrap();
        assert!(coverage_debt(&s, 100).unwrap().is_empty());
    }
}
