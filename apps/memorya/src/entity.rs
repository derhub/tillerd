//! Entity resolution: exact, aliases, fuzzy match, or create.

use crate::store::Store;
use rusqlite::params;

/// Names at or above this Jaro-Winkler similarity resolve to an existing entity.
const FUZZY_THRESHOLD: f64 = 0.92;

pub(crate) fn new_id(store: &Store) -> anyhow::Result<String> {
    let id: String = store
        .conn()
        .query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?;
    Ok(id)
}

/// Resolve `name` to an entity id, creating the entity if no match is found.
pub fn resolve_or_create(store: &Store, name: &str) -> anyhow::Result<String> {
    let conn = store.conn();

    // Exact name match.
    if let Ok(id) = conn.query_row(
        "SELECT id FROM entities WHERE name = ?1 LIMIT 1",
        params![name],
        |r| r.get::<_, String>(0),
    ) {
        return Ok(id);
    }

    // Alias or fuzzy-name match over existing entities.
    let mut best: Option<(String, f64)> = None;
    {
        let mut stmt = conn.prepare("SELECT id, name, aliases FROM entities")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            let (id, ename, aliases) = row?;
            if let Some(json) = &aliases {
                if let Ok(list) = serde_json::from_str::<Vec<String>>(json) {
                    if list.iter().any(|a| a.eq_ignore_ascii_case(name)) {
                        return Ok(id);
                    }
                }
            }
            let sim = strsim::jaro_winkler(&ename.to_lowercase(), &name.to_lowercase());
            if sim >= FUZZY_THRESHOLD && best.as_ref().map(|b| sim > b.1).unwrap_or(true) {
                best = Some((id, sim));
            }
        }
    }
    if let Some((id, _)) = best {
        return Ok(id);
    }

    let id = new_id(store)?;
    conn.execute(
        "INSERT INTO entities(id, name, type, aliases) VALUES (?1, ?2, NULL, NULL)",
        params![id, name],
    )?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_name_resolves_to_same_entity() {
        let s = Store::open_in_memory().unwrap();
        let a = resolve_or_create(&s, "table-api").unwrap();
        let b = resolve_or_create(&s, "table-api").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_names_are_distinct_entities() {
        let s = Store::open_in_memory().unwrap();
        let a = resolve_or_create(&s, "table-api").unwrap();
        let b = resolve_or_create(&s, "rendering-engine").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn alias_resolves_to_owner() {
        let s = Store::open_in_memory().unwrap();
        let id = resolve_or_create(&s, "table-api").unwrap();
        s.conn()
            .execute(
                "UPDATE entities SET aliases = ?1 WHERE id = ?2",
                params![r#"["the api"]"#, id],
            )
            .unwrap();
        let resolved = resolve_or_create(&s, "the api").unwrap();
        assert_eq!(resolved, id);
    }
}
