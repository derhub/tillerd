# Tasks — render-as-you-fetch route data (Suspense + non-awaited loaders)

## 1. Readiness in the queryFn (retire enabled-gating)

- [x] 1.1 `client.ts`: add `whenClientReady(): Promise<OrchestratorClient | null>` — resolves the client when the host is ready, `null` on web. `DesktopHostProvider` keeps the status subscription and resolves it.
- [x] 1.2 queryOptions factories (`projects`/`sessions`/`workspaces`/layout): queryFns `await whenClientReady()` then call the orchestrator; `null` → empty/web shape. Dropped the 6 readiness `enabled` gates (sidebar-data ×3, WorkspaceSwitcher, usePanelTree `!!client`). (logs file-list reads through its host `loadLogSource` adapter, not the orchestrator client — already host-gated, left as-is.)

## 2. Compose route-shaped queries (avoid Promise.all)

- [x] 2.1 Sidebar composes in the FRONTEND (no backend read): `useSidebarData` reads the unscoped project + session lists via `useSuspenseQuery` and slices them client-side; `WorkspaceSwitcher` reads the workspace list. Switching scope is an in-memory filter (instant, no refetch). A server-side composed/JOIN read is deferred — taken only if profiling shows the client compose is too slow. (Earlier `sidebar_tree` Rust+SDK read fully reverted.)
- [x] 2.2 `session.$id` layout is a single read; `logs` file list is a single source read.

## 3. Loaders kick off (no await) + components suspend

- [x] 3.1 `__root` loader `void ensureQueryData` for the workspace/project/session lists (independent kick-offs, NO await, not `Promise.all`); `session.$id` loader `void ensureQueryData(sessionQueries.layout(id))`.
- [x] 3.2 `sidebar-data` (composed tree) + `WorkspaceSwitcher` → `useSuspenseQuery`. **Scoped (design: useQuery for deferred/optional reads):** `usePanelTree` layout stays `useQuery` — optional when no session is selected (the `/` route has none), warmed by the `session.$id` loader. `DiffPanel` stays `useQuery` — it gates on session run-status and reads the HTTP REST diff API, not the orchestrator client (out of the readiness axis).
- [x] 3.3 Sidebar suspends into a local `<Suspense>` boundary with a skeleton fallback so the chrome stays visible while it pends (honours the progressive-boot rule over blanking the whole shell via a route `pendingComponent`). `PanelContent`'s boot skeleton stays host-status-driven (the e2e-proven panels boot UI). Errors via the existing `defaultErrorComponent`.

## 4. Verify

- [x] 4.1 Built `app/lib/test/suspense.tsx` (`renderWithSuspense`: QueryClientProvider + Suspense). Unit: queryFn pends until `whenClientReady` then returns data, empty on web; fallback → content render-as-you-fetch; `useSidebarData` client-side scope slicing (workspace/project/all) covered in `SessionSidebar.test`.
- [ ] 4.2 Full e2e green (boot/pending, nav, detach/re-attach, workspace/project scoping, logs `?service`, reload-deep-route) + Rust + ast-grep + `bun run verify`.
- [ ] 4.3 Docs `docs/tanstack-client-engine.md` + memory `client-engine.md`: render-as-you-fetch, non-awaited loaders, queryFn readiness (`withClient`), frontend composition, realtime = streams patch cache.

## 5. Lazy per-project sessions (instant-first; no whole-sidebar fetch)

Removes the pre-existing all-sessions over-fetch (10 projects x thousands of sessions). Backend groundwork done: `session_list` now takes additive `limit`/`offset` (offset paging).

- [x] 5.1 `sessionQueries.infinite(projectId)` via `infiniteQueryOptions` (offset paging, page 50, `getNextPageParam` = next offset while a full page returns). Unit-tested.
- [x] 5.2 `ProjectRow` gains expand/collapse; `ProjectSessions` mounts only when expanded and runs the infinite query + "Load more". Collapsed projects fetch NOTHING. `sessions` prop dropped.
- [x] 5.3 `useSidebarData` returns only projects (memoized for stable reference). All-sessions read + `groupByProject`-over-all removed.
- [x] 5.4 Palette "switch to session" → `SessionSearchDialog` issuing `session_search` (typed query), not a static full list.
- [x] 5.5 Unfiled bucket always renders (its sessions load lazily on expand); drag-reorder narrowed to the loaded pages (no cross-page reorder).
- [~] 5.6 Async-ban compliant (sg scan clean). Unit + `bun run verify` green (UI 246). **e2e specs NOT updated + desktop e2e NOT run** — happy-dom can't test expand/scroll/infinite (testing memory); the existing sidebar e2e specs assume inline sessions and need rewriting for expand-to-load before a desktop e2e pass. REMAINING.

  Fixes landed enabling 5.x: split the command-registry context (stable dispatch vs changing list) to stop a register/re-render loop; shimmed `getAnimations` in the happy-dom preload (base-ui ScrollArea).

## 6. Async-ban enforcement

- [x] 6.1 ast-grep `no-async-in-component` authored at `warning`.
- [x] 6.2 Swept `apps/ui/app/**` (minus `lib/data/**`): 42 violations across 15 files refactored to `mutate()`/`subscribe()`/lowercase helpers; rule flipped to `error`; sg scan clean.
