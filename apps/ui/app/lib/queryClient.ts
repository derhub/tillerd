import { MutationCache, QueryClient } from "@tanstack/react-query";

import { broadcastInvalidate } from "./crossWindowSync";
import { recordNotification } from "./notifications/context";
import { randomId } from "./transport/web-crypto";

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
    // Optimistic mutations roll back via their own onError; this is the separate user-facing signal.
    onError: (error) => {
      recordNotification({
        id: randomId(),
        category: "mutation-error",
        severity: "error",
        title: null,
        message: error instanceof Error ? error.message : "Action failed",
        detail: null,
        ts: Date.now(),
        sessionId: null,
        surfaceId: null,
      });
    },
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
