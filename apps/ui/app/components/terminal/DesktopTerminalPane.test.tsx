import type { SurfaceChannelEvent, SurfaceChannelHandle } from "@tillerd/client-bindings";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";
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

void mock.module("~/lib/lazy", () => ({
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
  command: () => ({ mutationFn: async () => null, meta: { invalidates: [] } }),
}));

const { DesktopTerminalPane } = await import("./DesktopTerminalPane");

function renderPane() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <DesktopTerminalPane sessionId="session-1" placement="main" cwd="/tmp" />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  channelListener = null;
  closeSpy.mockClear();
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

  test("a clean exit (ok) does not show the overlay", async () => {
    renderPane();
    await waitFor(() => expect(channelListener).not.toBeNull());

    act(() => channelListener?.({ kind: "exit", value: "ok" }));

    expect(screen.queryByTestId("terminal-failure-overlay")).toBeNull();
  });

  test("dismiss hides the overlay", async () => {
    renderPane();
    await waitFor(() => expect(channelListener).not.toBeNull());

    act(() => channelListener?.({ kind: "error", value: "spawn failed" }));
    await waitFor(() => expect(screen.queryByTestId("terminal-failure-overlay")).not.toBeNull());

    act(() => screen.getByRole("button", { name: /dismiss/i }).click());

    await waitFor(() => expect(screen.queryByTestId("terminal-failure-overlay")).toBeNull());
  });
});
