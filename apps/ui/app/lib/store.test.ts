/// <reference lib="dom" />
import { afterEach, beforeEach, describe, expect, test } from "bun:test";

import { _resetForTests, settingsStore } from "./settings/context";
import { sidebarExpandedKey, VIEW_ACTIVE_WORKSPACE_KEY } from "./settings/keys";
import {
  resetUiStore,
  setActiveProject,
  setActiveWorkspace,
  setProjectExpanded,
  uiStore,
} from "./store";

// Pre-hydration writes buffer inside the settings bootstrap (no transport call),
// so these tests exercise the pointer wiring without a mocked backend.
beforeEach(() => {
  _resetForTests();
  settingsStore.setState(() => ({ values: {} }));
  resetUiStore();
});

afterEach(() => {
  resetUiStore();
  settingsStore.setState(() => ({ values: {} }));
  _resetForTests();
});

describe("active-workspace view pointer", () => {
  test("starts unscoped (all workspaces)", () => {
    expect(settingsStore.state.values[VIEW_ACTIVE_WORKSPACE_KEY]).toBeUndefined();
  });

  test("setActiveWorkspace writes the settings-store pointer", () => {
    setActiveWorkspace("ws-1");
    expect(settingsStore.state.values[VIEW_ACTIVE_WORKSPACE_KEY]).toBe("ws-1");
  });

  test("setActiveWorkspace(null) returns to the unscoped view", () => {
    setActiveWorkspace("ws-1");
    setActiveWorkspace(null);
    expect(settingsStore.state.values[VIEW_ACTIVE_WORKSPACE_KEY]).toBeNull();
  });

  test("a subscriber sees the new selection and stops after unsubscribe", () => {
    const seen: unknown[] = [];
    const sub = settingsStore.subscribe(() =>
      seen.push(settingsStore.state.values[VIEW_ACTIVE_WORKSPACE_KEY]),
    );

    setActiveWorkspace("ws-2");
    sub.unsubscribe();
    setActiveWorkspace("ws-3");

    expect(seen).toContain("ws-2");
    expect(seen).not.toContain("ws-3");
  });

  test("view pointers no longer persist to webview localStorage", () => {
    localStorage.clear();
    setActiveWorkspace("ws-persist-test");
    expect(localStorage.getItem("tillerd:ui-state")).toBeNull();
  });
});

describe("sidebar-expanded view pointer", () => {
  test("setProjectExpanded writes a per-project settings key", () => {
    setProjectExpanded("p-1", true);
    expect(settingsStore.state.values[sidebarExpandedKey("p-1")]).toBe(true);
    setProjectExpanded("p-1", false);
    expect(settingsStore.state.values[sidebarExpandedKey("p-1")]).toBe(false);
  });

  test("setActiveProject expands the project and stays window-local", () => {
    setActiveProject("p-2");
    expect(uiStore.state.activeProjectId).toBe("p-2");
    expect(settingsStore.state.values[sidebarExpandedKey("p-2")]).toBe(true);
  });
});
