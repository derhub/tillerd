//! Fact operations: learn, supersede, soft-remove, valid.

use crate::entity;
use crate::store::Store;
use crate::Fact;
use rusqlite::params;

/// Record a fact about `entity_name`/`predicate`. Any currently-valid fact about
/// the same entity + predicate is superseded (its validity end set to `ts` and
/// its `superseded_by` pointed at the new fact). Returns the new fact id.
pub fn learn(
    store: &Store,
    entity_name: &str,
    predicate: &str,
    value: &str,
    ts: i64,
) -> anyhow::Result<String> {
    let conn = store.conn();
    let entity_id = entity::resolve_or_create(store, entity_name)?;
    let fact_id = entity::new_id(store)?;

    // Insert the new fact first so the supersede pointer references an existing
    // row, then supersede the prior currently-valid facts (excluding the new one).
    conn.execute(
        "INSERT INTO facts(id, entity_id, predicate, value, valid_from, valid_until)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
        params![fact_id, entity_id, predicate, value, ts],
    )?;
    conn.execute(
        "UPDATE facts SET valid_until = ?1, superseded_by = ?2
         WHERE entity_id = ?3 AND predicate = ?4 AND valid_until IS NULL AND id != ?2",
        params![ts, fact_id, entity_id, predicate],
    )?;
    Ok(fact_id)
}

/// Soft-remove the currently-valid fact about an entity + predicate.
pub fn remove(store: &Store, entity_name: &str, predicate: &str, ts: i64) -> anyhow::Result<()> {
    let conn = store.conn();
    let entity_id = entity::resolve_or_create(store, entity_name)?;
    conn.execute(
        "UPDATE facts SET valid_until = ?1
         WHERE entity_id = ?2 AND predicate = ?3 AND valid_until IS NULL",
        params![ts, entity_id, predicate],
    )?;
    Ok(())
}

/// All currently-valid facts for a named entity.
pub fn currently_valid_for_entity(store: &Store, name: &str) -> anyhow::Result<Vec<Fact>> {
    let conn = store.conn();
    let entity_id: Option<String> = conn
        .query_row(
            "SELECT id FROM entities WHERE name = ?1 LIMIT 1",
            params![name],
            |r| r.get(0),
        )
        .ok();
    let Some(entity_id) = entity_id else {
        return Ok(vec![]);
    };
    let mut stmt = conn.prepare(
        "SELECT id, entity_id, predicate, value, valid_from, valid_until
         FROM facts WHERE entity_id = ?1 AND valid_until IS NULL
         ORDER BY valid_from",
    )?;
    let rows = stmt.query_map(params![entity_id], |r| {
        Ok(Fact {
            id: r.get(0)?,
            entity_id: r.get(1)?,
            predicate: r.get(2)?,
            value: r.get(3)?,
            valid_from: r.get(4)?,
            valid_until: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learn_then_query_returns_current_fact() {
        let s = Store::open_in_memory().unwrap();
        learn(&s, "user", "prefers_pm", "bun", 100).unwrap();
        let facts = currently_valid_for_entity(&s, "user").unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].value, "bun");
        assert!(facts[0].valid_until.is_none());
    }

    #[test]
    fn superseding_a_fact_makes_only_the_new_one_current() {
        let s = Store::open_in_memory().unwrap();
        learn(&s, "user", "lives_in", "London", 100).unwrap();
        learn(&s, "user", "lives_in", "Copenhagen", 200).unwrap();

        let current = currently_valid_for_entity(&s, "user").unwrap();
        assert_eq!(
            current.iter().map(|f| f.value.as_str()).collect::<Vec<_>>(),
            vec!["Copenhagen"]
        );
    }

    #[test]
    fn superseding_a_fact_retains_the_old_one_with_bounded_validity() {
        let s = Store::open_in_memory().unwrap();
        learn(&s, "user", "lives_in", "London", 100).unwrap();
        let new_id = learn(&s, "user", "lives_in", "Copenhagen", 200).unwrap();

        let (until, superseded_by): (Option<i64>, Option<String>) = s
            .conn()
            .query_row(
                "SELECT valid_until, superseded_by FROM facts WHERE value = 'London'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((until, superseded_by), (Some(200), Some(new_id)));
    }

    #[test]
    fn remove_drops_from_current_but_retains_row() {
        let s = Store::open_in_memory().unwrap();
        learn(&s, "user", "uses", "vitest", 100).unwrap();
        remove(&s, "user", "uses", 150).unwrap();
        assert!(currently_valid_for_entity(&s, "user").unwrap().is_empty());
        let count: i64 = s
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM facts WHERE value = 'vitest'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "soft-remove must retain the row");
    }

    #[test]
    fn unknown_entity_returns_empty() {
        let s = Store::open_in_memory().unwrap();
        assert!(currently_valid_for_entity(&s, "nobody").unwrap().is_empty());
    }
}
