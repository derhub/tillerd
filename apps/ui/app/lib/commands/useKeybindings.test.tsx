import { cleanup, render } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";

import { CommandRegistryProvider, RegisterCommands, type Command } from "./registry";
import { useGlobalShortcuts } from "./useKeybindings";

afterEach(cleanup);

function Harness({ bindings, commands }: { bindings: Map<string, string>; commands: Command[] }) {
  useGlobalShortcuts(bindings);
  return (
    <>
      <RegisterCommands commands={commands} />
    </>
  );
}

function mount(onRun: () => void) {
  render(
    <CommandRegistryProvider>
      <Harness
        bindings={new Map([["x", "CmdOrCtrl+T"]])}
        commands={[{ id: "x", title: "X", run: onRun }]}
      />
    </CommandRegistryProvider>,
  );
}

describe("useGlobalShortcuts", () => {
  test("runs the bound action when no editable surface holds focus", () => {
    let ran = false;
    mount(() => (ran = true));
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "t", metaKey: true, bubbles: true }));
    expect(ran).toBe(true);
  });

  test("ignores the key while a terminal surface holds focus", () => {
    let ran = false;
    mount(() => (ran = true));
    const term = document.createElement("div");
    term.className = "xterm";
    document.body.appendChild(term);
    term.dispatchEvent(new KeyboardEvent("keydown", { key: "t", metaKey: true, bubbles: true }));
    document.body.removeChild(term);
    expect(ran).toBe(false);
  });

  test("ignores an unbound chord", () => {
    let ran = false;
    mount(() => (ran = true));
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "q", metaKey: true, bubbles: true }));
    expect(ran).toBe(false);
  });
});
