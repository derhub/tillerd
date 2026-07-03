import { createSyncStoragePersister } from "@tanstack/query-sync-storage-persister";
import { MutationCache, QueryClient } from "@tanstack/react-query";

import { broadcastInvalidate } from "./crossWindowSync";

export const PERSIST_KEY = "tillerd:query-cache";
export const PERSIST_BUSTER = "1.0.0";
export const PERSIST_MAX_AGE = 24 * 60 * 60 * 1000; // 24 hours

export const queryPersister =
  typeof window !== "undefined" && typeof window.localStorage !== "undefined"
    ? // eslint-disable-next-line @typescript-eslint/no-deprecated
      createSyncStoragePersister({
        key: PERSIST_KEY,
        storage: window.localStorage,
      })
    : undefined;

const PERSISTED_ENTITIES = new Set([
  "workspaces",
  "projects",
  "sessions",
  "logs",
  "settings",
  "templates",
]);

export function shouldDehydrateQuery(query: any): boolean {
  if (query.state.status !== "success") return false;
  const entity = query.queryKey[0];
  return typeof entity === "string" && PERSISTED_ENTITIES.has(entity);
}

// Restored data may be seconds old (the persister's throttled write races app close), and staleTime
// would treat it as fresh -- no refetch, so a restart could render a pre-mutation snapshot forever.
// Invalidating everything after restore keeps cold start instant (cached data renders immediately)
// while every restored read revalidates once the client is ready.
export function onPersistRestored(queryClient: QueryClient): void {
  void queryClient.invalidateQueries();
}

// Each window creates its own QueryClient so caches do not leak across windows.
export function makeQueryClient(): QueryClient {
  // Invalidation after a successful mutation is automatic and configured once here, not repeated in
  // every hook's onSuccess. A mutation declares the keys it touches via `meta.invalidates`; this
  // handler invalidates exactly those and never the whole cache. A mutation that declares no keys
  // invalidates nothing -- a missing declaration is a visible bug, not a silent cache wipe, so every
  // write mutation MUST set meta.invalidates.
  const mutationCache = new MutationCache({
    onSuccess: (_data, _vars, _ctx, mutation) => {
      const keys = mutation.meta?.invalidates;
      keys?.forEach((queryKey) => void queryClient.invalidateQueries({ queryKey }));
      if (keys?.length) broadcastInvalidate(keys);
    },
    // No onError recording here: the orchestrator records every failed command as a
    // `command-error` notification and pushes it over the notification channel -- the
    // renderer only displays, it never records. Optimistic mutations still roll back
    // via their own onError.
  });

  const queryClient = new QueryClient({
    mutationCache,
    defaultOptions: {
      queries: {
        // A short stale floor: explicit invalidation (mutation meta) drives freshness in-window and,
        // via broadcastInvalidate, across windows -- so cached reads need not refetch on every mount.
        staleTime: 30_000,
        retry: false,
        // Cross-window coherence is now the live invalidation broadcast, not focus refetch. Disabling
        // refetch-on-focus avoids a refetch storm when many windows regain focus together.
        refetchOnWindowFocus: false,
      },
    },
  });
  return queryClient;
}
