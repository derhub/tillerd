import { renderHook } from "@testing-library/react";
/// <reference lib="dom" />
import { describe, expect, mock, test } from "bun:test";

import type { PanelLeaf, PanelNode } from "~/lib/panelTree";

import { ACTION } from "~/lib/commands/ids";

import { useShellCommands } from "./useShellCommands";

function leaf(id: string, content: PanelLeaf["content"]): PanelLeaf {
  return { kind: "panel", id, title: id, content };
}

function ref<T>(value: T) {
  return { current: value };
}

function setup(tree: PanelNode, activeId: string | null) {
  const deps = {
    treeRef: ref(tree),
    activeLeafRef: ref<string | null>(activeId),
    detachedRef: ref(new Set<string>()),
    split: mock((_id: string, _d: "horizontal" | "vertical") => "new-leaf"),
    spawn: mock(() => {}),
    close: mock(() => {}),
    detach: mock(() => {}),
    setFocusedLeaf: mock(() => {}),
    toggleZoom: mock(() => {}),
  };
  const { result } = renderHook(() => useShellCommands(deps));
  return { handlers: result.current, deps };
}

describe("useShellCommands", () => {
  test("closing the only terminal is not blocked and routes through close", () => {
    const only = leaf("t1", { type: "terminal", placement: "p1" });
    const { handlers, deps } = setup(only, "t1");
    handlers[ACTION.surfaceClose]?.();
    expect(deps.close).toHaveBeenCalledTimes(1);
    expect(deps.close.mock.calls[0]?.[0]).toBe(only);
  });

  test("new surface spawns into an empty leaf without splitting", () => {
    const empty = leaf("e1", { type: "empty" });
    const { handlers, deps } = setup(empty, "e1");
    handlers[ACTION.surfaceNew]?.();
    expect(deps.spawn).toHaveBeenCalledWith("e1");
    expect(deps.split).not.toHaveBeenCalled();
  });

  test("new surface splits when no empty leaf exists", () => {
    const term = leaf("t1", { type: "terminal", placement: "p1" });
    const { handlers, deps } = setup(term, "t1");
    handlers[ACTION.surfaceNew]?.();
    expect(deps.split).toHaveBeenCalledWith("t1", "horizontal");
    expect(deps.spawn).toHaveBeenCalledWith("new-leaf");
  });

  test("zoom toggle acts on the active leaf", () => {
    const term = leaf("t1", { type: "terminal", placement: "p1" });
    const { handlers, deps } = setup(term, "t1");
    handlers[ACTION.paneZoomToggle]?.();
    expect(deps.toggleZoom).toHaveBeenCalledWith("t1");
  });
});
