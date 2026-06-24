# 0039. TanStack client engine: Query as the sync axis, Store for client state

- Status: accepted (routing decision superseded by ADR-0040)
- Date: 2026-06-23

## Context

The `apps/ui` renderer ran on react-router framework-mode (`ssr: false`) whose routes were empty
navigation-target stubs; the real shell rendered from URL params and search params. Server-state
was hand-synced through component-local `refresh()` callbacks and `useRevalidator().revalidate()`
calls that revalidated nothing (no loaders). Window intent rode `?w=detached|project|workspace`,
parsed by a hand-rolled string parser. The framework-mode toolchain also dragged in SSR machinery
(`@react-router/node|serve`, `react-router.config.ts`) the SPA-only desktop never used.

The desktop host serves the static client bundle over a custom-scheme origin with no deep-route SPA
fallback, so windows load at root and dispatch on the search param. ADR-0034 (accepted, deferred)
anticipated that "the client (0.0.16) wires sync status through TanStack Query"; ADR-0036 froze the
IPC/wire/data-model and is additive-only on those seams. The orchestrator core stays untouched by
this change.

## Decision

Adopt the TanStack stack as the client engine for `apps/ui`; the orchestrator backend is unchanged.

- **TanStack Router** over a plain Vite SPA build replaces react-router framework-mode. A code-based
  root route validates the window-intent search-params (a passthrough record; the typed
  `WindowIntent` is derived in `RootShell`, so non-intent params like the logs `?service=` filter
  survive); load-at-root + dispatch and `pushState`/`popstate` client-nav are preserved. The
  react-router build toolchain is removed (clean cutover, pre-v1).
  **[SUPERSEDED by ADR-0040: this code-based single-root dispatch is replaced by file-based routing
  (`@tanstack/router-plugin`) with a desktop SPA fallback; `?w=` window intent still dispatches at the
  root layout.]**
- **TanStack Query is the single server-state sync axis.** Reads flow through a per-window
  `QueryClient`, defined as `queryOptions` factories rooted at `[entity]` (`["projects"]`,
  `["sessions"]`, `["workspaces"]`); the renderer renders from query pending/error status. No
  component fetches in an effect; HTTP reads (the session diff) are a query `queryFn`.
- **Mutations auto-invalidate, globally.** A single `MutationCache.onSuccess` reads
  `mutation.meta.invalidates` and invalidates exactly those keys -- there is no per-mutation
  `onSuccess`, and a mutation that declares no keys invalidates nothing (no blanket cache wipe).
  Latency-felt mutations (rename, reorder, archive) add optimistic `onMutate`/`onError`
  (snapshot + `setQueryData` + rollback) for instant UI; the global cache does the settle-invalidate.
- **TanStack Store** holds client UI state shared across components -- the active workspace
  selection, global settings, and the notification feed -- each a per-window store read via
  `useStore(store, selector)`, never a second copy of server data the Query cache owns. Settings and
  notifications keep a thin bootstrap provider that runs their hydrate/subscribe lifecycle into the
  store; consumers read selector-scoped slices instead of a React context value.
- **Cross-window coherence is a live invalidation broadcast over the Tauri event bus.** Each window
  has its own `QueryClient`; a mutation refreshes its own window instantly and broadcasts the same
  `meta.invalidates` keys on a `query:invalidate` event, so sibling windows invalidate live -- not
  only on refocus. `refetchOnWindowFocus` is disabled. BroadcastChannel is unfit here: Tauri windows
  are separate OS webview processes, so it does not reliably broker across them. The broadcast is a
  transport-only event -- no backend, IPC-command, wire, or data-model change. A self-DDoS guard
  coalesces and dedupes keys per ~80ms flush in both directions, so a burst of mutations costs at
  most one emit per window and one invalidation pass per receiver.

This builds the engine ADR-0034 named; it does not implement the deferred state-model contract
itself (view pointers, lifecycle FSM, guards, sync-status enum remain out of scope).

## Consequences

- One sync axis: cache status (pending/error/stale/refetch) replaces ad-hoc loading flags and
  imperative refresh; mutations settle through one global handler instead of scattered callbacks.
- The SPA build sheds unused SSR machinery; the desktop/web hosts share one Vite client output.
- New client dependencies (`@tanstack/react-router`, `@tanstack/react-query`,
  `@tanstack/react-store`); react-router and `@react-router/*` are dropped.
- The orchestrator core, IPC commands, wire protocol, and data model are unchanged -- coherence is a
  pure client concern (optimistic + auto-invalidate in-window, live invalidation broadcast across
  windows over the Tauri event bus). Live side-by-side multi-window sync works now; the broadcast is
  a thin transport, additive over the engine.
- The later state-model slice (deferred ADR-0034, to be re-based onto ADR-0036) can wire view
  pointers/guards through this same Query/Store engine without re-deciding the engine.
- Conventions are captured in `docs/tanstack-client-engine.md` and enforced by ast-grep rules
  (no `fetch` and no inline dynamic `import()` inside hooks/components).
