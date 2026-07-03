// Subscription-driven activity invalidation : the orchestrator pushes a
// surface-status event after each status write commits; this window invalidates the
// activity rollup and surface reads so the next render reads fresh data. A spawn
// burst coalesces to one invalidation pass (same trailing-window guard as
// crossWindowSync); TanStack dedupes concurrent refetches in-flight.

import type { QueryClient, QueryKey } from "@tanstack/react-query";

import {
  surfaceStatusChannel,
  whenReady,
  type SurfaceStatusChannelHandle,
} from "@tillerd/client-bindings";

import { isDesktopHost } from "./transport/core";

const FLUSH_MS = 80;

// The activity rollup rides the workspaces cache root; surface reads ride surfaces.
const INVALIDATE_ON_STATUS: QueryKey[] = [["workspaces", "activity"], ["surfaces"]];

export function invalidateForSurfaceStatus(client: QueryClient): void {
  for (const queryKey of INVALIDATE_ON_STATUS) {
    void client.invalidateQueries({ queryKey });
  }
}

// Injectable edges (tests pass fakes; production uses the real bindings).
export interface SurfaceStatusSyncDeps {
  isDesktop: () => boolean;
  ready: () => Promise<boolean>;
  open: typeof surfaceStatusChannel;
}

const realDeps: SurfaceStatusSyncDeps = {
  isDesktop: isDesktopHost,
  ready: whenReady,
  open: surfaceStatusChannel,
};

// Mount once per window. Resolves the subscription after the client is ready;
// returns a cleanup that closes the channel (or cancels an in-flight open).
export function mountSurfaceStatusSync(
  client: QueryClient,
  deps: SurfaceStatusSyncDeps = realDeps,
): () => void {
  if (!deps.isDesktop()) return () => {};

  let disposed = false;
  let handle: SurfaceStatusChannelHandle | undefined;
  let timer: ReturnType<typeof setTimeout> | null = null;

  void (async () => {
    while (!(await deps.ready())) {
      /* await the next readiness promise */
    }
    if (disposed) return;
    handle = await deps.open(() => {
      if (timer !== null) return;
      timer = setTimeout(() => {
        timer = null;
        invalidateForSurfaceStatus(client);
      }, FLUSH_MS);
    });
    if (disposed) void handle.close();
  })();

  return () => {
    disposed = true;
    if (timer !== null) clearTimeout(timer);
    void handle?.close();
  };
}
