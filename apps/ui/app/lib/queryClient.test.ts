import { MutationObserver, QueryClient } from "@tanstack/react-query";
/// <reference lib="dom" />
import { afterEach, beforeEach, test, expect, describe } from "bun:test";

import { notificationsStore } from "./notifications/context";
import { makeQueryClient } from "./queryClient";

const resetNotifications = () => notificationsStore.setState(() => ({ items: [], unread: 0 }));
beforeEach(resetNotifications);
afterEach(resetNotifications);

describe("makeQueryClient", () => {
  test("returns a QueryClient instance", () => {
    const client = makeQueryClient();
    expect(client).toBeInstanceOf(QueryClient);
  });

  test("each call returns a distinct client", () => {
    const a = makeQueryClient();
    const b = makeQueryClient();
    expect(a).not.toBe(b);
  });

  test("invalidateQueries marks a seeded query stale", async () => {
    const client = makeQueryClient();

    client.setQueryData(["sessions", "ws-a"], [{ id: "s1" }]);

    expect(client.getQueryData(["sessions", "ws-a"])).toEqual([{ id: "s1" }]);

    await client.invalidateQueries({ queryKey: ["sessions"] });

    const state = client.getQueryState(["sessions", "ws-a"]);
    expect(state?.isInvalidated).toBe(true);
  });

  test("invalidating a key does not affect an unrelated key", async () => {
    const client = makeQueryClient();

    client.setQueryData(["projects", "ws-a"], [{ id: "p1" }]);
    client.setQueryData(["sessions", "ws-a"], [{ id: "s1" }]);

    await client.invalidateQueries({ queryKey: ["sessions"] });

    const projectState = client.getQueryState(["projects", "ws-a"]);
    expect(projectState?.isInvalidated ?? false).toBe(false);
  });

  test("a failed mutation records an error notification", async () => {
    const client = makeQueryClient();
    const observer = new MutationObserver(client, {
      mutationFn: async () => {
        throw new Error("boom");
      },
    });

    await observer.mutate().catch(() => {});

    const items = notificationsStore.state.items;
    expect(items).toHaveLength(1);
    expect(items[0].severity).toBe("error");
    expect(items[0].message).toBe("boom");
  });
});
