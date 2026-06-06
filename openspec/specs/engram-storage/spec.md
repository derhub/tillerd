# engram-storage

## Purpose

Defines the persistence layer for the memory layer: a single-writer store holding a temporal fact graph, entities and relations, session chunks and digests, out-of-band embeddings, and the lexical and vector retrieval indexes.

## Requirements

### Requirement: Single-writer storage

The memory layer SHALL expose exactly one component that may write to storage, and storage SHALL only be opened by a dedicated storage layer. No other component may write to or open the database directly.

#### Scenario: Write attempted through the sole writer

- **WHEN** any subsystem needs to persist a fact, chunk, digest, or embedding
- **THEN** it MUST route the write through the single writer component
- **AND** the write MUST be applied within the active database

#### Scenario: Direct database access rejected

- **WHEN** a component other than the storage layer attempts to open the database
- **THEN** the design MUST NOT provide an interface that permits it

### Requirement: Temporal fact graph

The store SHALL persist facts as a temporal knowledge graph in which each fact carries a validity interval, so that contradictory facts coexist with non-overlapping validity and history is never destroyed.

#### Scenario: New fact is currently valid

- **WHEN** a fact is recorded
- **THEN** it MUST be stored with a validity start and an open (unbounded) validity end
- **AND** it MUST be retrievable as a currently-valid fact

#### Scenario: A fact supersedes an earlier one

- **WHEN** a new fact replaces an existing fact about the same subject and predicate
- **THEN** the earlier fact's validity end MUST be set to the supersession time
- **AND** the earlier fact MUST remain stored for historical retrieval
- **AND** only the new fact MUST appear among currently-valid facts

#### Scenario: A fact is removed

- **WHEN** a fact is removed
- **THEN** its validity end MUST be set rather than the row deleted
- **AND** it MUST no longer appear among currently-valid facts

### Requirement: Entities and relations

The store SHALL persist entities (with a name, optional type, and aliases) and time-bounded relations between entities, so facts and relations can be grouped and traversed by subject.

#### Scenario: Facts grouped by entity

- **WHEN** all currently-valid facts for a named entity are requested
- **THEN** the store MUST return every currently-valid fact whose subject resolves to that entity

### Requirement: Session chunks and digests

The store SHALL persist captured session content as chunks and consolidated summaries as digests scoped to one of session, daily, weekly, or monthly, so raw content and progressively denser summaries are both queryable.

#### Scenario: Chunk persisted with kind and timestamp

- **WHEN** a captured event is stored
- **THEN** it MUST be recorded as a chunk carrying a kind, content, and timestamp

#### Scenario: Digest persisted with a scope

- **WHEN** a summary is produced by consolidation
- **THEN** it MUST be stored as a digest tagged with exactly one of the defined scopes

### Requirement: Out-of-band embeddings

The store SHALL keep embeddings in a separate table keyed by the embedded item and its kind, recording the embedding model identity and dimensionality, so the embedding model can change without a schema migration and the write path is never blocked by embedding computation.

#### Scenario: Embedding recorded after the item is stored

- **WHEN** an item requiring an embedding is persisted
- **THEN** the item MUST be committed first
- **AND** its embedding MUST be computed out-of-band and recorded afterward against the item and kind

#### Scenario: Embedding model identity is retained

- **WHEN** an embedding is stored
- **THEN** the stored row MUST record which model and dimensionality produced it

### Requirement: Lexical and vector retrieval indexes

The store SHALL maintain a full-text lexical index over chunk and fact content and SHALL support approximate-nearest-neighbor vector search over stored embeddings, so retrieval can combine lexical and semantic matching.

#### Scenario: Lexical index stays consistent with content

- **WHEN** chunk or fact content is inserted, updated, or deleted
- **THEN** the lexical index MUST be updated to reflect the change
