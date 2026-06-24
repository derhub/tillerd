## Why

The renderer reads server state with `useQuery` in components, each gated on client readiness via
`enabled`, and boot is gated in React (`DesktopHostProvider`). That works but it is not the TanStack
way: the router should own data loading (loaders + `ensureQueryData`), Query is the cache, components
read guaranteed data via `useSuspenseQuery`, and pending/error/boot are route concerns. Embracing
this makes the renderer idiomatic and unlocks prefetch-on-navigation, which the web-server SPA host
(real network latency) genuinely benefits from.

## What Changes (render-as-you-fetch: loaders kick off, Suspense consumes)

- **Route loaders kick off fetches WITHOUT `await`** (`context.queryClient.ensureQueryData(...)`,
  promise voided). The router renders immediately; `useSuspenseQuery` latches onto the in-flight
  request and suspends into the route `pendingComponent`. This is render-as-you-fetch -- not a
  waterfall (the fetch starts in the loader, before the component). `await` in a loader is the LAST
  resort.
- **`useSuspenseQuery` is the default for route-critical data.** Suspense is the goto for this
  data-heavy app; `data` is always defined; the route `pendingComponent` is the single loading UI and
  `errorComponent` catches failures. `useQuery` stays only for genuinely deferred/optional reads.
- **Readiness moves into the queryFn, not `enabled`.** queryFns await `whenClientReady()` (client when
  the host is ready, `null` on web) before hitting the orchestrator, so a suspense query pends through
  BOTH boot and fetch into one `pendingComponent` -- retiring `enabled`-gating and the separate React
  boot skeleton. `DesktopHostProvider` keeps the status subscription and resolves the signal.
- **Compose in the frontend; keep reads unscoped + slice client-side.** The sidebar reads the
  project and session lists once (unscoped) via `useSuspenseQuery` and derives its scoped slice in
  memory (project window -> that project's sessions; workspace scope -> that workspace's projects).
  Scoping is a pure filter, so switching workspace/project is instant -- no refetch, no suspense
  flash. The root loader kicks these reads off (independent, non-awaited -- not an awaited
  `Promise.all`). A backend composed/optimized read is deferred: only worth a real single-query SQL
  JOIN if profiling shows the frontend compose is too slow -- an N+1 server loop is no win over the
  same reads from the client.
- `defaultPreloadStaleTime: 0` (already set) keeps Query the single cache owner.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `client-engine`: the data-flow requirement becomes render-as-you-fetch -- route loaders kick off
  `ensureQueryData` without awaiting; route-critical reads use `useSuspenseQuery` into a Suspense
  boundary; readiness lives in the queryFn (`whenClientReady`) rather than `enabled`; route data is
  composed in the frontend from unscoped reads sliced client-side (no blanket caching, no backend
  read). Query-as-cache, mutation auto-invalidate, cross-window broadcast, and Store usage are
  unchanged.

## Impact

- `apps/ui` ONLY (no backend change): `app/routes/*` (non-awaited `loader` kicking off
  `ensureQueryData`); `app/lib/data/client.ts` (`whenClientReady()` + queryFns await it instead of
  `enabled`); read hooks `sidebar-data` + `WorkspaceSwitcher` → `useSuspenseQuery` over the unscoped
  project/session/workspace lists, sliced client-side; `usePanelTree` layout + `DiffPanel` stay
  `useQuery` (deferred/optional); `DesktopHostProvider` resolves readiness; the sidebar suspends into
  a local `<Suspense>` boundary (chrome stays visible).
- **No backend read.** The orchestrator is untouched: the sidebar composes the existing per-entity
  reads in the frontend. A server-side composed/optimized read (real SQL JOIN) is a deferred option,
  taken only if profiling shows the client compose is too slow.
- Verification: UI unit (suspense harness) + full e2e (nav, detach, scoping, logs filter, reload) +
  the boot/pending path + 55 Rust + ast-grep + `bun run verify`.
- Builds on ADR-0039/0040. Archive `tanstack-client-engine` first so `client-engine` is canonical.
