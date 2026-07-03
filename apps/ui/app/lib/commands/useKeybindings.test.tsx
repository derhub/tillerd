import { cleanup, render } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";

import type { CommandHandler } from "./types";

import { resetContext, setContextKey } from "./context";
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

// session.new has no `when` gate, so it fires regardless of context.
function mountUngated(onRun: () => void) {
  render(
    <CommandRegistryProvider>
      <Harness
        bindings={new Map([[ACTION.sessionNew, "CmdOrCtrl+N"]])}
        handlers={{ [ACTION.sessionNew]: onRun }}
      />
    </CommandRegistryProvider>,
  );
}

// surface.spawn is gated on `hasActiveSession`.
function mountGated(onRun: () => void) {
  render(
    <CommandRegistryProvider>
      <Harness
        bindings={new Map([[ACTION.surfaceSpawn, "CmdOrCtrl+T"]])}
        handlers={{ [ACTION.surfaceSpawn]: onRun }}
      />
    </CommandRegistryProvider>,
  );
}

describe("useGlobalShortcuts", () => {
  test("runs the bound action when no editable surface holds focus", () => {
    let ran = false;
    mountUngated(() => (ran = true));
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "n", metaKey: true, bubbles: true }));
    expect(ran).toBe(true);
  });

  test("ignores the key while a terminal surface holds focus", () => {
    let ran = false;
    mountUngated(() => (ran = true));
    const term = document.createElement("div");
    term.className = "xterm";
    document.body.appendChild(term);
    term.dispatchEvent(new KeyboardEvent("keydown", { key: "n", metaKey: true, bubbles: true }));
    document.body.removeChild(term);
    expect(ran).toBe(false);
  });

  test("ignores an unbound chord", () => {
    let ran = false;
    mountUngated(() => (ran = true));
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "q", metaKey: true, bubbles: true }));
    expect(ran).toBe(false);
  });

  test("does not fire a gated binding out of context", () => {
    let ran = false;
    mountGated(() => (ran = true));
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "t", metaKey: true, bubbles: true }));
    expect(ran).toBe(false);
  });

  test("fires a gated binding once its context is satisfied", () => {
    let ran = false;
    mountGated(() => (ran = true));
    setContextKey("hasActiveSession", true);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "t", metaKey: true, bubbles: true }));
    expect(ran).toBe(true);
  });
});
