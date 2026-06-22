# store-architecture Specification

## Purpose
TBD - created by archiving change restructure-store-layers. Update Purpose after archive.
## Requirements
### Requirement: Cross-aggregate session creation goes through a coordinator, not a store

A standalone coordinator owned by the composition root SHALL create a session from a launch template
-- resolving `launch_template` -> spec, then materializing the session through the `Sessions` store.
This spans two aggregates. The `Sessions` store SHALL NOT depend on `LaunchTemplates`, and the
coordination SHALL NOT be duplicated in host controllers.

#### Scenario: Templated session creation resolves through the coordinator

- **WHEN** a session is created with a `template_id`
- **THEN** the coordinator resolves the template to a spec and then creates the session through the
  `Sessions` store
- **AND** the `Sessions` store itself has no dependency on `LaunchTemplates`

### Requirement: Storage behavior is preserved across the restructure

The restructure SHALL NOT change observable storage behavior. Every storage scenario that held
before the restructure SHALL hold after it.

#### Scenario: The existing storage suite passes unchanged

- **WHEN** the pre-restructure storage scenario suite (workspace/project/session/surface CRUD,
  ordering, rename, archive, placement uniqueness, seeding) runs against the per-entity stores
- **THEN** every scenario passes with assertions unchanged (only adapted to the async signatures)

### Requirement: Domain data is stored in sqlite via per-entity async repositories

Each domain entity (workspace, project, session, surface, command, launch_template, notification) SHALL
be persisted as sqlite rows through a per-entity repository in `infra/` that holds a `SqlitePool` and
exposes typed async `create`, `get`, `list`, `update`, and `delete`. Persistence SHALL use `sqlx`
(async, compile-time-checked queries); there SHALL be no ORM, no dispatch enum, no `store/` wrapper, and
no in-memory backend. Each repository SHALL own its table, columns, and `Row -> Entity` mapping. Nesting
SHALL be a `parent_id` column resolved by `list`; rename, move, and archive SHALL be `UPDATE`s; there
SHALL be no slug derivation, unique-slug scan, directory move, or subtree reindex.

#### Scenario: A repository persists and reads a typed entity

- **WHEN** an entity is created and read back through its repository
- **THEN** the repository writes/reads its sqlite row and maps it to the typed entity, with no slug-tree
  directory and no `Backend` dispatch

#### Scenario: A rename is a plain update

- **WHEN** a domain entity is renamed
- **THEN** the command loads it, applies the entity rename rule, and persists it through the repository's
  `update`, with no directory move or slug scan

#### Scenario: Children are found by parent id

- **WHEN** the children of a parent entity are listed
- **THEN** the repository issues a `list` filtered by the `parent_id` column, not a directory walk

### Requirement: list supports pagination

Every repository `list` SHALL accept a `Page` (`All`, `Offset { limit, offset }`, or
`Cursor { after, limit }`) and return a `Listing<T> { items, next }`, where `next` is the cursor for the
following page (or none). `Page` and `Listing` SHALL live in `shared/`.

#### Scenario: A bounded page returns a continuation cursor

- **WHEN** `list` is called with `Cursor { after, limit }`
- **THEN** at most `limit` items are returned in a stable order, with `next` set when more remain and
  unset at the end

#### Scenario: Unbounded listing is explicit

- **WHEN** a caller wants every row
- **THEN** it passes `Page::All`

### Requirement: shared/ holds reusable building blocks, not a storage abstraction

`shared/` SHALL hold the reusable building blocks: `fs` (file read/write/list/delete utils), `kv` (a
`Kv` trait with `put(key, value, options)` and `get(key)` returning an optional value, async, with
`SqliteKv` and `MemoryKv` impls), `pagination`, `datetime`, `errors` (one error-registry enum), and the
CQS machinery (`Command`/`Query` traits and the `Bus`). There SHALL be no generic entity-agnostic
`Repository` trait.

The error registry SHALL be a single enum where each variant declares a stable, low-cardinality telemetry
code via an `#[error_code("...")]` attribute, generated into a `code()` method by an `ErrorCode` derive
(in a small core-owned proc-macro crate); a variant missing the attribute SHALL be a compile error.
`Display`, `#[from]`, and the source chain stay with `thiserror`. There SHALL be no `level`/`category` --
every error logs at `ERROR` (a returned `Err` is a real failure; expected absence is `Ok(None)`). Ids
SHALL NOT appear in a code (they belong in the message and on the span).

#### Scenario: Every error variant declares a stable code

- **WHEN** an error variant is added without `#[error_code]`
- **THEN** compilation fails until a code is declared, and the code is a stable low-cardinality string
  (no id), available via `code()`

#### Scenario: The key-value store round-trips by key

- **WHEN** `put(key, value, options)` is called and then `get(key)`
- **THEN** `get` returns the stored value as `Some`, returns `None` for an absent key, and honors any TTL
  option, for both `SqliteKv` and `MemoryKv`

#### Scenario: User-config is read and written via fs utils

- **WHEN** a settings/config/theme/keybinding/profile value is persisted
- **THEN** it is read/written through `shared::fs` utils, not a domain repository

### Requirement: Pinnable entities order pinned-first

Workspace, project, session, command, and template SHALL each carry a `pinned` flag toggled by `Pin*`/
`Unpin*` commands, and their `list` queries SHALL return pinned items first (then the existing order).

#### Scenario: A pinned item sorts ahead of unpinned

- **WHEN** an entity is pinned and its collection is listed
- **THEN** it appears before unpinned items in the result order

### Requirement: The config plane is settings + profile + theme + keybinding over fs

User-config SHALL be the settings, profile, theme, and keybinding domains, persisted as files through
`shared::fs` (not sqlite). Settings SHALL support scoped override (`ApplySetting`), clear
(`ResetSetting`), and effective resolution through the profile cascade (`ResolveSetting`/
`ResolveSettings`). `ReloadConfig` SHALL re-read all config from disk so external edits are picked up.

#### Scenario: An external config edit is picked up on reload

- **WHEN** a config file is edited on disk and `ReloadConfig` runs
- **THEN** the new values become effective without restarting

#### Scenario: The active profile drives the cascade

- **WHEN** `ActivateProfile` switches the active profile
- **THEN** `ResolveSetting`/`ResolveSettings` reflect the new profile's values in the cascade

#### Scenario: Theme and keybinding choices persist via fs

- **WHEN** `ActivateTheme` or `RebindKey` runs
- **THEN** the choice is written to a config file through `shared::fs` and `ListThemes`/`ListKeybindings`
  reflect it (a prebuilt theme cannot be discarded; `ResetKeybinding` reverts one binding to its default)

### Requirement: Domain entities are searched in sqlite, not in app code

Projects and sessions SHALL be fuzzy-searchable by name, with the match evaluated in the sqlite query
(not by loading every row into the app and filtering there). The repository SHALL expose a `search`
that returns matching rows in a stable, match-ranked order.

#### Scenario: A fuzzy search filters in the query

- **WHEN** `SearchProjects`/`SearchSessions` runs with a query string
- **THEN** the repository issues a sqlite query that returns only matching rows in a stable order, with no
  app-side scan of the full table

### Requirement: Notification records carry read and snooze state

A `NotificationRecord` SHALL persist a `read` flag and an optional `snooze_until` timestamp as columns.
Its repository SHALL support listing all records, listing only unread, and counting unread, and SHALL
support a retention cap that keeps only the most recent N records.

#### Scenario: Read and snooze state round-trip

- **WHEN** a notification's `read` flag or `snooze_until` is set and the record is read back
- **THEN** the persisted state is returned, unread listing excludes read records, and the unread count
  reflects only unread records

#### Scenario: Retention caps stored records

- **WHEN** the retention cap is applied with more than N records stored
- **THEN** only the most recent N records remain

### Requirement: Templates use two stores — project launch templates in sqlite, a portable library in files

A project-bound launch template (`LaunchTemplate { project_id, spec }`) SHALL persist as a sqlite row via
its repository. The portable template library (prebuilt + custom bundles selectable at session creation)
SHALL persist as files via `shared::fs`. A `Prebuilt` library template SHALL be immutable.

#### Scenario: A project launch template is a sqlite row

- **WHEN** a launch template is saved for a project and read back
- **THEN** it persists and reads as a sqlite row keyed by `project_id`

#### Scenario: A library template is a file bundle, prebuilt immutable

- **WHEN** a custom template is imported into the library
- **THEN** it is written as a file bundle through `shared::fs`, and a `Prebuilt` library template cannot be
  discarded or edited

