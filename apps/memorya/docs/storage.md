# Storage

One embedded SQLite database (`~/.athing/memorya.db`), single writer. The warm
tier is the temporal knowledge graph; the cold tier is the session archive.
Embeddings are kept out-of-band and brute-force cosine is used for vector search
(no ANN index — sub-millisecond at memorya's scale).

## Temporal knowledge graph (warm tier)

Schema created here; populated by the deferred curation change.

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

CREATE TABLE schema_version (version INTEGER PRIMARY KEY);

CREATE TABLE entities (
  id      TEXT PRIMARY KEY,
  name    TEXT NOT NULL,
  type    TEXT,
  aliases TEXT  -- JSON array
);

CREATE TABLE facts (
  id            TEXT PRIMARY KEY,
  entity_id     TEXT REFERENCES entities(id),
  predicate     TEXT NOT NULL,
  value         TEXT NOT NULL,
  valid_from    INTEGER NOT NULL,
  valid_until   INTEGER,        -- NULL = currently valid
  superseded_by TEXT REFERENCES facts(id),
  confidence    REAL DEFAULT 1.0,
  last_accessed INTEGER,
  access_count  INTEGER DEFAULT 0
);
CREATE INDEX idx_facts_valid ON facts(valid_until);

CREATE TABLE relations (
  subject_id  TEXT REFERENCES entities(id),
  predicate   TEXT NOT NULL,
  object_id   TEXT REFERENCES entities(id),
  valid_from  INTEGER NOT NULL,
  valid_until INTEGER
);

CREATE VIRTUAL TABLE facts_fts USING fts5(
  predicate, value, content='facts', content_rowid='rowid',
  tokenize='porter unicode61'
);
-- insert/delete/update triggers keep facts_fts in sync

CREATE TABLE embeddings (
  observation_id TEXT PRIMARY KEY,   -- facts.id or chunks.id
  kind           TEXT NOT NULL,      -- 'fact' | 'chunk' | 'digest'
  model          TEXT NOT NULL,      -- recorded so vectors are never compared across models
  dim            INTEGER NOT NULL,
  vec            BLOB NOT NULL        -- little-endian f32
);
CREATE INDEX idx_embeddings_kind ON embeddings(kind);
```

Temporal supersede: a contradicting fact sets the prior fact's `valid_until` and
`superseded_by` rather than deleting it, so history is preserved. Only facts with
`valid_until IS NULL` are currently valid.

## Session archive (cold tier)

```sql
CREATE TABLE sessions (
  id TEXT PRIMARY KEY, ide TEXT NOT NULL, cwd TEXT,
  started_at INTEGER NOT NULL, ended_at INTEGER, metadata TEXT
);

CREATE TABLE chunks (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id        TEXT REFERENCES sessions(id) ON DELETE CASCADE,
  kind              TEXT NOT NULL DEFAULT 'chunk',  -- 'chunk' | 'tool' | 'doc'
  content           TEXT NOT NULL,
  title             TEXT,
  file_path         TEXT,        -- set for kind='doc'
  turn_index        INTEGER,
  ts                INTEGER NOT NULL,
  covered_by_digest INTEGER REFERENCES digests(id),
  covered_by_fact   TEXT REFERENCES facts(id),
  last_accessed     INTEGER,
  access_count      INTEGER DEFAULT 0,
  archived          INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_chunks_session ON chunks(session_id, ts);
CREATE INDEX idx_chunks_doc     ON chunks(file_path) WHERE kind = 'doc';
-- suppress duplicate hook fires; allow identical content across sessions
CREATE UNIQUE INDEX idx_chunks_dedup ON chunks(session_id, turn_index, kind)
  WHERE kind != 'doc';

CREATE VIRTUAL TABLE chunks_fts USING fts5(
  content, content='chunks', content_rowid='id', tokenize='porter unicode61'
);
-- insert/delete/update triggers keep chunks_fts in sync

CREATE TABLE digests (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT REFERENCES sessions(id) ON DELETE CASCADE,
  scope TEXT NOT NULL CHECK(scope IN ('session','daily','weekly','monthly')),
  content TEXT NOT NULL, ts INTEGER NOT NULL,
  covered_by INTEGER REFERENCES digests(id),
  last_accessed INTEGER, access_count INTEGER DEFAULT 0
);
CREATE INDEX idx_digests_scope ON digests(scope, ts);
```

## Invariants

- Only `Engram` may write; only the storage layer may open the database.
- Embeddings computed out-of-band — never block the write path.
- Embeddings record `model` + `dim`; on open, vectors from any other model are
  dropped so recall only compares same-model vectors.
