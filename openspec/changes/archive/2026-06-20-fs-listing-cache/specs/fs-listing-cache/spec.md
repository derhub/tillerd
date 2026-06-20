## ADDED Requirements

### Requirement: Read-through cache preserves read results

The fs backend SHALL serve entity reads and listings through an in-memory cache of parsed entity files, and the cached path SHALL return results identical to reading the files directly from disk.

#### Scenario: Repeated reads return identical results

- **WHEN** an entity is read or listed twice with no intervening change
- **THEN** both calls return the same data as a direct read of the on-disk files

### Requirement: mtime revalidation on read

The fs backend SHALL validate each cached entity file against its on-disk modification time before reuse: an unchanged mtime SHALL serve the cached struct, and a changed mtime SHALL trigger a re-read that replaces the cached entry.

#### Scenario: Externally changed file is re-read

- **WHEN** a cached entity file's content and modification time change on disk out of band
- **THEN** the next read returns the new on-disk content

#### Scenario: Unchanged file is served without behavioral difference

- **WHEN** a cached entity file's modification time is unchanged since the last read
- **THEN** the read returns data equal to the on-disk content

### Requirement: Write-driven cache invalidation

On the fs backend's own mutations the affected cache and id→path index entries SHALL be updated or invalidated so subsequent reads reflect the mutation.

#### Scenario: Read after a backend write reflects the write

- **WHEN** the backend creates, renames, or archives an entity
- **THEN** a subsequent read or listing reflects that mutation

### Requirement: Lazy id→path index construction

The fs backend SHALL build its id→path index on demand rather than by an eager full-tree scan at `open`, while seeding of the Default workspace and Unfiled project on an empty tree SHALL remain eager at `open`.

#### Scenario: Entity resolves by id after open without a prior listing

- **WHEN** the backend opens an existing tree and is asked for an entity by id before any listing call
- **THEN** the entity resolves correctly

#### Scenario: Empty tree is seeded eagerly at open

- **WHEN** the backend opens an empty tree
- **THEN** the Default workspace and Unfiled project exist immediately after `open`
