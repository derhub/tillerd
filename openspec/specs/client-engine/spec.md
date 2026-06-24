# Capability: client-engine

## Purpose

TBD — routing, state management, and cross-window coherence layer for the renderer, built on TanStack Router and TanStack Query.

## Requirements

### Requirement: Window intent dispatches at the root layout; content is file-based routes

The renderer SHALL use a file-based route tree (`@tanstack/router-plugin`) with a root layout and
path routes (`/`, `/session/$id`, `/logs`). The root layout SHALL derive the window intent from typed
search-params (`?w=detached|project|workspace` with associated ids) read through the router, and
dispatch on it: `detached` renders the detached surface; `project`/`workspace`/`main` render the shell
chrome (sidebar scoped to the intent, providers, command center, indicators) around an `<Outlet/>`.
Path-based content (panels vs the log viewer) SHALL be the child routes, with the session id read as a
typed path param on `/session/$id`. Invalid or absent `w` params SHALL resolve to the `main` intent.
Every window SHALL be openable at the root (`/?w=...`); deep path routes reached by client navigation
SHALL survive a reload via Tauri's built-in `index.html` asset fallback (which serves `index.html`
for unmatched non-asset paths), with no custom asset handler.

#### Scenario: Detached-panel window resolves its intent

- **WHEN** a window loads with `?w=detached&session=<sid>&placement=<p>`
- **THEN** the root layout exposes a typed intent of kind `detached` carrying `<sid>` and `<p>`
- **AND** the shell renders the detached surface for that session and placement

#### Scenario: Project window resolves its intent

- **WHEN** a window loads with `?w=project&project=<pid>` (optionally `&session=<sid>`)
- **THEN** the root layout exposes a typed intent of kind `project` carrying `<pid>` and the optional `<sid>`

#### Scenario: Missing or malformed intent falls back to main

- **WHEN** a window loads with no `w` param, or a `w` value whose required ids are absent
- **THEN** the root layout resolves the intent to kind `main`

#### Scenario: A deep path route survives a reload

- **WHEN** the app is at `/session/<id>` (reached by client navigation) and the window reloads
- **THEN** Tauri's built-in `index.html` fallback serves the app and the router matches `/session/$id`
  client-side, rendering that session's surface (not a 404)

### Requirement: Server-state cache is the single sync axis

Renderer reads of orchestrator data SHALL flow through the server-state query cache, keyed by the
orchestrator read command and its arguments. The cache SHALL expose pending, error, and stale
status for a read, and the renderer SHALL render from that status rather than from ad-hoc local
loading flags. A renderer mutation SHALL settle by invalidating the affected query keys so the
cache refetches; the renderer SHALL NOT call an imperative `refresh()` callback or a route
revalidator to re-read data.

#### Scenario: A read resolves through the query cache

- **WHEN** a component reads project/session data
- **THEN** it subscribes to a query keyed by the orchestrator read command and its arguments
- **AND** it observes the pending, error, and resolved states of that query

#### Scenario: A mutation refreshes by invalidation, not imperative refresh

- **WHEN** the renderer performs a create/rename/delete mutation
- **THEN** it invalidates the affected query keys and the cache refetches
- **AND** no imperative `refresh()` callback or route revalidation is used to re-read the list

### Requirement: A reactive store holds shared client UI state

Client UI state shared across components (e.g. the active workspace selection) SHALL live in a
reactive client store read through a selector, so a single update reflects in every subscriber
within the window. The store SHALL hold client state only and SHALL NOT become a second source of
truth for server data the query cache owns.

#### Scenario: A store update reflects in all subscribers

- **WHEN** the reactive store updates
- **THEN** every component subscribed (via its selector) re-renders with the new value

### Requirement: Mutations auto-invalidate; cross-window coherence is a live invalidation broadcast

A successful mutation SHALL invalidate exactly the query keys it declares (via `meta.invalidates`),
through one global handler -- not a per-mutation callback. A mutation that declares no keys SHALL
invalidate nothing; the renderer SHALL NOT blanket-invalidate the whole cache. A latency-felt
mutation (rename, reorder, archive) MAY apply an optimistic update (snapshot, apply, roll back on
error) so the UI changes instantly, with the global handler doing the settle-invalidate. Each window
owns its own query cache. On a successful mutation the global handler SHALL broadcast the declared
keys to sibling windows over the Tauri event bus, and each window SHALL invalidate its matching
queries on receipt -- live, not on refocus (`refetchOnWindowFocus` disabled). The broadcast SHALL
carry only invalidation keys and SHALL NOT require any backend, IPC-command, wire, or data-model
change. To prevent a self-inflicted refetch storm, the broadcast SHALL coalesce and dedupe keys over
a short flush window in both directions, so a burst of mutations costs at most one emit per window
and one invalidation pass per receiver; a window SHALL ignore its own broadcast.

#### Scenario: A successful mutation invalidates only its declared keys

- **WHEN** a mutation that declares `meta.invalidates` for an entity succeeds
- **THEN** the global handler invalidates exactly those keys and the cache refetches them
- **AND** a mutation that declares no keys invalidates nothing

#### Scenario: An optimistic mutation updates the UI before the server responds

- **WHEN** a rename, reorder, or archive mutation runs
- **THEN** the cache is updated optimistically so the change shows immediately
- **AND** on error the pre-mutation snapshot is restored

#### Scenario: A sibling window invalidates live when another window writes

- **WHEN** a mutation in one window succeeds and broadcasts its declared keys
- **THEN** every other window invalidates its matching queries and renders the updated data
- **AND** the window that wrote ignores its own broadcast (it already invalidated locally)

#### Scenario: A burst of mutations does not become a refetch storm

- **WHEN** many mutations succeed within one coalesce window
- **THEN** the declared keys are deduped and the broadcast emits at most once per window
- **AND** each receiving window runs at most one invalidation pass over the unique key set
