## Context

First paint waits on orchestrator-process boot, not data latency (`tillerd.db` is local). Persisting
the last-known Query cache client-side lets the shell paint immediately on relaunch and revalidate
once ready. Storage is **browser-native only** (localStorage / IndexedDB) -- portable to the web host,
no native-plugin coupling, identical on both hosts.

## Goals / Non-Goals

- **Goals:** persist the per-window Query cache + active-workspace to browser storage; hydrate before
  first paint; stale-while-revalidate on boot; bounded by `maxAge` + version `buster`.
- **Non-Goals:** native `tauri-plugin-store`/fs; persisting ephemeral/stream data; cross-window shared
  cold-start cache (each window cold-starts from its own; runtime coherence is the broadcast).

## Decisions

- **`PersistQueryClientProvider` + sync localStorage persister.** `@tanstack/react-query-persist-client`
  + `@tanstack/query-sync-storage-persister` over `localStorage`. Synchronous restore = the cache is
  present on first render (no async restore flash). Wrap at the same level as `QueryClientProvider`
  in `AppRouter`, one persister per window.
- **Bound + invalidate.** `maxAge` ~24h (drop very stale cache); `buster = app version` (drop on
  upgrade so shape changes can't deserialize wrong); `dehydrateOptions.shouldDehydrateQuery` persists
  only `success` queries (never pending/error). Keys are the existing `queryOptions` keys.
- **Persist scope (what to cache).** workspaces / projects / sessions lists, session layout, log-file
  list, active-workspace (`uiStore`). Excluded by the dehydrate filter / by not being queries:
  terminal PTY output, orchestrator status, in-flight mutations, large diff bodies, live notification
  feed (orchestrator replays on boot).
- **`uiStore` persistence.** Read the persisted active-workspace synchronously at store init; write on
  change (small, its own `localStorage` key). Restores sidebar scope instantly.
- **localStorage now, IndexedDB later.** The cache (lists + layouts) is small -> localStorage's sync
  hydrate is ideal. If it outgrows ~5MB, swap the persister to an IndexedDB-backed async persister
  (the only change; `PersistQueryClientProvider` shows nothing-or-children until restored).
- **Multi-window.** Each window has its own `QueryClient`; macOS WKWebView windows are separate
  processes that may not share `localStorage`, so each cold-starts from its own persisted cache --
  acceptable (they want the same lists). Runtime coherence is the cross-window invalidation broadcast,
  not shared storage. Optionally namespace the persist key by window label to avoid write races.

## Risks / Trade-offs

- **Stale-on-paint.** Persisted data may be briefly stale until revalidation; acceptable
  (stale-while-revalidate) and bounded by `maxAge`. Mutations + broadcast keep it fresh once ready.
- **Schema drift.** A cached shape that no longer matches after an upgrade -> `buster = version`
  drops the cache; the dehydrate filter avoids persisting partials.
- **localStorage budget.** ~5MB; current cache is far under. IndexedDB is the documented escape hatch
  if it grows (e.g. large session lists).
- **Depends on `ui-route-loaders`.** The revalidation path (queryFn awaits `whenClientReady`) lands
  there; persistence layers on top.
