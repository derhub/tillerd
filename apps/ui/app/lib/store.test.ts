/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";

import { setActiveWorkspace, uiStore } from "./store";

afterEach(() => setActiveWorkspace(null));

describe("uiStore activeWorkspaceId", () => {
  test("starts unscoped (all workspaces)", () => {
    expect(uiStore.state.activeWorkspaceId).toBeNull();
  });

  test("setActiveWorkspace selects a workspace", () => {
    setActiveWorkspace("ws-1");
    expect(uiStore.state.activeWorkspaceId).toBe("ws-1");
  });

  test("setActiveWorkspace(null) returns to the unscoped view", () => {
    setActiveWorkspace("ws-1");
    setActiveWorkspace(null);
    expect(uiStore.state.activeWorkspaceId).toBeNull();
  });

  test("a subscriber sees the new selection and stops after unsubscribe", () => {
    const seen: (string | null)[] = [];
    const sub = uiStore.subscribe(() => seen.push(uiStore.state.activeWorkspaceId));

    setActiveWorkspace("ws-2");
    sub.unsubscribe();
    setActiveWorkspace("ws-3");

    expect(seen).toContain("ws-2");
    expect(seen).not.toContain("ws-3");
  });
});
