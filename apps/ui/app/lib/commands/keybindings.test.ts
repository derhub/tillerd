/// <reference lib="dom" />
import { describe, expect, test } from "bun:test";

import { COMMAND_DEFS_BY_ID } from "./defs";
import { ACTION } from "./ids";
import {
  DEFAULT_PRESET,
  PRESETS,
  PRESET_NAMES,
  canonicalize,
  displayAccelerator,
  eventToAccelerator,
  formatAccelerator,
  parseAccelerator,
  resolveBindings,
} from "./keybindings";
import { surfacesOf } from "./types";

describe("parseAccelerator", () => {
  test("parses modifiers and an uppercased key", () => {
    expect(parseAccelerator("cmdorctrl+shift+n")).toEqual({
      cmdOrCtrl: true,
      alt: false,
      shift: true,
      key: "N",
    });
  });

  test("accepts a named key verbatim", () => {
    expect(parseAccelerator("CmdOrCtrl+Enter")?.key).toBe("Enter");
  });

  test("rejects a chord with no non-modifier key", () => {
    expect(parseAccelerator("CmdOrCtrl+Shift")).toBeNull();
  });
});

describe("formatAccelerator", () => {
  test("renders modifiers in canonical order then the key", () => {
    expect(formatAccelerator({ cmdOrCtrl: true, alt: true, shift: true, key: "P" })).toBe(
      "CmdOrCtrl+Alt+Shift+P",
    );
  });
});

describe("canonicalize", () => {
  test("round-trips a messy chord to canonical form", () => {
    expect(canonicalize("shift+cmdorctrl+p")).toBe("CmdOrCtrl+Shift+P");
  });

  test("returns null for an unparseable chord", () => {
    expect(canonicalize("Shift")).toBeNull();
  });
});

describe("eventToAccelerator", () => {
  test("maps a modified keydown to the canonical accelerator", () => {
    const e = new KeyboardEvent("keydown", { key: "n", metaKey: true, shiftKey: true });
    expect(eventToAccelerator(e)).toBe("CmdOrCtrl+Shift+N");
  });

  test("treats ctrl as CmdOrCtrl", () => {
    const e = new KeyboardEvent("keydown", { key: "t", ctrlKey: true });
    expect(eventToAccelerator(e)).toBe("CmdOrCtrl+T");
  });

  test("returns null for a modifier-only keydown", () => {
    const e = new KeyboardEvent("keydown", { key: "Shift", shiftKey: true });
    expect(eventToAccelerator(e)).toBeNull();
  });
});

describe("displayAccelerator", () => {
  test("uses mac glyphs without separators", () => {
    expect(displayAccelerator("CmdOrCtrl+Shift+N", true)).toBe("⌘⇧N");
  });

  test("uses words joined by plus off mac", () => {
    expect(displayAccelerator("CmdOrCtrl+Shift+N", false)).toBe("Ctrl+Shift+N");
  });
});

describe("presets", () => {
  test("default is among the preset names and is fully populated for palette-eligible actions", () => {
    expect(PRESET_NAMES).toContain(DEFAULT_PRESET);
    const def = PRESETS[DEFAULT_PRESET];
    for (const id of Object.values(ACTION)) {
      const commandDef = COMMAND_DEFS_BY_ID.get(id);
      // Row-scoped context-menu actions (rename, archive, delete, ...) are never
      // reachable from the palette or a keybinding -- only the entity's own
      // context menu invokes them, with the row's id as an argument.
      if (!commandDef || !surfacesOf(commandDef).includes("palette")) continue;
      expect(canonicalize(def[id] ?? "")).not.toBeNull();
    }
  });

  test("every preset binding is a canonical accelerator", () => {
    for (const name of PRESET_NAMES) {
      for (const accel of Object.values(PRESETS[name])) {
        expect(canonicalize(accel)).toBe(accel);
      }
    }
  });
});

describe("resolveBindings", () => {
  test("uses the preset baseline when there are no overrides", () => {
    const resolved = resolveBindings(DEFAULT_PRESET, {});
    expect(resolved.get(ACTION.surfaceSpawn)).toBe(PRESETS[DEFAULT_PRESET][ACTION.surfaceSpawn]);
  });

  test("an override wins over the preset for that action only", () => {
    const resolved = resolveBindings(DEFAULT_PRESET, {
      [ACTION.surfaceClose]: "CmdOrCtrl+Shift+W",
    });
    expect(resolved.get(ACTION.surfaceClose)).toBe("CmdOrCtrl+Shift+W");
    expect(resolved.get(ACTION.surfaceSpawn)).toBe(PRESETS[DEFAULT_PRESET][ACTION.surfaceSpawn]);
  });

  test("clearing an override (absence) falls back to the preset", () => {
    const resolved = resolveBindings(DEFAULT_PRESET, {});
    expect(resolved.get(ACTION.surfaceClose)).toBe(PRESETS[DEFAULT_PRESET][ACTION.surfaceClose]);
  });

  test("an action absent from preset and overrides has no binding", () => {
    const resolved = resolveBindings("vim", {});
    const vimHasSettings = ACTION.appSettings in PRESETS.vim;
    expect(resolved.has(ACTION.appSettings)).toBe(vimHasSettings);
  });

  test("switching presets keeps overrides and re-bases the rest", () => {
    const overrides = { [ACTION.surfaceClose]: "CmdOrCtrl+Shift+W" };
    const onDefault = resolveBindings(DEFAULT_PRESET, overrides);
    const onVscode = resolveBindings("vscode", overrides);
    expect(onVscode.get(ACTION.surfaceClose)).toBe("CmdOrCtrl+Shift+W");
    expect(onVscode.get(ACTION.surfaceSpawn)).toBe(PRESETS.vscode[ACTION.surfaceSpawn]);
    expect(onVscode.get(ACTION.surfaceSpawn)).not.toBe(onDefault.get(ACTION.surfaceSpawn));
  });
});
