## Why

The shell shows a skeleton until the embedded orchestrator process spawns and reaches ready -- not
because data is slow (`tillerd.db` is local SQLite) but because the process takes time to boot. If the
last-known server-state is persisted client-side, the shell can paint instantly from cache on
relaunch and revalidate once the orchestrator is ready -- decoupling first paint from orchestrator
boot. This pairs with render-as-you-fetch (`ui-route-loaders`): a persisted cache means the suspense
reads have data on mount and never suspend on a cold start.

## What Changes

- **Persist the per-window TanStack Query cache to webview storage.** Add
  `@tanstack/react-query-persist-client` + `@tanstack/query-sync-storage-persister`; wrap with
  `PersistQueryClientProvider` over a synchronous `localStorage` persister so the cache hydrates
  before first paint. `maxAge` ~24h, `buster = app version` (drop cache on version change), persist
  only successful queries (`dehydrateOptions` filters pending/error).
- **Persist the active-workspace selection** (`uiStore`) so the sidebar restores its scope instantly.
- **Cache scope:** workspaces / projects / sessions lists, session layout, log-file list, active
  workspace. NOT persisted: terminal PTY output (re-binds via replay), orchestrator status, in-flight
  mutations, large diff bodies, the live notification feed (orchestrator replays on boot).
- **Stale-while-revalidate:** persisted data renders immediately; the queryFn (post `ui-route-loaders`,
  awaits `whenClientReady`) revalidates when the orchestrator is ready. Cross-window broadcast keeps
  live windows coherent; persistence only covers cold start.
- **Browser storage only** -- `localStorage` (sync hydrate) now, `IndexedDB` if the cache outgrows the
  ~5MB localStorage budget. NO `tauri-plugin-store`/native: browser storage is portable to the web
  host, avoids cross-platform webview-store API differences, and keeps the persister identical on both
  hosts. Server source-of-truth stays `tillerd.db` (native, unchanged).

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `client-engine`: the data-flow requirement gains client-side cache persistence -- the per-window
  Query cache and the active-workspace selection persist to webview storage and hydrate before first
  paint, so the shell renders last-known state on relaunch and revalidates when the client is ready.
  Server source-of-truth (`tillerd.db`), mutation auto-invalidate, cross-window broadcast, and Store
  semantics are unchanged.

## Impact

- `apps/ui`: `app/router.tsx` (`PersistQueryClientProvider` + persister), `app/lib/queryClient.ts`
  (persist options: maxAge/buster/dehydrate filter), `app/lib/store.ts` (persist + rehydrate
  `uiStore`). New deps: `@tanstack/react-query-persist-client`, `@tanstack/query-sync-storage-persister`.
- No backend change. `tillerd.db` remains source of truth.
- Sequencing: depends on `ui-route-loaders` (render-as-you-fetch) being applied; archive
  `tanstack-client-engine` first so `client-engine` is canonical.
- Verification: cache hydrates before paint (unit) + cold-start render-from-cache (e2e: relaunch shows
  last-known sidebar before ready) + full suite stays green.
