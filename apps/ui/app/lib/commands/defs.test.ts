import { describe, expect, test } from "bun:test";

import { COMMAND_DEFS, COMMAND_DEFS_BY_ID } from "./defs";
import { ACTION, SESSION_SEARCH_ACTION_ID } from "./ids";
import { PRESETS, PRESET_NAMES } from "./keybindings";
import { surfacesOf } from "./types";

describe("command definitions", () => {
  test("every ACTION id and session search has a definition", () => {
    for (const id of Object.values(ACTION)) {
      expect(COMMAND_DEFS_BY_ID.has(id)).toBe(true);
    }
    expect(COMMAND_DEFS_BY_ID.has(SESSION_SEARCH_ACTION_ID)).toBe(true);
  });

  test("ids are unique", () => {
    const ids = COMMAND_DEFS.map((d) => d.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  test("default keys match the current presets exactly", () => {
    // Parity guard: the defs table must reproduce every existing preset binding
    // before PRESETS is deleted in favor of the table.
    for (const preset of PRESET_NAMES) {
      const fromDefs = new Map<string, string>();
      for (const def of COMMAND_DEFS) {
        const accel = def.defaultKeys?.[preset];
        if (accel) fromDefs.set(def.id, accel);
      }
      const fromPresets = new Map(
        Object.entries(PRESETS[preset]).filter(([, a]) => a) as [string, string][],
      );
      expect(Object.fromEntries(fromDefs)).toEqual(Object.fromEntries(fromPresets));
    }
  });

  test("commands default to the palette surface", () => {
    for (const def of COMMAND_DEFS) {
      expect(surfacesOf(def)).toContain("palette");
    }
  });
});

describe("title-bar toggle commands", () => {
  const toggleIds = [
    ACTION.panelToggleLeft,
    ACTION.panelToggleRight,
    ACTION.panelToggleBottom,
    ACTION.commandToggle,
  ];

  test("are on both the titlebar and palette surfaces", () => {
    for (const id of toggleIds) {
      const def = COMMAND_DEFS_BY_ID.get(id);
      expect(surfacesOf(def!)).toEqual(["titlebar", "palette"]);
    }
  });

  test("each has a toggle selector reading its own context key", () => {
    const expectations: Record<string, string> = {
      [ACTION.panelToggleLeft]: "leftPanelVisible",
      [ACTION.panelToggleRight]: "rightPanelVisible",
      [ACTION.panelToggleBottom]: "bottomPanelVisible",
      [ACTION.commandToggle]: "commandPaletteOpen",
    };
    for (const [id, key] of Object.entries(expectations)) {
      const def = COMMAND_DEFS_BY_ID.get(id);
      expect(def!.toggle!({ [key]: true })).toBe(true);
      expect(def!.toggle!({ [key]: false })).toBe(false);
      expect(def!.toggle!({})).toBe(false);
    }
  });
});
