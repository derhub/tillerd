import type { Workspace } from "@tillerd/client-bindings";

import { useSuspenseQuery } from "@tanstack/react-query";
import { cleanup, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, expect, test } from "bun:test";

import { renderWithSuspense } from "./suspense";

// The subject under test is the readiness gate: a suspense read must hold the fallback until the
// client signals ready, then resolve. Drive it through the REAL readiness module (un-mocked source
// submodule, immune to sibling suites stubbing the `@tillerd/client-bindings` package specifier).
// The data source is local so the assertion does not race a process-global `@tauri-apps/api/core` mock.
const { whenReady, setReady } =
  await import("../../../../../packages/client-bindings/src/readiness");

let fakeWorkspaces: Workspace[] = [];

// Mirrors the real query() wrapper: an array queryKey plus a queryFn gated on whenReady().
function workspaceListQuery() {
  return {
    queryKey: ["workspaces", "list", null] as const,
    queryFn: async (): Promise<Workspace[]> => {
      while (!(await whenReady())) {
        /* await next readiness promise */
      }
      return fakeWorkspaces;
    },
  };
}

afterEach(() => {
  cleanup();
  fakeWorkspaces = [];
  setReady(false);
});

function Probe() {
  const { data } = useSuspenseQuery(workspaceListQuery());
  return <div data-testid="content">{data.length} workspaces</div>;
}

test("a suspense read shows the fallback until the client is ready, then the content", async () => {
  // The shared test preload defaults every test to ready; this suite drives the not-ready -> ready
  // transition, so reset to not-ready before rendering.
  setReady(false);
  fakeWorkspaces = [{ id: "ws-1", name: "Default" }];

  renderWithSuspense(<Probe />);

  expect(screen.queryByTestId("suspense-fallback")).not.toBeNull();
  expect(screen.queryByTestId("content")).toBeNull();

  setReady(true);
  await waitFor(() => expect(screen.queryByTestId("content")).not.toBeNull());
  expect(screen.getByTestId("content").textContent).toBe("1 workspaces");
  expect(screen.queryByTestId("suspense-fallback")).toBeNull();
});
