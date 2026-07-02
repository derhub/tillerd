## Context

Reads are `useQuery` in components, `enabled`-gated on client readiness; boot is gated in
`DesktopHostProvider`. The modern TanStack/React shape is **render-as-you-fetch**: route loaders kick
off the fetch (without awaiting), and components consume with `useSuspenseQuery`, suspending into a
route `pendingComponent` until data is ready. Suspense is the goto for this data-heavy app; it is
NOT a waterfall when the fetch is initiated in the loader (not in the component). No backend
data-model change; new read queries (if needed to shape data well) are additive on the frozen seams.

## Goals / Non-Goals

- **Goals:** render-as-you-fetch -- loaders `ensureQueryData` WITHOUT `await`; `useSuspenseQuery` for
  route-critical data; `pendingComponent`/`errorComponent` per route; readiness handled inside the
  queryFn so suspense pends through boot; queries shaped so no `Promise.all`/waterfall is needed.
- **Non-Goals:** awaiting in loaders (last resort only); `Promise.all` of many reads (smell -> shape
  one query instead); blocking the route transition on data.

## Decisions

- **Loaders kick off, never await (render-as-you-fetch).** Each route loader calls
  `context.queryClient.ensureQueryData(<shared queryOptions>)` and returns immediately (no `await`,
  `void` the promise). The router renders the component at once; `useSuspenseQuery` latches onto the
  in-flight request and suspends into `pendingComponent`. `await` in a loader is the LAST resort
  (only when a render truly must block on data) and is called out in review if used.
- **`useSuspenseQuery` is the default for route-critical reads.** `data` is always defined; the route
  `pendingComponent` is the single loading UI; the `defaultErrorComponent` (and per-route
  `errorComponent`) catch failures. `useQuery` is reserved for genuinely deferred/optional data
  (e.g. background/secondary panels), not the primary route data.
- **Readiness lives in the queryFn, not `enabled`/`beforeLoad`.** The queryFns await a client-ready
  signal (`whenClientReady()` in `client.ts`: resolves the orchestrator client when the host is
  ready, or `null` on the web host) before calling the orchestrator. A suspense query therefore pends
  through BOTH boot and fetch into one `pendingComponent` -- unifying "booting" and "loading" -- with
  no `enabled` flag and no `beforeLoad` await. `DesktopHostProvider` keeps the status subscription and
  resolves `whenClientReady`.
- **Compose in the frontend; reads stay unscoped, sliced client-side.** The sidebar reads the project
  and session lists once (unscoped) and the workspace list, each via `useSuspenseQuery`, then derives
  its scoped slice in memory (project window -> that project + its sessions; workspace scope -> that
  workspace's projects; main -> all). Scoping is a pure filter, so switching workspace/project is
  instant -- no refetch, no suspense flash. The root loader kicks the reads off independently
  (non-awaited; not an awaited `Promise.all`). We do NOT add a backend composed read: an N+1 server
  loop over the same per-entity reads is no faster than composing on the client. A real server-side
  optimization (single SQL JOIN) is deferred -- taken only if profiling shows the client compose is
  too slow (instant-first; cache/optimize only what is provably slow).
- **Web host:** `whenClientReady()` resolves `null`; route queryFns short-circuit to the web surface
  (the single `TerminalPane`); no suspense hang.

## Realtime coexistence (loaders own the snapshot, streams patch the cache)

Loader = initial snapshot; subscription = live updates into the same cache entry.

- **Cache-shaped realtime:** the loader kicks off the snapshot; the live signal calls
  `queryClient.setQueryData` (merge) or `invalidateQueries` (refetch). The **cross-window broadcast
  already is this** and stays. High `staleTime` (up to `Infinity`) for fully push-driven entries.
- **Notifications + service health:** subscription-driven into their Store / source; not loaders.
- **Terminal PTY output:** raw byte stream over a Tauri Channel into xterm -- outside Query entirely.
- **Orchestrator status:** the readiness signal the queryFns await; not a cache entry.

## Risks / Trade-offs

- **Data-layer readiness change.** Moving readiness into the queryFn (`whenClientReady`) replaces
  `enabled`-gating across the read hooks -- the central change. Guarded by the unit + e2e boot path.
- **Orchestrator read addition (decided: IN SCOPE).** No composed `sidebar_tree` read exists, so this
  change adds one additive read query (Rust orchestrator command + `command_contract.rs` entry + SDK
  method + a Rust scenario test) returning the workspace/project/session tree + initial-load hints.
  Read-only, additive on the frozen seams; existing per-entity reads untouched. Cross-repo blast
  radius (crates/orchestrator + packages/sdk + apps/ui) is the accepted trade for one suspending read.
- **Suspense test harness.** Components need a Suspense boundary + primed/awaitable QueryClient in
  tests; render helpers get a small update. Contract unchanged.
- **Desktop value is still modest** (local IPC), but the render-as-you-fetch + suspense structure is
  the idiomatic foundation and pays off on the web host + intent preload.
