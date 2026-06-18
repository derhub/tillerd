import { afterEach, expect, mock, test } from "bun:test";
import { act, cleanup, render, waitFor } from "@testing-library/react";
import * as realWindows from "~/lib/windows";

// The detach/re-attach lifecycle isn't e2e-driveable (WebDriver can't close the detached child
// window), so it's covered here. Re-attach is what the app fires when that window closes.

const alpha = { id: "ws-1", name: "Alpha" };
const beta = { id: "ws-2", name: "Beta" };

const opened: string[] = [];
let reattach: ((p: { workspaceId: string }) => void) | undefined;

mock.module("~/lib/useDesktopHost", () => ({
  useDesktopHost: () => ({
    status: "ready",
    orchestratorClient: { listWorkspaces: async () => [alpha, beta] },
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
