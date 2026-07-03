import type { QueryClient } from "@tanstack/react-query";

import { describe, expect, test } from "bun:test";

import { mountSurfaceStatusSync, type SurfaceStatusSyncDeps } from "./surfaceStatusSync";

// Deps are injected (no module mocks): fakes stand in only at the transport edge.
function harness() {
  const invalidated: unknown[] = [];
  const client = {
    invalidateQueries: (filter: { queryKey: unknown }) => {
      invalidated.push(filter.queryKey);
      return Promise.resolve();
    },
  } as unknown as QueryClient;

  let callback: ((event: unknown) => void) | undefined;
  let closed = 0;
  const deps: SurfaceStatusSyncDeps = {
    isDesktop: () => true,
    ready: () => Promise.resolve(true),
    open: ((cb: (event: unknown) => void) => {
      callback = cb;
      return Promise.resolve({
        close: () => {
          closed += 1;
          return Promise.resolve();
        },
      });
    }) as SurfaceStatusSyncDeps["open"],
  };

  return {
    client,
    invalidated,
    deps,
    fire: (event: unknown) => callback?.(event as never),
    closedCount: () => closed,
  };
}

const flush = () => new Promise((r) => setTimeout(r, 120));

describe("surface-status push invalidation", () => {
  test("a push event invalidates the activity rollup and surface reads", async () => {
    const h = harness();
    const cleanup = mountSurfaceStatusSync(h.client, h.deps);
    await flush();

    h.fire({ surfaceId: "sf", sessionId: "s", workspaceId: "w", status: "failed" });
    await flush();

    expect(h.invalidated).toContainEqual(["workspaces", "activity"]);
    expect(h.invalidated).toContainEqual(["surfaces"]);
    cleanup();
  });

  test("a burst of events coalesces to one invalidation pass", async () => {
    const h = harness();
    const cleanup = mountSurfaceStatusSync(h.client, h.deps);
    await flush();

    for (let i = 0; i < 5; i++) {
      h.fire({ surfaceId: `sf-${i}`, sessionId: "s", workspaceId: "w", status: "live" });
    }
    await flush();

    // One pass = one invalidation per key, not one per event.
    expect(h.invalidated.filter((k) => JSON.stringify(k) === '["surfaces"]')).toHaveLength(1);
    cleanup();
  });

  test("off the desktop host nothing mounts", () => {
    const h = harness();
    const cleanup = mountSurfaceStatusSync(h.client, { ...h.deps, isDesktop: () => false });
    cleanup();
    expect(h.invalidated).toHaveLength(0);
  });

  test("cleanup closes the channel", async () => {
    const h = harness();
    const cleanup = mountSurfaceStatusSync(h.client, h.deps);
    await flush();

    cleanup();
    await flush();

    expect(h.closedCount()).toBe(1);
  });
});
