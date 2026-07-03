import { cleanup, render } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";

import type { CommandHandler } from "./types";

import { resetContext } from "./context";
import { ACTION } from "./ids";
import { CommandRegistryProvider, RegisterHandlers } from "./registry";
import { useGlobalShortcuts } from "./useKeybindings";

afterEach(() => {
  cleanup();
  resetContext();
});

function Harness({
  bindings,
  handlers,
}: {
  bindings: Map<string, string>;
  handlers: Record<string, CommandHandler>;
}) {
  useGlobalShortcuts(bindings);
  return <RegisterHandlers handlers={handlers} />;
}

// session.new is bound to CmdOrCtrl+N and has a registered handler.
function mount(onRun: () => void) {
  render(
    <CommandRegistryProvider>
      <Harness
        bindings={new Map([[ACTION.sessionNew, "CmdOrCtrl+N"]])}
        handlers={{ [ACTION.sessionNew]: onRun }}
      />
    </CommandRegistryProvider>,
  );
}

describe("useGlobalShortcuts", () => {
  test("runs the bound action when no editable surface holds focus", () => {
    let ran = false;
    mount(() => (ran = true));
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "n", metaKey: true, bubbles: true }));
    expect(ran).toBe(true);
  });

  test("ignores the key while a terminal surface holds focus", () => {
    let ran = false;
    mount(() => (ran = true));
    const term = document.createElement("div");
    term.className = "xterm";
    document.body.appendChild(term);
    term.dispatchEvent(new KeyboardEvent("keydown", { key: "n", metaKey: true, bubbles: true }));
    document.body.removeChild(term);
    expect(ran).toBe(false);
  });

  test("ignores an unbound chord", () => {
    let ran = false;
    mount(() => (ran = true));
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "q", metaKey: true, bubbles: true }));
    expect(ran).toBe(false);
  });

  test("lets the keystroke through when the bound command has no registered handler", () => {
    render(
      <CommandRegistryProvider>
        {/* Bind a key to session.new but register no handler, so the command is inactive. */}
        <Harness bindings={new Map([[ACTION.sessionNew, "CmdOrCtrl+N"]])} handlers={{}} />
      </CommandRegistryProvider>,
    );
    const event = new KeyboardEvent("keydown", {
      key: "n",
      metaKey: true,
      bubbles: true,
      cancelable: true,
    });
    window.dispatchEvent(event);
    // No active command => the binding does not swallow the keystroke.
    expect(event.defaultPrevented).toBe(false);
  });
});
