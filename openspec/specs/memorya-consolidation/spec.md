# memorya-consolidation

## Purpose

Defines how the memory layer condenses and ages content: a model-free consolidation ladder from chunks to monthly digests, coverage tracking, and lazy eviction of covered content to a year-sharded archive.

## Requirements

### Requirement: Consolidation ladder

The memory layer SHALL consolidate content up a ladder — session digests from chunks at session end, daily digests from session digests, weekly digests from daily digests, and monthly digests from weekly digests — without invoking a language model at any ladder step.

#### Scenario: Session digest produced at session end

- **WHEN** a session ends
- **THEN** its chunks MUST be aggregated into a session-scoped digest

#### Scenario: Higher-level digests aggregate lower ones

- **WHEN** a daily, weekly, or monthly consolidation runs
- **THEN** it MUST aggregate the digests of the level below into a digest of its own scope

#### Scenario: No model call in the ladder

- **WHEN** any ladder step runs
- **THEN** it MUST complete using aggregation and embedding only, with no language-model call

### Requirement: Coverage tracking

When content is summarized into a higher level or distilled into a fact, the memory layer SHALL mark the source content as covered, so eviction can favor content that already exists in denser form.

#### Scenario: Chunk marked covered by its digest

- **WHEN** a chunk is included in a digest
- **THEN** that chunk MUST be marked as covered by the digest

#### Scenario: Coverage debt triggers summarization

- **WHEN** a session has accumulated uncovered chunks older than a threshold
- **THEN** it MUST become eligible for summarization

### Requirement: Lazy eviction to archive

The memory layer SHALL provide an eviction operation that scores active chunks and moves high-scoring chunks to an archive database in atomic batches, never permanently deleting them. Eviction score MUST increase with age and time-since-access, scale with coverage, and decrease with access frequency. The operation MUST be invocable on demand; automatic scheduling is out of scope for this change.

#### Scenario: Covered, unaccessed content evicted first

- **WHEN** eviction runs
- **THEN** content that is both covered and unaccessed MUST be evicted before content that is uncovered or frequently accessed

#### Scenario: Frequently accessed content retained

- **WHEN** a chunk has a high access count
- **THEN** it MUST be retained in the active database

#### Scenario: Eviction is a move, not a delete

- **WHEN** a chunk is evicted
- **THEN** it MUST be inserted into the archive and removed from the active database within a single atomic operation

#### Scenario: Core memory never evicted

- **WHEN** eviction runs
- **THEN** monthly digests, facts, entities, and relations MUST NOT be evicted

### Requirement: Year-sharded archive

The archive SHALL be partitioned into year shards recorded in a shard registry, sealed shards SHALL be opened read-only, and a new shard SHALL be started at the year boundary or when the current shard exceeds a size limit.

#### Scenario: Shard rotation at year boundary

- **WHEN** the year changes or the current shard exceeds its size limit
- **THEN** the current shard MUST be sealed and a new shard started

#### Scenario: Sealed shard is read-only

- **WHEN** a sealed shard is opened for an archive search
- **THEN** it MUST be opened read-only

#### Scenario: Only needed shards opened

- **WHEN** an archive search targets a date range
- **THEN** only the shards overlapping that range MUST be opened
