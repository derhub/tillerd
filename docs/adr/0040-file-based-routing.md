# 0040. File-based routing (Tauri's built-in index.html fallback covers deep-route reload)

- Status: accepted
- Date: 2026-06-23
- Supersedes: 0039 (routing decision only; Query/Store/coherence in 0039 stand)

## Context

ADR-0039 adopted **code-based single-root** routing: every window loads at `/` and dispatches on the
`?w=` search param, because the desktop custom-scheme origin serves `frontendDist` over Tauri's
default asset protocol, which 404s unmatched paths -- there is no deep-route SPA fallback. That
avoided the fallback problem but diverged from idiomatic TanStack Router: no typed path params, no
`Outlet`, the path parsed by a hand-rolled regex in `useShellLocation`, and no
`@tanstack/router-plugin` (file-based codegen, `autoCodeSplitting`, devtools).

The single deep-route hazard would be **reload**: a window navigated client-side to `/session/abc`
and then reloaded must still serve the app. ADR-0039 assumed the custom-scheme origin had no
fallback and would 404 -- **that premise was wrong**: Tauri v2's asset protocol falls back to
`index.html` for unmatched paths by default (tauri core "fallback to index.html on asset loading";
the opt-out is a feature request, issue #5082), so history-mode deep routes reload safely with no
custom handler. Child windows still open at `/?w=...` (root + search, see `window_host.rs`).

## Decision

Adopt **file-based routing**; deep routes are reload-safe via Tauri's built-in `index.html` fallback.

- **`@tanstack/router-plugin`** in `vite.config.ts` (`tanstackRouter({ target: 'react', autoCodeSplitting: true })`, before `react()`); routes live under `app/routes/` and the plugin generates `routeTree.gen.ts`. `@tanstack/react-router-devtools` mounts in the root layout (dev only).
- **Route tree:** `__root.tsx` (RootLayout), `index.tsx` (`/`), `session.$id.tsx` (`/session/$id`, typed `useParams`), `logs.tsx` (`/logs`). `router.tsx` imports the generated tree; `createBrowserHistory` + `Register` are kept.
- **Window intent stays at the root.** RootLayout reads `?w=` (validated passthrough search) and branches: `detached` -> `DetachedWindow`; otherwise it renders the chrome (providers, sidebar scoped by intent, command center, indicators) around an `<Outlet/>`. Path-based content (panels vs logs) is the child routes. The `useShellLocation` pathname regex is removed.
- **Deep-route reload: Tauri's built-in fallback.** Tauri v2 serves `index.html` for unmatched
  non-asset paths by default, so a reload at `/session/abc` loads the app and the router matches
  client-side. No custom `src-tauri` handler is added; no IPC/wire/data-model seam is touched. A
  reload-at-deep-route e2e proves it -- a handler is added only if that test fails.
- **Tooling:** `routeTree.gen.ts` is added to the tsconfig include and ignored by oxfmt, oxlint, and
  ast-grep (generated, not hand-edited).

The Query/Store/coherence model from ADR-0039 (per-window `QueryClient`, global `MutationCache`
auto-invalidate, cross-window invalidation broadcast, TanStack Store for client UI state) is
unchanged.

## Consequences

- Idiomatic routing: typed path params, `Outlet`, per-route boundaries that can later carry loaders /
  `pendingComponent` / `errorComponent`; `autoCodeSplitting` available (limited value today -- the
  content components are small and heavy deps already lazy-load via `lib/lazy`).
- `AppShell` decomposes into RootLayout (chrome) + per-route content components; `SessionContext`
  (spanning sidebar chrome and terminal content) and the command registry stay at the root so the
  palette still sees panel commands.
- No desktop change: deep-route reload rides Tauri's built-in `index.html` fallback (the migration is
  UI-only). A reload-at-deep-route e2e guards the assumption.
- Generated `routeTree.gen.ts` is committed but tooling-ignored; renaming a route regenerates it.
- Behavior is preserved: window-intent dispatch, detach/re-attach, workspace/project scoping, the logs
  `?service` filter, and client navigation are unchanged and gated by the existing e2e suite.

## Alternatives considered

- **Keep code-based single-root (ADR-0039).** Works and ships green, but non-idiomatic (regex path
  dispatch, no typed params, no plugin). Rejected in favour of framework conventions.
- **Hash history (`createHashHistory`).** `/#/session/abc` always loads `index.html`. Unnecessary
  once the built-in fallback is confirmed; it would also reshape the `?w=` window-intent URL scheme
  and yield uglier URLs. Browser history is kept.
- **Custom `src-tauri` asset handler.** Considered while ADR-0039's "no fallback" premise stood;
  dropped once Tauri's built-in fallback was confirmed (avoids touching frozen-adjacent desktop code).
