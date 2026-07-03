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
let reattach: ((p: { workspaceId: string }) => void) | undefined;

void mock.module("~/lib/useDesktopHost", () => ({
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

// typedError() wraps invoke's return value -- invoke must return raw data, not a typed-error shape.
void mock.module("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string, args?: Record<string, unknown>) => {
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
    return null;
  },
  Channel: class Channel {
    onmessage: ((v: unknown) => void) | null = null;
  },
}));

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
  reattach = undefined;
  setActiveWorkspace(null);
  setReady(false);
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

// Lifecycle resolution (ADR-0044): a pointer to a deleted workspace resolves to the
// Default workspace and the pointer is rewritten once — never an error or empty shell.
test("a stale active-workspace pointer falls back to the Default workspace", async () => {
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
