# client-engine (delta)

## MODIFIED Requirements

### Requirement: Client-side cache persistence for fast cold start

The per-window Query cache SHALL persist to browser storage (`localStorage`; `IndexedDB`
permitted if it outgrows the localStorage budget) and SHALL hydrate before first paint, so
on relaunch the shell renders last-known server-state immediately and revalidates once the
orchestrator client is ready (stale-while-revalidate). The active-workspace selection is no
longer persisted in browser storage: it is the `activeWorkspace` view pointer in the
orchestrator settings store (see `view-pointers`), read through the same persisted Query
cache so cold-start hydration still renders the last-known workspace scope before the
orchestrator is ready. Persistence SHALL be bounded by a `maxAge` and invalidated by a
version `buster`, and SHALL persist only successful queries (never pending/error). Native
persistence plugins SHALL NOT be used (browser storage is portable to the web host).
Ephemeral data SHALL NOT persist: terminal output, orchestrator status, in-flight
mutations, large diff bodies, the live notification feed. `tillerd.db` remains the server
source-of-truth.

#### Scenario: Shell paints from cache before the orchestrator is ready

- **WHEN** the app relaunches with a non-expired persisted cache
- **THEN** the persisted Query cache (including the cached view-pointer queries) hydrates
  synchronously and the shell renders the last-known sidebar/lists before the orchestrator
  process reaches ready
- **AND** once ready, the queryFns revalidate and the UI updates in place

#### Scenario: A version upgrade drops the cache

- **WHEN** the app version (buster) differs from the persisted cache's buster
- **THEN** the persisted cache is discarded rather than deserialized into a possibly-changed shape

#### Scenario: Ephemeral data is not persisted

- **WHEN** the cache is persisted
- **THEN** only successful list/layout/log-list/view-pointer queries are written; terminal
  output, orchestrator status, in-flight mutations, diff bodies, and the notification feed
  are not
