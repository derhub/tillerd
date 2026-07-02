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

All server-state reads SHALL resolve through the Query cache (`useQuery`/`useSuspenseQuery` over
the generated `query()` factories); components SHALL NOT fetch in effects and mirror results into
local state. Mutations SHALL refresh by invalidation. One exception is permitted: a
**high-frequency stream** (terminal output, live log tail) MAY feed the render through a local
bounded buffer, because patching the Query cache per record would re-render the world on every
frame; the durable part of such a feature (backlog, file lists) still resolves through the cache
and revalidates by invalidation.

#### Scenario: A read resolves through the query cache

- **WHEN** a component needs server state
- **THEN** it reads via a Query hook over the `query()` factory and renders from the cache

#### Scenario: A mutation refreshes by invalidation, not imperative refresh

- **WHEN** a mutation succeeds
- **THEN** affected queries refresh via declared invalidation keys, never a hand-called refresh

#### Scenario: A high-frequency stream renders from a bounded local buffer

- **WHEN** a component renders a high-frequency stream (PTY bytes, live log records)
- **THEN** records append to a bounded local buffer merged at render time
- **AND** the feature's durable reads (backlog, lists) still resolve through the Query cache and
  revalidate via invalidation
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

### Requirement: The per-entity TanStack surface is generated, not hand-written

The renderer's per-entity TanStack surface — query hooks (list/get, including infinite/paged), mutation hooks (create, rename, archive, delete, reorder, …), and event subscriptions — SHALL be consumed from the generated hook surface rather than hand-written one operation at a time. The generated hooks SHALL preserve the engine's existing semantics: query keys, declared `meta.invalidates`, optimistic snapshot/apply/rollback, and the global settle-invalidate handler.

#### Scenario: The renderer uses generated query and mutation hooks

- **WHEN** a component needs an entity's list or a create/rename/archive/delete/reorder mutation
- **THEN** it imports the generated hook
- **AND** no hand-written per-operation hook for that entity exists in the renderer

#### Scenario: Generated hooks keep optimistic and invalidation behavior

- **WHEN** a generated rename, reorder, or archive hook runs
- **THEN** the cache updates optimistically and rolls back on error
- **AND** on success only the declared `meta.invalidates` keys are invalidated through the global handler

#### Scenario: Hook argument and result types come from the generated bindings

- **WHEN** a generated hook is called
- **THEN** its argument and result types originate from the generated bindings
- **AND** a backend type change surfaces as a type error at the call site

### Requirement: Render-as-you-fetch route data with Suspense

Each data route SHALL kick off its reads in a `loader` via `context.queryClient.ensureQueryData(<the same queryOptions the component reads>)` WITHOUT awaiting (render-as-you-fetch); the route SHALL render immediately and route-critical reads SHALL be consumed with `useSuspenseQuery`, suspending into a pending fallback (a Suspense boundary) until ready. `await` in a loader SHALL be the last resort. Read query functions SHALL obtain the orchestrator client by awaiting a client-ready signal (resolving the client when the host is ready, or `null` on the web host) rather than gating with `enabled`, so a suspense read pends through both boot and fetch into one fallback. Composite views (e.g. the sidebar) SHALL be composed in the frontend from unscoped entity reads sliced client-side, NOT a server-side composed read; the loader kicks the reads off independently (non-awaited, not an awaited `Promise.all`). A server-side composed/optimized read is added only if profiling shows the client compose is too slow. Loader and component MUST use the same `queryOptions`. Realtime axes are unchanged: streams patch the cache (`setQueryData`/`invalidateQueries`) or feed their Store; the terminal byte stream stays outside Query.

#### Scenario: A route renders immediately and suspends for its data

- **WHEN** a data route is navigated to
- **THEN** its loader kicks off the fetch without awaiting and the component renders, `useSuspenseQuery` suspending into the route `pendingComponent` until the data resolves (no blocking route transition)

#### Scenario: Boot and fetch pend as one through the queryFn

- **WHEN** the app loads before the orchestrator client is ready
- **THEN** the read's queryFn awaits client readiness, the suspense read stays pending, and the pending fallback shows -- with no `enabled` flag and no separate boot gate

#### Scenario: A composite view is composed in the frontend, not by a backend read

- **WHEN** a route needs a tree/composite (e.g. the sidebar)
- **THEN** the frontend reads the unscoped entity lists via `useSuspenseQuery` and slices them client-side; switching scope is an in-memory filter, not a refetch
- **AND** no server-side composed read is added unless profiling shows the client compose is too slow

#### Scenario: The web host renders without a client

- **WHEN** the client-ready signal resolves `null` (web host)
- **THEN** the route queryFns short-circuit to the web surface and no suspense query hangs



### Requirement: Client-side cache persistence for fast cold start

The per-window Query cache and the active-workspace selection SHALL persist to browser storage
(`localStorage`; `IndexedDB` permitted if it outgrows the localStorage budget) and SHALL hydrate
before first paint, so on relaunch the shell renders last-known server-state immediately and
revalidates once the orchestrator client is ready (stale-while-revalidate). Persistence SHALL be
bounded by a `maxAge` and invalidated by a version `buster`, and SHALL persist only successful queries
(never pending/error). Native persistence plugins SHALL NOT be used (browser storage is portable to
the web host). Ephemeral data SHALL NOT persist: terminal output, orchestrator status, in-flight
mutations, large diff bodies, the live notification feed. `tillerd.db` remains the server
source-of-truth.

#### Scenario: Shell paints from cache before the orchestrator is ready

- **WHEN** the app relaunches with a non-expired persisted cache
- **THEN** the persisted Query cache + active-workspace hydrate synchronously and the shell renders
  the last-known sidebar/lists before the orchestrator process reaches ready
- **AND** once ready, the queryFns revalidate and the UI updates in place

#### Scenario: A version upgrade drops the cache

- **WHEN** the app version (buster) differs from the persisted cache's buster
- **THEN** the persisted cache is discarded rather than deserialized into a possibly-changed shape

#### Scenario: Ephemeral data is not persisted

- **WHEN** the cache is persisted
- **THEN** only successful list/layout/log-list queries + active-workspace are written; terminal
  output, orchestrator status, in-flight mutations, diff bodies, and the notification feed are not
