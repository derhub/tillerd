# Consolidation, eviction, archive

Background jobs that keep the active database small while preserving everything.
CLI-triggered today; automatic scheduling lands when memorya adopts the daemon.

## Consolidation ladder

Aggregation only — no language model.

```
RAW CHUNKS
    │  consolidate session   (CLI; later: Stop hook)
    ▼
SESSION DIGEST     scope='session'
    │  consolidate daily     (CLI; later: nightly)
    ▼
DAILY DIGEST       scope='daily'
    │  consolidate weekly    (CLI; later: weekly)
    ▼
WEEKLY DIGEST      scope='weekly'
    │  consolidate monthly   (CLI; later: monthly)
    ▼
MONTHLY DIGEST     scope='monthly'  (never evicted)
```

Each step concatenates the level below → INSERT digest → embed async → mark the
sources `covered_by`. (The deferred curation change adds the one daily LLM step
that also produces `MEMORY.md` and extracts facts.)

Coverage-debt detection triggers session summarization:

```sql
SELECT session_id, COUNT(*) AS uncovered
FROM chunks
WHERE covered_by_digest IS NULL AND ts < (unixepoch() - 86400)
GROUP BY session_id HAVING uncovered > 5
```

## Lazy eviction

Scores active chunks and moves high scorers to the archive in atomic batches.
Nothing is deleted. Source: `src/coverage.rs`.

```rust
fn eviction_score(c: &ChunkStat, now: i64) -> f32 {
    let age      = days_since(c.ts, now);
    let recency  = days_since(c.last_accessed.unwrap_or(c.ts), now);
    let freq     = c.access_count as f32;
    let coverage = match (c.covered_by_digest, c.covered_by_fact) {
        (true, true)   => 2.0,
        (true, false)  => 1.5,
        (false, true)  => 1.2,
        (false, false) => 0.5,
    };
    ((age * 0.3) + (recency * 0.3)) * coverage - (freq * 10.0)
}
```

| Coverage                | Access     | Action         |
| ----------------------- | ---------- | -------------- |
| covered (digest + fact) | unaccessed | archive fast   |
| covered                 | accessed   | archive normal |
| uncovered               | unaccessed | archive slow   |
| uncovered               | accessed   | keep           |

Doc chunks are not evicted (regenerated each session). Facts, entities, relations,
and `scope='monthly'` digests are never evicted.

Move (atomic, batch 500): `ATTACH` the year shard, `INSERT … SELECT` then `DELETE`
in one transaction, `DETACH`.

## Archive sharding

```
~/.athing/
  archive-2025.db    sealed, read-only
  archive-2026.db    current write target
  archive-index.json [{ "file": "archive-2026.db", "year": 2026, "sealed": false }]
```

Rotation at the year boundary (or a size trigger). Opening a newer shard seals
older ones. `ArchiveRouter` lazy-opens sealed shards read-only and searches them
newest-first on a confirmed archive fallback.
