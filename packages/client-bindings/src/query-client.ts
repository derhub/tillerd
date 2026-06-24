import type { QueryClient } from "@tanstack/react-query";

// Module-level QueryClient so generated mutationOptions can run optimistic onMutate/onError
// without a hook. The app sets it once at boot (alongside setReady).
let client: QueryClient | null = null;

export function setQueryClient(next: QueryClient | null): void {
  client = next;
}

export function getQueryClient(): QueryClient {
  if (!client) throw new Error("QueryClient not set -- call setQueryClient at app boot");
  return client;
}
