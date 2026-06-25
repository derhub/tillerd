import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";

import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, RegisterCommands, type Command } from "~/lib/commands/registry";

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
