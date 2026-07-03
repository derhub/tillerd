import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, describe, expect, mock, test } from "bun:test";

import type { CommandHandler } from "~/lib/commands/types";

import { resetContext, setContextKey } from "~/lib/commands/context";
import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, RegisterHandlers } from "~/lib/commands/registry";

// Capture the handler registered via subscribe("commandCenterOpen").listen so openPalette() can
// fire it. The component consumes the client-bindings `subscribe`/`runCommand` wrappers, so the mock
// targets that layer (a sibling suite's partial mock of the same module must not clobber it).
let activateHandler: (() => void) | null = null;

const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  runCommand: async () => null,
  subscribe: () => ({
    listen: (cb: (e: unknown) => void) => {
      activateHandler = () => cb({});
      return Promise.resolve(() => {});
    },
  }),
}));

const { CommandCenter } = await import("./CommandCenter");

afterEach(() => {
  cleanup();
  resetContext();
});
afterAll(() => mock.restore());

function renderWith(handlers: Record<string, CommandHandler>) {
  // The panel/surface commands are gated on an active session.
  setContextKey("hasActiveSession", true);
  return render(
    <CommandRegistryProvider>
      <RegisterHandlers handlers={handlers} />
      <CommandCenter />
    </CommandRegistryProvider>,
  );
}

function openPalette() {
  activateHandler?.();
}

// cmdk fuzzy filtering requires layout happy-dom does not provide; filter behaviour is in desktop e2e (WKWebView).
describe("CommandCenter", () => {
  test("is closed until the open signal fires", () => {
    renderWith({ [ACTION.surfaceClose]: () => {} });
    expect(screen.queryByTestId("command-center")).toBeNull();
  });

  test("opens and lists commands by their definition titles", async () => {
    renderWith({ [ACTION.surfaceClose]: () => {}, [ACTION.panelSplitH]: () => {} });
    openPalette();
    await waitFor(() => expect(screen.queryByTestId("command-center")).not.toBeNull());
    expect(screen.queryByText("Close panel")).not.toBeNull();
    expect(screen.queryByText("Split panel right")).not.toBeNull();
  });

  test("shows the resolved key hint for a bound action", async () => {
    renderWith({ [ACTION.surfaceClose]: () => {} });
    openPalette();
    // Default preset binds surface.close to CmdOrCtrl+W; hint ends in W.
    await waitFor(() => expect(screen.queryByText(/W$/)).not.toBeNull());
  });

  test("selecting a command runs it and closes the palette", async () => {
    let ran = false;
    renderWith({ [ACTION.surfaceClose]: () => (ran = true) });
    openPalette();
    const item = await screen.findByText("Close panel");
    fireEvent.click(item);
    expect(ran).toBe(true);
    await waitFor(() => expect(screen.queryByTestId("command-center")).toBeNull());
  });

  test("Escape closes without running anything", async () => {
    let ran = false;
    renderWith({ [ACTION.surfaceClose]: () => (ran = true) });
    openPalette();
    const box = await screen.findByTestId("command-center");
    fireEvent.keyDown(box, { key: "Escape" });
    await waitFor(() => expect(screen.queryByTestId("command-center")).toBeNull());
    expect(ran).toBe(false);
  });

  test("omits commands whose context is not satisfied", async () => {
    // surface.detach is gated on an active session; clear the flag after render.
    renderWith({ [ACTION.surfaceDetach]: () => {} });
    resetContext();
    openPalette();
    await waitFor(() => expect(screen.queryByTestId("command-center")).not.toBeNull());
    expect(screen.queryByText("Detach panel")).toBeNull();
  });
});
