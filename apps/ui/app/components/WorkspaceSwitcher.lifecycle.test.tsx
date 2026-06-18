import { afterEach, expect, mock, test } from "bun:test";
import { act, cleanup, render, waitFor } from "@testing-library/react";
import * as realWindows from "~/lib/windows";

// The detach/re-attach lifecycle isn't e2e-driveable (WebDriver can't close the detached child
// window), so it's covered here. Re-attach is what the app fires when that window closes.

const alpha = { id: "ws-1", name: "Alpha" };
const beta = { id: "ws-2", name: "Beta" };

const opened: string[] = [];
const created: { name: string }[] = [];
let reattach: ((p: { workspaceId: string }) => void) | undefined;

mock.module("~/lib/useDesktopHost", () => ({
  useDesktopHost: () => ({
    status: "ready",
    orchestratorClient: {
      listWorkspaces: async () => [alpha, beta],
      createWorkspace: async ({ name }: { name: string }) => {
        created.push({ name });
        return { id: "ws-new", name };
      },
    },
  }),
}));

// mock.module is process-global; spread the real module so other tests' imports survive.
mock.module("~/lib/windows", () => ({
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

mock.module("~/components/SessionSidebar", () => ({ SessionSidebar: () => null }));

const { WorkspaceSwitcher } = await import("./WorkspaceSwitcher");

afterEach(() => {
  cleanup();
  opened.length = 0;
  created.length = 0;
  reattach = undefined;
});

const detachBtn = () =>
  document.querySelector('[data-testid="workspace-detach"][data-workspace-id="ws-1"]');
const detachedIndicator = () =>
  document.querySelector('[data-testid="workspace-detached-indicator"][data-workspace-id="ws-1"]');

test("detaching a workspace opens its window and re-attaching closes it back to attached", async () => {
  render(<WorkspaceSwitcher />);

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

// Guards the bug where New workspace bailed on a null prompt — window.prompt returns null in the
// Tauri webview, so the button must still create a workspace (under a default name).
test("New workspace still creates one when window.prompt returns null", async () => {
  const original = window.prompt;
  window.prompt = () => null;
  try {
    render(<WorkspaceSwitcher />);
    await waitFor(() =>
      expect(document.querySelector('[data-testid="new-workspace"]')).not.toBeNull(),
    );
    act(() => {
      (document.querySelector('[data-testid="new-workspace"]') as HTMLElement).click();
    });
    await waitFor(() => expect(created).toHaveLength(1));
    expect(created[0].name).toBe("New workspace");
  } finally {
    window.prompt = original;
  }
});
