## 1. Build toolchain + dependencies

- [x] 1.1 Add `@tanstack/react-router`, `@tanstack/react-query`, `@tanstack/react-store` to `apps/ui`; remove `react-router`, `@react-router/dev|node|serve`. Install in the worktree.
- [x] 1.2 Replace the build toolchain: delete `react-router.config.ts`; update `vite.config.ts` (drop `reactRouter()`, keep tailwind; add an `index.html` SPA entry that mounts the router); set `package.json` scripts to `vite build`/`vite dev`/`vite preview` and `check-types` to `tsc` (drop `react-router typegen`); update `tsconfig.json` (drop `.react-router/types`, `rootDirs`).
- [x] 1.3 Realign hosts to the Vite client output dir: `serve.ts` SPA-fallback root and desktop `tauri.conf.json` `frontendDist`/`devUrl`. Confirm `bun run build` (UI) emits the dir both hosts point at.

## 2. TanStack Router — typed window intent (spec: client-engine)

- [x] 2.1 Write unit tests (red) for window-intent search validation: `detached`, `project`, and missing/malformed → `main` (port `windows.test.ts` intent cases to the router `validateSearch`).
- [x] 2.2 Build the code-based router: one root route whose `validateSearch` parses `?w=...` into the typed `WindowIntent` union (reuse the existing union shape); browser history preserving `pushState`+`popstate`. Mount in `index.html`; delete `routes.ts`, `app/routes/_shell*.tsx`, and the react-router document shell in `root.tsx`.
- [x] 2.3 Replace react-router hooks in `AppShell`/`SessionSidebar`/`useSpawnSession`/`TerminalPane` with router hooks (`useSearch`/`useNavigate`/`useParams`); the shell dispatches on the typed intent. Green 2.1.

## 3. TanStack Query — server-state sync axis (spec: client-engine)

- [x] 3.1 Write unit/component tests (red): a read resolves through the query cache (pending/error/resolved); a mutation re-reads by `invalidateQueries`, with no imperative `refresh()` or revalidation.
- [x] 3.2 Add a per-window `QueryClient` provider. Convert `SessionSidebar`/`WorkspaceSwitcher` reads to `useQuery` keyed `[command, ...args]`; convert create/rename/delete to `useMutation` settling via `invalidateQueries`. Remove the `refresh()` callbacks and `useRevalidator().revalidate()` calls. Green 3.1.

## 4. TanStack Store — coherent lists (spec: client-engine)

- [x] 4.1 Write a unit test (red): a store update reflects in every subscriber.
- [x] 4.2 Introduce the reactive store for derived client list/selection state lifted out of component `useState`; subscribers read via the store hook. Green 4.1. Keep server data in the Query cache (store holds derived state only).

## 5. Mutation auto-invalidation (spec: client-engine)

- [x] 5.1 Configure one global `MutationCache.onSuccess` on the per-window `QueryClient` that reads `mutation.meta.invalidates` and invalidates exactly those keys — no per-mutation `onSuccess`, and no blanket invalidate when a mutation declares nothing. Test the handler against a real `makeQueryClient()`.
- [x] 5.2 Make every write mutation hook a one-liner declaring `meta: { invalidates: [<entity>Queries.all()] }` (project/session/workspace; cross-entity writes like delete-project-cascades-sessions list both roots). No `refresh()`, no `useRevalidator`. (No backend change — the orchestrator core and transport are untouched.)

## 6. Optimistic updates + cross-window coherence (spec: client-engine)

- [x] 6.1 Add optimistic `onMutate`/`onError` (cancel + snapshot via `getQueryData` + apply via `setQueryData` + rollback) to the latency-felt mutations — rename, reorder, archive — scoped to the entity `lists()` so the row updates instantly; the global cache does the settle-invalidate. Tests lock instant-apply + rollback.
- [x] 6.2 Cross-window coherence is a live invalidation broadcast over the Tauri event bus (`lib/data/crossWindowSync.ts`): the global `MutationCache` handler broadcasts the declared `meta.invalidates` keys on a `query:invalidate` event; each window mounts a listener (in `AppRouter`) that invalidates matching queries on receipt and ignores its own broadcast. `refetchOnWindowFocus` disabled. A coalesce/dedupe guard (~80ms flush, both directions) bounds a mutation burst to one emit per window and one invalidation pass per receiver. Tests lock coalesce, dedupe, source-skip, and per-window invalidation counts.

## 7. Desktop e2e (tauri-webdriver)

- [x] 7.1 e2e: window-intent dispatch under the custom-scheme origin — covered by the passing `a workspace window scopes the sidebar to its own projects` and `a project window scopes the sidebar to that project only` specs (child windows load with `?w=` and the shell renders the scoped surface).
- [x] 7.2 The component reorganization + monolith split changed structure only, not rendered DOM; every e2e affordance (labels, testids, context menus, detach/re-attach, routes) preserved and the full e2e suite stays green.

## 8. Final verify gate

- [x] 8.1 `bun run verify` green (format:check + check-types + lint + test). (Note: local `cargo fmt --check` flags an unrelated `apps/memorya` file from a rustfmt version skew; CI's rustfmt accepts the committed form.)
- [x] 8.2 Desktop e2e suite green. Fixed a routing regression where `validateSearch` narrowed the search to `WindowIntent` and dropped the logs `?service=` filter — now a passthrough record with intent derived in RootShell.
- [x] 8.3 Spec scenarios ↔ tests checked 1:1; the `client-engine` scenarios each have a matching unit test.
- [x] 8.4 ast-grep rules added: no `fetch` and no inline dynamic `import()` inside hooks/components (dynamic imports centralized in `lib/lazy.ts` + `lib/transport/core.ts`).

## 9. File-based routing migration (ADR-0040 — supersedes the code-based routing decision)

- [x] 9.1 Add deps `@tanstack/router-plugin` + `@tanstack/react-router-devtools`; wire `vite.config.ts` `tanstackRouter({ target: 'react', autoCodeSplitting: true, routesDirectory: 'app/routes', generatedRouteTree: 'app/routeTree.gen.ts' })` BEFORE `react()`. `routeTree.gen.ts` ignored by oxfmt + oxlint + both ast-grep inline-dynamic-import rules; tsconfig `**/*` include already covers it.
- [x] 9.2 Deep-route reload: rely on Tauri v2's built-in `index.html` asset fallback (default-on; ADR-0039's "no fallback" premise was wrong) — NO custom `src-tauri` handler. Proven by the reload-at-deep-route e2e in 9.5; a handler is added only if that test fails.
- [x] 9.3 Create the file-based tree under `app/routes/`: `__root.tsx` (RootLayout: validateSearch passthrough, `?w=` dispatch — detached→DetachedWindow else chrome+`<Outlet/>`, devtools dev-only), `index.tsx` (`/`), `session.$id.tsx` (`/session/$id` typed param), `logs.tsx` (`/logs`). Let the plugin generate `routeTree.gen.ts`; `router.tsx` imports it + keeps `Register` + `createBrowserHistory`.
- [x] 9.4 Decompose `AppShell` → RootLayout (chrome: providers, `SessionContext` with sessionId = `useParams({strict:false}).id ?? intentSessionId`, sidebar scoped by intent, command center, indicators, view-logs/settings commands) + content route components (panels route owns `usePanelTree`/`useDetachedPanels`/panel commands; logs route renders the viewer reading `?service`). Remove the `useShellLocation` pathname regex.
- [x] 9.5 Verify behavior preserved: 224 UI unit + full e2e (nav, detach/re-attach, workspace/project scoping, logs `?service`) + **new reload-at-deep-route e2e** + 55 Rust + ast-grep all green. Update `docs/tanstack-client-engine.md` conventions + memory `client-engine.md`.
