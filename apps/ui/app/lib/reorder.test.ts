import { describe, expect, test } from "bun:test";

import { reorderByDrop } from "./reorder";

describe("reorderByDrop", () => {
  test("moves an item earlier in the list", () => {
    expect(reorderByDrop(["a", "b", "c"], "c", "a")).toEqual(["c", "a", "b"]);
  });

  test("moves an item later in the list", () => {
    expect(reorderByDrop(["a", "b", "c"], "a", "c")).toEqual(["b", "c", "a"]);
  });

  test("is a no-op when source equals target", () => {
    expect(reorderByDrop(["a", "b", "c"], "b", "b")).toEqual(["a", "b", "c"]);
  });

  test("returns the input unchanged when the source is absent (cross-list drop)", () => {
    const ids = ["a", "b", "c"];
    expect(reorderByDrop(ids, "x", "b")).toBe(ids);
  });

  test("returns the input unchanged when the target is absent", () => {
    const ids = ["a", "b", "c"];
    expect(reorderByDrop(ids, "a", "x")).toBe(ids);
  });

  test("does not mutate the input array", () => {
    const ids = ["a", "b", "c"];
    reorderByDrop(ids, "c", "a");
    expect(ids).toEqual(["a", "b", "c"]);
  });
});
