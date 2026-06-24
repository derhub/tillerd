import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";

import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, RegisterCommands, type Command } from "~/lib/commands/registry";

// Capture the handler registered via events.commandCenterOpen.listen so openPalette() can fire it.
let activateHandler: (() => void) | null = null;

void mock.module("@tauri-apps/api/event", () => ({
  listen: (_event: string, cb: (e: unknown) => void) => {
    activateHandler = () => cb({});
    return Promise.resolve(() => {});
  },
}));

void mock.module("@tauri-apps/api/core", () => ({
  invoke: async () => null,
  Channel: class Channel {
    onmessage: ((v: unknown) => void) | null = null;
  },
}));

const { CommandCenter } = await import("./CommandCenter");

afterEach(cleanup);

function renderWith(commands: Command[]) {
  return render(
    <CommandRegistryProvider>
      <RegisterCommands commands={commands} />
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
    renderWith([{ id: ACTION.surfaceClose, title: "Close surface", run: () => {} }]);
    expect(screen.queryByTestId("command-center")).toBeNull();
  });

  test("opens and lists registered commands", async () => {
    renderWith([
      { id: ACTION.surfaceClose, title: "Close surface", run: () => {} },
      { id: ACTION.panelSplitH, title: "Split right", run: () => {} },
    ]);
    openPalette();
    await waitFor(() => expect(screen.queryByTestId("command-center")).not.toBeNull());
    expect(screen.queryByText("Close surface")).not.toBeNull();
    expect(screen.queryByText("Split right")).not.toBeNull();
  });

  test("shows the resolved key hint for a bound action", async () => {
    renderWith([{ id: ACTION.surfaceClose, title: "Close surface", run: () => {} }]);
    openPalette();
    // Default preset binds surface.close to CmdOrCtrl+W; hint ends in W.
    await waitFor(() => expect(screen.queryByText(/W$/)).not.toBeNull());
  });

  test("selecting a command runs it and closes the palette", async () => {
    let ran = false;
    renderWith([{ id: ACTION.surfaceClose, title: "Close surface", run: () => (ran = true) }]);
    openPalette();
    const item = await screen.findByText("Close surface");
    fireEvent.click(item);
    expect(ran).toBe(true);
    await waitFor(() => expect(screen.queryByTestId("command-center")).toBeNull());
  });

  test("Escape closes without running anything", async () => {
    let ran = false;
    renderWith([{ id: ACTION.surfaceClose, title: "Close surface", run: () => (ran = true) }]);
    openPalette();
    const box = await screen.findByTestId("command-center");
    fireEvent.keyDown(box, { key: "Escape" });
    await waitFor(() => expect(screen.queryByTestId("command-center")).toBeNull());
    expect(ran).toBe(false);
  });
});
