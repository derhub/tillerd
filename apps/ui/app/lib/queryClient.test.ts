import { QueryClient } from "@tanstack/react-query";
import { describe, expect, test } from "bun:test";

import { onPersistRestored, shouldDehydrateQuery } from "./queryClient";

describe("queryClient persistence filter", () => {
  test("only success queries are dehydrated", () => {
    const pendingQuery = {
      state: { status: "pending" },
      queryKey: ["workspaces", "list", null],
    };
    const errorQuery = {
      state: { status: "error" },
      queryKey: ["workspaces", "list", null],
    };
    const successQuery = {
      state: { status: "success" },
      queryKey: ["workspaces", "list", null],
    };

    expect(shouldDehydrateQuery(pendingQuery)).toBe(false);
    expect(shouldDehydrateQuery(errorQuery)).toBe(false);
    expect(shouldDehydrateQuery(successQuery)).toBe(true);
  });

  test("only whitelisted entities are dehydrated", () => {
    const successWorkspace = {
      state: { status: "success" },
      queryKey: ["workspaces", "list", null],
    };
    const successDiff = {
      state: { status: "success" },
      queryKey: ["diffs", "get", { id: "1" }],
    };
    const successNotifications = {
      state: { status: "success" },
      queryKey: ["notifications", "list", null],
    };

    expect(shouldDehydrateQuery(successWorkspace)).toBe(true);
    expect(shouldDehydrateQuery(successDiff)).toBe(false);
    expect(shouldDehydrateQuery(successNotifications)).toBe(false);
  });
});

describe("restore revalidation", () => {
  test("a restored query is invalidated so a cold start revalidates against the backend", () => {
    const qc = new QueryClient({
      defaultOptions: { queries: { staleTime: 30_000, retry: false } },
    });
    // Seed a query the way a persister restore leaves it: success data with a recent
    // dataUpdatedAt, which staleTime would otherwise treat as fresh (no refetch on mount).
    qc.setQueryData(["projects", "list", null], [{ id: "p1" }]);
    expect(qc.getQueryState(["projects", "list", null])?.isInvalidated).toBe(false);

    onPersistRestored(qc);

    expect(qc.getQueryState(["projects", "list", null])?.isInvalidated).toBe(true);
  });
});
