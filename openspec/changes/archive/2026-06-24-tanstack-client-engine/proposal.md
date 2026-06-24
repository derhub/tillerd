## Why

The `apps/ui` renderer runs on react-router framework-mode, but its routes are empty
navigation-target stubs and its server-state is hand-synced through imperative `refresh()`
callbacks and vestigial `useRevalidator().revalidate()` calls (the routes carry no loaders).
ROADMAP 0.0.16 moves the client to the TanStack stack for ecosystem cohesion and typed
search-params that fit the `?w=<id>` window-intent model: TanStack Query makes the server-state
cache the single sync axis (pending/error/stale/refetch), TanStack Store backs lists that must
stay coherent across windows, and TanStack Router carries window intent as typed search-params
over a plain Vite SPA build — retiring the react-router framework-mode toolchain that brings SSR
machinery the SPA-only desktop never uses.

## What Changes

- **BREAKING (dev toolchain, pre-v1, no wire change)**: replace the react-router build toolchain
  (`react-router build/dev/serve`, `@react-router/dev|node|serve`, `react-router.config.ts`, the
  `reactRouter()` Vite plugin) with a plain Vite SPA build. Realign `apps/ui` `package.json`
  scripts, `vite.config.ts`, `tsconfig`, `serve.ts` (web SPA-fallback root), and the desktop
  `tauri.conf.json` `frontendDist`/`devUrl` to the new Vite output dir.
- **TanStack Router** replaces react-router routing (`routes.ts`, the `app/routes/_shell*.tsx`
  stubs, `root.tsx`, and the `react-router` imports in `AppShell`, `SessionSidebar`,
  `useSpawnSession`, `TerminalPane`). Typed search-params carry the existing window intent
  (`?w=detached|project|workspace`, today parsed by `parseWindowIntent`). The load-at-root +
  dispatch-on-search-param pattern is preserved (the desktop custom-scheme origin has no
  deep-route SPA fallback; client nav stays `history.pushState`+`popstate`).
  **[REVISED by ADR-0040: routing moves to a file-based tree (`@tanstack/router-plugin`) with path
  routes (`/`, `/session/$id`, `/logs`) under a root layout; `?w=` window intent still dispatches at
  the layout, and a desktop asset-protocol SPA fallback makes deep routes reload-safe. See tasks 9.x.]**
- **TanStack Query** becomes the server-state sync axis. The imperative component-local
  `refresh()` callbacks (`SessionSidebar`, `WorkspaceSwitcher`) and `useRevalidator().revalidate()`
  calls are removed; reads are Query-keyed by the orchestrator read commands the SDK exposes
  (`listProjects`/`listSessions`/...).
- **TanStack Store** provides the reactive client store backing lists that must stay coherent
  across windows.
- **Mutation coherence**: a successful mutation auto-invalidates exactly its declared keys via one
  global `MutationCache` handler (`meta.invalidates`) — no per-mutation `onSuccess`, no blanket
  wipe; latency-felt mutations (rename/reorder/archive) also apply an optimistic update for instant
  UI. Cross-window coherence is a live invalidation broadcast over the Tauri event bus (each window
  has its own cache; a mutation broadcasts its declared keys and siblings invalidate live, not on
  refocus — `refetchOnWindowFocus` disabled), with a coalesce/dedupe guard against refetch storms —
  no backend change. The orchestrator core, IPC signatures, data model, and dynamic ACL are all
  unchanged.

Out of scope (deferred): ROADMAP bullet 5 — view pointers, state-model guards, and the
workspace-activity read-model — which ride the deferred ADR-0034 state-model contract (itself
needing re-basing onto ADR-0036, since ADR-0033's `state.db` two-plane is superseded). No
`state.db`, no `contracts/state-model.json`, no lifecycle-FSM / sync-status enum / guards work
lands here. No change to IPC command signatures, the data model, or the dynamic ACL.

## Capabilities

### New Capabilities

- `client-engine`: the renderer's TanStack engine — typed search-param window intent via the
  router, server-state cache as the single sync axis via Query (no imperative refresh), mutations
  that auto-invalidate via a global `MutationCache` (`meta.invalidates`) with optional optimistic
  updates, a reactive Store for shared client UI state, and live cross-window coherence via an
  invalidation broadcast over the Tauri event bus (coalesced/deduped against refetch storms).

### Modified Capabilities

None. The coherence model is entirely client-side and part of the new `client-engine` capability.
`desktop-renderer-build` is not listed: its requirements — static SPA, no SSR runtime, single
renderer for both hosts, client routing under the web-view origin — are toolchain-agnostic and the
Vite SPA build satisfies them unchanged. The toolchain swap is an implementation detail captured in
design/tasks, not a spec-behavior change.

## Impact

- `apps/ui`: routing, data-fetching, and build configuration across `root.tsx`,
  `app/routes/**`, `AppShell`, `SessionSidebar`, `WorkspaceSwitcher`, `useSpawnSession`,
  `TerminalPane`, `app/lib/windows.ts`, `package.json`, `vite.config.ts`, `tsconfig.json`,
  `serve.ts`. New deps: `@tanstack/react-router`, `@tanstack/react-query`,
  `@tanstack/react-store`; removed: `react-router`, `@react-router/*`.
- `apps/desktop`: `tauri.conf.json` build wiring updated to the Vite output dir. The
  `crates/orchestrator` core and the transport command surface are unchanged (no backend change for
  coherence).
- No change to the IPC command surface, wire protocol, data model, or dynamic ACL (frozen at
  0.0.6).
