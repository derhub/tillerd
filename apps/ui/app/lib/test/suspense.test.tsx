import type { Workspace } from "@tillerd/client-bindings";

import { useSuspenseQuery } from "@tanstack/react-query";
import { cleanup, screen, waitFor } from "@testing-library/react";
import { setReady } from "@tillerd/client-bindings";
/// <reference lib="dom" />
import { afterEach, expect, mock, test } from "bun:test";

import { renderWithSuspense } from "./suspense";

// invoke must return raw data, not a typed-error shape (typedError() wraps it).
let fakeWorkspaces: Workspace[] = [];
void mock.module("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string) => {
    if (cmd === "workspace_list") return fakeWorkspaces;
    return [];
  },
  Channel: class Channel {
    onmessage: ((v: unknown) => void) | null = null;
  },
}));

const { query } = await import("@tillerd/client-bindings");

afterEach(() => {
  cleanup();
  fakeWorkspaces = [];
  setReady(false);
});

function Probe() {
  const { data } = useSuspenseQuery(query("workspaceList"));
  return <div data-testid="content">{data.length} workspaces</div>;
}

test("a suspense read shows the fallback until the client is ready, then the content", async () => {
  fakeWorkspaces = [{ id: "ws-1", name: "Default" }];

  renderWithSuspense(<Probe />);

  expect(screen.queryByTestId("suspense-fallback")).not.toBeNull();
  expect(screen.queryByTestId("content")).toBeNull();

  setReady(true);
  await waitFor(() => expect(screen.queryByTestId("content")).not.toBeNull());
  expect(screen.getByTestId("content").textContent).toBe("1 workspaces");
  expect(screen.queryByTestId("suspense-fallback")).toBeNull();
});
