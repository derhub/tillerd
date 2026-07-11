import { QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRouter,
} from "@tanstack/react-router";
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { setReady } from "@tillerd/client-bindings";
import { setQueryClient } from "@tillerd/client-bindings";
/// <reference lib="dom" />
import { afterEach, expect, mock, test } from "bun:test";
import React from "react";

import { makeQueryClient } from "~/lib/queryClient";
import { setActiveWorkspace } from "~/lib/store";
import * as realWindows from "~/lib/windows";

// The detach/re-attach lifecycle isn't e2e-driveable (WebDriver can't close the detached child
// window), so it's covered here. Re-attach is what the app fires when that window closes.

const alpha = { id: "ws-1", name: "Alpha" };
const beta = { id: "ws-2", name: "Beta" };

const opened: string[] = [];
const created: { name: string }[] = [];
let workspaceList: { id: string; name: string }[] = [alpha, beta];
let workspaceActivity: { workspaceId: string; running: number; failed: number }[] = [];
let reattach: ((p: { workspaceId: string }) => void) | undefined;

// mock.module is process-global; spread the real module so `desktopHostStore` (imported by
// other suites, e.g. NotificationIndicator) survives once this mock is installed.
const actualDesktopHost = await import("~/lib/useDesktopHost");
void mock.module("~/lib/useDesktopHost", () => ({
  ...actualDesktopHost,
  useDesktopHost: () => ({ status: "ready" }),
}));

// mock.module is process-global; spread the real module so other tests' imports survive.
void mock.module("~/lib/windows", () => ({
  ...realWindows,
  openWindow: async (label: string) => {
    opened.push(label);
  },
  onReattachWorkspace: (cb: (p: { workspaceId: string }) => void) => {
    reattach = cb;
    return Promise.resolve(() => {
      reattach = undefined;
    });
  },
}));

import { beforeEach } from "bun:test";

beforeEach(() => {
  (globalThis as any).__tillerd_set_invoke_mock(
    async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "workspace_list") return [...workspaceList];
      if (cmd === "workspace_create") {
        const name = args?.["name"] as string;
        created.push({ name });
        const ws = { id: "ws-new", name };
        workspaceList.push(ws);
        return ws;
      }
      if (cmd === "project_list") return [];
      if (cmd === "session_list") return [];
      if (cmd === "workspace_activity") return [...workspaceActivity];
      return undefined;
    },
  );
});

const { WorkspaceSwitcher } = await import("./WorkspaceSwitcher");

function installClient() {
  const queryClient = makeQueryClient();
  setQueryClient(queryClient);
  setReady(true);
  return queryClient;
}

afterEach(() => {
  cleanup();
  opened.length = 0;
  created.length = 0;
  workspaceList = [alpha, beta];
  workspaceActivity = [];
  reattach = undefined;
  setActiveWorkspace(null);
  setReady(false);
  (globalThis as any).__tillerd_clear_invoke_mock();
});

const detachBtn = () =>
  document.querySelector('[data-testid="workspace-detach"][data-workspace-id="ws-1"]');
const detachedIndicator = () =>
  document.querySelector('[data-testid="workspace-detached-indicator"][data-workspace-id="ws-1"]');

function withQuery(ui: React.ReactNode) {
  const client = installClient();
  const rootRoute = createRootRoute({
    component: () => (
      <QueryClientProvider client={client}>
        <React.Suspense fallback={<div data-testid="sidebar-skeleton" />}>{ui}</React.Suspense>
      </QueryClientProvider>
    ),
  });
  const router = createRouter({
    routeTree: rootRoute,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  return render(<RouterProvider router={router as never} />);
}

test("detaching a workspace opens its window and re-attaching closes it back to attached", async () => {
  withQuery(<WorkspaceSwitcher />);

  await waitFor(() => expect(detachBtn()).not.toBeNull());

  act(() => {
    (detachBtn() as HTMLElement).click();
  });
  await waitFor(() => expect(detachedIndicator()).not.toBeNull());
  expect(opened).toContain("workspace-ws-1");
  expect(detachBtn()).toBeNull();

  act(() => {
    reattach?.({ workspaceId: "ws-1" });
  });
  await waitFor(() => expect(detachedIndicator()).toBeNull());
  expect(detachBtn()).not.toBeNull();
});

// The activity dot proves the workspace-activity read-model end to end: rollup rows
// render as a per-workspace indicator colored by the worst state.
test("the activity dot reflects the rollup for its workspace", async () => {
  workspaceActivity = [
    { workspaceId: "ws-1", running: 2, failed: 0 },
    { workspaceId: "ws-2", running: 0, failed: 0 },
  ];

  withQuery(<WorkspaceSwitcher />);

  await waitFor(() => {
    const dot = document.querySelector('[data-testid="workspace-activity"]');
    expect(dot).not.toBeNull();
    expect(dot?.getAttribute("data-running")).toBe("2");
  });
  // ws-2 has no live/failed surfaces: exactly one dot renders.
  expect(document.querySelectorAll('[data-testid="workspace-activity"]')).toHaveLength(1);
});

// Lifecycle resolution: a pointer to a deleted workspace renders the Default
// scope -- never an error or empty shell. The pointer itself is NOT rewritten
// for a merely-absent id (the list can be a stale snapshot missing a young
// workspace); it self-heals if the workspace reappears.
test("a pointer to an absent workspace renders Default without rewriting", async () => {
  const defaultWs = { id: "00000000-0000-0000-0000-000000000001", name: "Default" };
  workspaceList = [defaultWs, alpha];
  setActiveWorkspace("ws-deleted");

  withQuery(<WorkspaceSwitcher />);

  await waitFor(() => {
    const active = document.querySelector(
      `[data-testid="workspace-item"][data-workspace-id="${defaultWs.id}"]`,
    );
    expect(active?.className ?? "").toContain("font-medium");
  });
  const { settingsStore } = await import("~/lib/settings/context");
  expect(settingsStore.state.values["view.active-workspace"]).toBe("ws-deleted");
});

// A target the list KNOWS is archived is a settled fact: render Default AND
// rewrite the pointer once so it does not re-resolve every start.
test("a pointer to an archived workspace falls back and is rewritten", async () => {
  const defaultWs = { id: "00000000-0000-0000-0000-000000000001", name: "Default" };
  const archivedWs = { id: "ws-arch", name: "Archived", status: "archived" };
  workspaceList = [defaultWs, alpha, archivedWs];
  setActiveWorkspace("ws-arch");

  withQuery(<WorkspaceSwitcher />);

  await waitFor(() => {
    const active = document.querySelector(
      `[data-testid="workspace-item"][data-workspace-id="${defaultWs.id}"]`,
    );
    expect(active?.className ?? "").toContain("font-medium");
  });
  const { settingsStore } = await import("~/lib/settings/context");
  await waitFor(() =>
    expect(settingsStore.state.values["view.active-workspace"]).toBe(defaultWs.id),
  );
});

// New workspace must not depend on window.prompt (unreliable in the Tauri webview): it creates a
// placeholder workspace and drops straight into inline rename.
test("New workspace creates a placeholder and opens it for inline rename", async () => {
  withQuery(<WorkspaceSwitcher />);
  await waitFor(() =>
    expect(document.querySelector('[data-testid="new-workspace"]')).not.toBeNull(),
  );
  act(() => {
    (document.querySelector('[data-testid="new-workspace"]') as HTMLElement).click();
  });
  await waitFor(() => expect(created).toHaveLength(1));
  expect(created[0].name).toBe("New workspace");
  await waitFor(() =>
    expect(document.querySelector('[data-testid="inline-rename-input"]')).not.toBeNull(),
  );
});
