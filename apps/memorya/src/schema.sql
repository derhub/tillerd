PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);

-- Temporal knowledge graph (warm tier). Schema created here; population is
-- deferred to the curation change, so these tables start empty.
CREATE TABLE IF NOT EXISTS entities (
  id      TEXT PRIMARY KEY,
  name    TEXT NOT NULL,
  type    TEXT,
  aliases TEXT
);

CREATE TABLE IF NOT EXISTS facts (
  id            TEXT PRIMARY KEY,
  entity_id     TEXT REFERENCES entities(id),
  predicate     TEXT NOT NULL,
  value         TEXT NOT NULL,
  valid_from    INTEGER NOT NULL,
  valid_until   INTEGER,
  superseded_by TEXT REFERENCES facts(id),
  confidence    REAL DEFAULT 1.0,
  last_accessed INTEGER,
  access_count  INTEGER DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_facts_valid  ON facts(valid_until);
CREATE INDEX IF NOT EXISTS idx_facts_entity ON facts(entity_id, valid_until);

CREATE TABLE IF NOT EXISTS relations (
  subject_id  TEXT REFERENCES entities(id),
  predicate   TEXT NOT NULL,
  object_id   TEXT REFERENCES entities(id),
  valid_from  INTEGER NOT NULL,
  valid_until INTEGER
);

CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
  predicate, value,
  content='facts',
  content_rowid='rowid',
  tokenize='porter unicode61'
);
CREATE TRIGGER IF NOT EXISTS facts_ai AFTER INSERT ON facts BEGIN
  INSERT INTO facts_fts(rowid, predicate, value) VALUES (new.rowid, new.predicate, new.value);
END;
CREATE TRIGGER IF NOT EXISTS facts_ad AFTER DELETE ON facts BEGIN
  INSERT INTO facts_fts(facts_fts, rowid, predicate, value)
  VALUES ('delete', old.rowid, old.predicate, old.value);
END;
CREATE TRIGGER IF NOT EXISTS facts_au AFTER UPDATE ON facts BEGIN
  INSERT INTO facts_fts(facts_fts, rowid, predicate, value)
  VALUES ('delete', old.rowid, old.predicate, old.value);
  INSERT INTO facts_fts(rowid, predicate, value) VALUES (new.rowid, new.predicate, new.value);
END;

-- Embeddings, kept out-of-band and keyed by item + kind. The model + dim are
-- recorded so the embedding model can change without a schema migration.
CREATE TABLE IF NOT EXISTS embeddings (
  observation_id TEXT PRIMARY KEY,
  kind           TEXT NOT NULL,
  model          TEXT NOT NULL,
  dim            INTEGER NOT NULL,
  vec            BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_embeddings_kind ON embeddings(kind);

-- Session archive (cold tier).
CREATE TABLE IF NOT EXISTS sessions (
  id         TEXT PRIMARY KEY,
  ide        TEXT NOT NULL,
  cwd        TEXT,
  started_at INTEGER NOT NULL,
  ended_at   INTEGER,
  metadata   TEXT
);

CREATE TABLE IF NOT EXISTS chunks (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id        TEXT REFERENCES sessions(id) ON DELETE CASCADE,
  kind              TEXT NOT NULL DEFAULT 'chunk',
  content           TEXT NOT NULL,
  title             TEXT,
  file_path         TEXT,
  turn_index        INTEGER,
  ts                INTEGER NOT NULL,
  covered_by_digest INTEGER REFERENCES digests(id),
  covered_by_fact   TEXT REFERENCES facts(id),
  last_accessed     INTEGER,
  access_count      INTEGER DEFAULT 0,
  archived          INTEGER NOT NULL DEFAULT 0,
  content_hash      TEXT
);
CREATE INDEX IF NOT EXISTS idx_chunks_session ON chunks(session_id, ts);
CREATE INDEX IF NOT EXISTS idx_chunks_doc     ON chunks(file_path) WHERE kind = 'doc';
CREATE UNIQUE INDEX IF NOT EXISTS idx_chunks_dedup
  ON chunks(session_id, turn_index, kind) WHERE kind != 'doc';
-- Content-hash dedup covers exactly the gap the structural index misses: rows
-- with a NULL turn_index are mutually distinct under (session_id, turn_index,
-- kind) because SQLite treats NULLs as unequal in a unique index. Scoped to
-- NULL turn_index so distinct-turn chunks that share content are still kept.
CREATE UNIQUE INDEX IF NOT EXISTS idx_chunks_content_dedup
  ON chunks(session_id, content_hash)
  WHERE kind != 'doc' AND turn_index IS NULL AND content_hash IS NOT NULL;

CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
  content,
  content='chunks',
  content_rowid='id',
  tokenize='porter unicode61'
);
CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
  INSERT INTO chunks_fts(rowid, content) VALUES (new.id, new.content);
END;
CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
  INSERT INTO chunks_fts(chunks_fts, rowid, content) VALUES ('delete', old.id, old.content);
END;
CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
  INSERT INTO chunks_fts(chunks_fts, rowid, content) VALUES ('delete', old.id, old.content);
  INSERT INTO chunks_fts(rowid, content) VALUES (new.id, new.content);
END;

CREATE TABLE IF NOT EXISTS digests (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT REFERENCES sessions(id) ON DELETE CASCADE,
  scope      TEXT NOT NULL CHECK(scope IN ('session','daily','weekly','monthly')),
  content    TEXT NOT NULL,
  ts         INTEGER NOT NULL,
  covered_by INTEGER REFERENCES digests(id),
  last_accessed INTEGER,
  access_count  INTEGER DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_digests_scope ON digests(scope, ts);

-- Durable embedding work queue. Each committed chunk enqueues one request; the
-- background worker drains it. A row is removed on success, retried (kept
-- 'pending') on failure, and reset from 'processing' to 'pending' if a run is
-- abandoned mid-flight. The chunk reference cascades, so archived chunks drop
-- their queue rows.
CREATE TABLE IF NOT EXISTS capture_queue (
  chunk_id   INTEGER PRIMARY KEY REFERENCES chunks(id) ON DELETE CASCADE,
  status     TEXT NOT NULL DEFAULT 'pending',
  attempts   INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_capture_queue_pending
  ON capture_queue(created_at, chunk_id) WHERE status = 'pending';

INSERT OR IGNORE INTO schema_version(version) VALUES (1);
