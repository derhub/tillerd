# Tasks — client-side cache persistence (browser storage, fast cold start)

## 1. Query cache persistence

- [ ] 1.1 Add deps `@tanstack/react-query-persist-client` + `@tanstack/query-sync-storage-persister`.
- [ ] 1.2 `queryClient.ts`/`router.tsx`: wrap `AppRouter` with `PersistQueryClientProvider` over a `createSyncStoragePersister({ storage: localStorage })`. Options: `maxAge` ~24h, `buster = <app version>`, `dehydrateOptions.shouldDehydrateQuery = q => q.state.status === "success"`.
- [ ] 1.3 Verify synchronous hydrate: cache present on first render (no async-restore flash).

## 2. Active-workspace persistence

- [ ] 2.1 `store.ts`: initialize `uiStore.activeWorkspaceId` from a persisted `localStorage` key; subscribe to persist on change. Restores sidebar scope on cold start.

## 3. Scope guard

- [ ] 3.1 Confirm the dehydrate filter persists only the intended reads (workspaces/projects/sessions lists, session layout, log-file list) and that ephemeral data (terminal output, orchestrator status, in-flight mutations, diff bodies, notification feed) is NOT persisted.

## 4. Verify

- [ ] 4.1 Unit: persister round-trips a seeded cache; `buster` mismatch discards; only `success` queries dehydrate; `uiStore` restores from storage.
- [ ] 4.2 e2e: relaunch shows the last-known sidebar/lists before the orchestrator reaches ready (cold-start-from-cache), then revalidates. Full suite + `bun run verify` green.
- [ ] 4.3 Docs `docs/tanstack-client-engine.md` + memory `client-engine.md`: cache persistence (browser storage, maxAge/buster, what-to-cache, IndexedDB escape hatch, multi-window cold-start).

## Notes
- Browser storage ONLY (no `tauri-plugin-store`): portable to the web host, identical persister both hosts.
- Depends on `ui-route-loaders` (revalidation via queryFn `whenClientReady`); sequence after it.
