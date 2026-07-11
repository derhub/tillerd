import type { SurfaceChannelEvent, SurfaceChannelHandle } from "@tillerd/client-bindings";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

// The failure overlay is a channel-event effect, not a DOM/render concern of xterm.js itself --
// stub the Terminal/FitAddon so the test drives real component state without a real xterm mount.
class FakeTerminal {
  cols = 80;
  rows = 24;
  options: Record<string, unknown> = {};
  onData() {
    return { dispose() {} };
  }
  onResize() {
    return { dispose() {} };
  }
  onBell() {
    return { dispose() {} };
  }
  onSelectionChange() {
    return { dispose() {} };
  }
  attachCustomKeyEventHandler() {}
  getSelection() {
    return "";
  }
  focus() {}
  paste() {}
  selectAll() {}
  clear() {}
  reset() {}
  open() {}
  dispose() {}
  write() {}
  loadAddon() {}
}
class FakeFitAddon {
  fit() {}
}
class FakeSearchAddon {
  findNext() {}
  findPrevious() {}
  clearDecorations() {}
  onDidChangeResults() {
    return { dispose() {} };
  }
  dispose() {}
}
class FakeWebLinksAddon {
  dispose() {}
}

// mock.module is process-global; spread the real module so its other exports (lazyDiffs, ...)
// survive for suites that run after this one.
const actualLazy = await import("~/lib/lazy");
void mock.module("~/lib/lazy", () => ({
  ...actualLazy,
  lazyXterm: () => Promise.resolve({ Terminal: FakeTerminal }),
  lazyFitAddon: () => Promise.resolve({ FitAddon: FakeFitAddon }),
  lazySearchAddon: () => Promise.resolve({ SearchAddon: FakeSearchAddon }),
  lazyWebLinksAddon: () => Promise.resolve({ WebLinksAddon: FakeWebLinksAddon }),
}));

let channelListener: ((event: SurfaceChannelEvent) => void) | null = null;
const closeSpy = mock(() => Promise.resolve());

const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  runCommand: async () => ({ id: "surf-1" }),
  surfaceChannel: async (
    _params: { surfaceId: string },
    callback: (event: SurfaceChannelEvent) => void,
  ): Promise<SurfaceChannelHandle> => {
    channelListener = callback;
    return { send: async () => {}, close: closeSpy };
  },
  // Keep the real `command` (via the spread above): globally overriding it to a no-op
  // mutation leaks into every later suite's mutations. The pane's only mutation
  // (surfaceDetach) never fires here because renderPane sets detachOnUnmount={false}.
}));

const { DesktopTerminalPane } = await import("./DesktopTerminalPane");

const onRequestResetSpy = mock(() => {});
const onStatusChangeSpy = mock(() => {});

function renderPane(props?: Partial<React.ComponentProps<typeof DesktopTerminalPane>>) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <DesktopTerminalPane
        sessionId="session-1"
        placement="main"
        cwd="/tmp"
        detachOnUnmount={false}
        onRequestReset={onRequestResetSpy}
        onStatusChange={onStatusChangeSpy}
        {...props}
      />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  channelListener = null;
  closeSpy.mockClear();
  onRequestResetSpy.mockClear();
  onStatusChangeSpy.mockClear();
});

afterAll(() => {
  mock.restore();
});

describe("DesktopTerminalPane surface failure overlay", () => {
  test("no overlay while the surface runs normally", async () => {
    renderPane();
    await waitFor(() => expect(channelListener).not.toBeNull());

    act(() => channelListener?.({ kind: "status", value: "live" }));

    expect(screen.queryByTestId("terminal-failure-overlay")).toBeNull();
  });

  test("an abnormal exit shows the overlay with a reason and resume/dismiss actions", async () => {
    renderPane();
    await waitFor(() => expect(channelListener).not.toBeNull());

    act(() => channelListener?.({ kind: "exit", value: "crashed" }));

    await waitFor(() => expect(screen.queryByTestId("terminal-failure-overlay")).not.toBeNull());
    expect(screen.getByText(/crashed/i)).toBeTruthy();
    expect(screen.getByRole("button", { name: /resume/i })).toBeTruthy();
    expect(screen.getByRole("button", { name: /dismiss/i })).toBeTruthy();
  });

  test("a clean exit (ok) does not show the overlay but renders the exit bar", async () => {
    renderPane();
    await waitFor(() => expect(channelListener).not.toBeNull());

    act(() => channelListener?.({ kind: "exit", value: "ok" }));

    expect(screen.queryByTestId("terminal-failure-overlay")).toBeNull();
    const exitBar = await screen.findByTestId("terminal-exit-bar");
    expect(exitBar).toBeTruthy();
    expect(exitBar.textContent).toContain("exited");

    // Clicking Restart clears status/exits and triggers reconnect
    act(() => screen.getByTestId("terminal-exit-restart").click());
    expect(screen.getByText("connecting")).toBeTruthy();
    expect(screen.queryByTestId("terminal-exit-bar")).toBeNull();
  });

  test("dismiss hides the overlay and triggers requestReset", async () => {
    renderPane();
    await waitFor(() => expect(channelListener).not.toBeNull());

    act(() => channelListener?.({ kind: "error", value: "spawn failed" }));
    await waitFor(() => expect(screen.queryByTestId("terminal-failure-overlay")).not.toBeNull());

    act(() => screen.getByRole("button", { name: /dismiss/i }).click());

    await waitFor(() => expect(screen.queryByTestId("terminal-failure-overlay")).toBeNull());
    expect(onRequestResetSpy).toHaveBeenCalled();
  });

  test("manual reconnect clears terminal buffer and re-resolves channel", async () => {
    let clearCalled = false;
    FakeTerminal.prototype.clear = () => {
      clearCalled = true;
    };

    renderPane();
    await waitFor(() => expect(channelListener).not.toBeNull());

    act(() => channelListener?.({ kind: "status", value: "live" }));
    expect(screen.getByText("live")).toBeTruthy();

    const reconnectBtn = screen.getByTestId("terminal-status-reconnect");
    expect(reconnectBtn).toBeTruthy();

    // Click reconnect
    act(() => reconnectBtn.click());

    expect(clearCalled).toBe(true);
    expect(screen.getByText("connecting")).toBeTruthy();
    expect(closeSpy).toHaveBeenCalled();
  });
});
