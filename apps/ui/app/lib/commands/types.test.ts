import { describe, expect, test } from "bun:test";

import { isOnSurface, surfacesOf, type CommandDef, type Surface } from "./types";

describe("surface projection", () => {
  test("a command tagged for the activity bar projects onto it and no other surface", () => {
    const def: CommandDef = { id: "a", title: "A", surfaces: ["activitybar"] };
    expect(isOnSurface(def, "activitybar")).toBe(true);
    expect(isOnSurface(def, "palette")).toBe(false);
  });

  test("a command tagged for the status bar projects onto it and no other surface", () => {
    const def: CommandDef = { id: "a", title: "A", surfaces: ["statusbar"] };
    expect(isOnSurface(def, "statusbar")).toBe(true);
    expect(isOnSurface(def, "titlebar")).toBe(false);
  });

  test("an untagged command defaults to the palette only", () => {
    const def: CommandDef = { id: "a", title: "A" };
    const surfaces: readonly Surface[] = surfacesOf(def);
    expect(surfaces).toEqual(["palette"]);
  });
});
