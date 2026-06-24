## Context

`apps/ui` runs on react-router framework-mode (`ssr: false`). The route files
(`app/routes/_shell*.tsx`) are empty navigation-target stubs returning `null`; `AppShell` renders
everything from `useParams`/`useSearchParams`. Server-state is hand-synced: component-local
`refresh()` callbacks in `SessionSidebar`/`WorkspaceSwitcher` plus `useRevalidator().revalidate()`
calls that revalidate nothing (no loaders). Window intent rides `?w=detached|project|workspace`,
parsed by `parseWindowIntent` (`app/lib/windows.ts`). The desktop host serves the static client
bundle over a custom-scheme origin with no deep-route SPA fallback, so every window loads at root
and dispatches on the search param; client nav is `history.pushState`+`popstate`.

The backend is the runtime-agnostic Rust `orchestrator` crate (sqlx/sqlite `tillerd.db`) embedded
in-process by the Tauri desktop host. Outbound events follow ADR-0037 (in force): a domain exposes
a borrowed-event sink trait; the host implements the sink and forwards to webviews via
`app.emit(<name>, payload)` (e.g. `bridge.rs` `DAEMON_LOST_EVENT`, the notification channel). The
renderer subscribes via `@tauri-apps/api/event` `listen()` or the SDK `client.subscribe()`.

In-force ADRs that constrain this design: ADR-0037 (event dispatch), ADR-0036 (de-abstracted
sqlite storage; the IPC/wire/data-model freeze), ADR-0025 (runtime paths), ADR-0023 (data model).
ADR-0034 (state-model contract) is accepted-but-deferred and anticipates "the client (0.0.16)
wires sync status through TanStack Query"; this change builds the engine it named, not the
contract itself. ADR-0033 (two-plane `state.db`) is superseded by ADR-0036.

## Goals / Non-Goals

**Goals:**

- Replace react-router routing with TanStack Router; carry window intent as validated typed
  search-params; preserve load-at-root + dispatch.
- Make TanStack Query the single server-state sync axis (pending/error/stale/refetch); remove
  imperative `refresh()` and route revalidation.
- Mutations auto-invalidate via a global `MutationCache` (`meta.invalidates`), with optional
  optimistic updates; cross-window coherence is a live invalidation broadcast over the Tauri event
  bus (`refetchOnWindowFocus` disabled). No backend change.
- Use TanStack Store for shared client UI state (active workspace selection).
- Swap the build toolchain to a plain Vite SPA build and realign desktop/web wiring.

**Non-Goals:**

- ROADMAP bullet 5: view pointers, state-model guards, workspace-activity read-model (ride the
  deferred ADR-0034 contract; need re-basing onto ADR-0036). No `state.db`, no
  `contracts/state-model.json`, no lifecycle-FSM / sync-status enum / guards.
- Any change to IPC command signatures, the wire framing, the data model, or the dynamic ACL.
- Restore-after-quit / persisted view state; SSR or a web-server route rewrite.

## Decisions

- **Router: code-based tree, single root, validated search.** *(SUPERSEDED by ADR-0040 -- see
  "Routing revised" below.)* The routing surface is trivial (root + dispatch on `?w=`). Use a
  code-based `@tanstack/react-router` tree with one root route whose `validateSearch` parses the
  window-intent params into the typed `WindowIntent` union (the same shape `parseWindowIntent`
  produces today), using the project's validation library. No file-based route plugin (it adds
  codegen the trivial tree does not need). `_shell*.tsx` stubs, `routes.ts`, and the react-router
  imports in `AppShell`/`SessionSidebar`/`useSpawnSession`/`TerminalPane` are replaced by router
  hooks (`useSearch`, `useNavigate`, `useParams`) and a browser history that keeps the
  `pushState`+`popstate` nav the e2e harness drives.

- **Routing revised: file-based tree + desktop SPA fallback (ADR-0040).** The code-based single root
  is replaced by file-based routing via `@tanstack/router-plugin`
  (`tanstackRouter({ target: 'react', autoCodeSplitting: true })` before `react()` in
  `vite.config.ts`; `@tanstack/react-router-devtools` mounts dev-only). Routes under `app/routes/`:
  `__root.tsx` (RootLayout), `index.tsx` (`/`), `session.$id.tsx` (`/session/$id`, typed
  `useParams`), `logs.tsx` (`/logs`); the plugin generates `routeTree.gen.ts`, which `router.tsx`
  imports. **Chrome vs content split:** RootLayout owns chrome -- `DesktopHostProvider`,
  Settings/Notifications/CommandRegistry providers, `SessionContext` (spans sidebar chrome AND
  terminal content, so it stays at the root), the sidebar scoped by `?w=` window intent, the command
  center, the bottom indicators, and `<Outlet/>`. `?w=` dispatch stays in RootLayout: `detached`
  renders `DetachedWindow`; otherwise the chrome wraps the Outlet. Per-route content components own
  their state: the panels route owns `usePanelTree`/`useDetachedPanels`/the panel commands (so panel
  commands register only where panels exist, while view-logs/settings commands register from
  RootLayout), and the logs route renders the log viewer reading `?service`. `sessionId` derives at
  the root from `useParams({ strict: false }).id ?? intentSessionId`, provided via `SessionContext`;
  the `useShellLocation` pathname regex is removed. **Reload safety:** Tauri v2 already serves
  `index.html` for unmatched non-asset paths (built-in, default-on -- ADR-0039's "no fallback"
  premise was wrong), so a reload at `/session/abc` loads the app and the router matches client-side.
  No custom `src-tauri` handler is added; a reload-at-deep-route e2e proves it. `routeTree.gen.ts` is
  committed but ignored by oxfmt/oxlint/ast-grep and added to the tsconfig include.

- **Query as the sync axis.** A single `QueryClient` per window. Read commands the SDK exposes
  (`listProjects`/`listSessions`/...) become queries keyed `[command, ...args]`. `SessionSidebar`
  and `WorkspaceSwitcher` read via `useQuery` (data-layer `queryOptions` factories rooted at
  `[entity]`) and drop their `refresh()` callbacks; the vestigial `useRevalidator().revalidate()`
  calls are deleted.

- **Mutation invalidation is global, not per call site.** One `MutationCache.onSuccess` reads
  `mutation.meta.invalidates` and invalidates exactly those keys — no per-mutation `onSuccess`, and
  a mutation that declares no keys invalidates nothing (never a blanket cache wipe). Mutation hooks
  are one-liners (`mutationFn` + `meta.invalidates: [entityQueries.all()]`). Latency-felt mutations
  (rename/reorder/archive) add optimistic `onMutate`/`onError` (snapshot + `setQueryData` +
  rollback) for instant UI; the global handler does the settle-invalidate.

- **Store scope.** TanStack Store holds client UI state shared across components (the active
  workspace selection), read through a selector, never a second copy of server data the Query cache
  owns. Windows are separate webviews with separate JS contexts, so the store is per-window.

- **Cross-window coherence = live invalidation broadcast over the Tauri event bus.** Each window has
  its own `QueryClient`; a mutation refreshes its own window instantly (optimistic + auto-invalidate)
  and broadcasts its `meta.invalidates` keys on a `query:invalidate` event, so sibling windows
  invalidate live -- not only on refocus (`refetchOnWindowFocus` disabled). BroadcastChannel is unfit
  because Tauri windows are separate OS webview processes; the native event bus is the transport.
  There is no backend change — the orchestrator core, IPC commands, and wire are untouched; the
  broadcast carries only invalidation keys. A self-DDoS guard coalesces and dedupes keys per ~80ms
  flush in both directions: a burst of mutations costs at most one emit per window and one
  invalidation pass per receiver, on top of TanStack's in-flight refetch dedup.

- **Build toolchain: plain Vite SPA.** Remove `@react-router/dev|node|serve`,
  `react-router.config.ts`, and the `reactRouter()` Vite plugin; `vite.config.ts` keeps
  `@tailwindcss/vite` and adds the TanStack Router/Query devtools only as needed. `package.json`
  scripts become `vite build` / `vite dev` / `vite preview` (or the existing `serve.ts` for the
  web SPA-fallback host) and `tsc` for `check-types` (drop `react-router typegen`). The Vite
  client output dir (default `dist/`) replaces `build/client`; `serve.ts` root and the desktop
  `tauri.conf.json` `frontendDist`/`devUrl` are realigned to it. `root.tsx` loses the react-router
  `Links/Meta/Scripts/Outlet/ScrollRestoration` document shell — the SPA `index.html` owns the
  document and mounts the router.

- **Clean cutover.** Pre-v1, no back-compat shims; react-router is removed in the same change.

## Risks / Trade-offs

- **Broad diff.** Routing + data-fetching + build config + a component reorganization touch the
  whole of `apps/ui`. Mitigated by Query/Store being additive to observable behavior (the component
  specs — "sessions load on mount", navigation, empty states — are preserved, e2e DOM untouched).
- **Live cross-window coherence.** A side-by-side window invalidates the moment another window
  writes, via the `query:invalidate` broadcast — no refocus needed. The coalesce/dedupe guard bounds
  a mutation burst to one emit per window and one invalidation pass per receiver.
- **e2e nav under custom scheme.** TanStack Router must keep working with the harness's
  `pushState`+`popstate` client-nav and load-at-root pattern; verified by the existing desktop e2e
  window-intent + navigation specs.
- **Search-param validation.** `validateSearch` is a passthrough record (the typed `WindowIntent`
  is derived in `RootShell`) so non-intent params like `?service=` survive.

## Open Questions

- None blocking. ADR-0034 remains deferred and is not revisited here.
