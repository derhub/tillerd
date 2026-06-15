/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";

import { CommandCenter } from "./CommandCenter";
import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, RegisterCommands, type Command } from "~/lib/commands/registry";
import { COMMAND_CENTER_OPEN_EVENT } from "~/lib/transport/leader-source";

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
  fireEvent(window, new CustomEvent(COMMAND_CENTER_OPEN_EVENT));
}

// cmdk's fuzzy filtering depends on layout that happy-dom does not provide; the query-filter
// behaviour is asserted in the desktop e2e (real WKWebView). These cover open/list/run/dismiss.
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
    // The default preset binds surface.close to CmdOrCtrl+W -> the hint ends in W.
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
