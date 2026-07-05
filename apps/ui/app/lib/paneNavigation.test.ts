import { describe, expect, test } from "bun:test";

import { nearestLeafInDirection, type LeafRect } from "./paneNavigation";

// A 2x2 grid of 100x100 panes:
//   A(0,0) B(100,0)
//   C(0,100) D(100,100)
const grid: LeafRect[] = [
  { id: "A", left: 0, right: 100, top: 0, bottom: 100 },
  { id: "B", left: 100, right: 200, top: 0, bottom: 100 },
  { id: "C", left: 0, right: 100, top: 100, bottom: 200 },
  { id: "D", left: 100, right: 200, top: 100, bottom: 200 },
];

describe("nearestLeafInDirection", () => {
  test("moves right to the aligned neighbor", () => {
    expect(nearestLeafInDirection("A", "right", grid)).toBe("B");
  });

  test("moves down to the aligned neighbor", () => {
    expect(nearestLeafInDirection("A", "down", grid)).toBe("C");
  });

  test("moves left from the right column", () => {
    expect(nearestLeafInDirection("B", "left", grid)).toBe("A");
  });

  test("moves up from the bottom row", () => {
    expect(nearestLeafInDirection("C", "up", grid)).toBe("A");
  });

  test("no neighbor in the direction returns null", () => {
    expect(nearestLeafInDirection("A", "left", grid)).toBeNull();
    expect(nearestLeafInDirection("A", "up", grid)).toBeNull();
  });

  test("unknown focused id returns null", () => {
    expect(nearestLeafInDirection("Z", "right", grid)).toBeNull();
  });

  test("prefers the overlapping pane over a closer diagonal one", () => {
    // Focused pane on the left; two candidates to the right: one aligned (overlaps rows),
    // one diagonally above with a slightly closer center. Overlap must win.
    const rects: LeafRect[] = [
      { id: "focus", left: 0, right: 100, top: 100, bottom: 200 },
      { id: "aligned", left: 110, right: 210, top: 100, bottom: 200 },
      { id: "diagonal", left: 105, right: 205, top: 0, bottom: 90 },
    ];
    expect(nearestLeafInDirection("focus", "right", rects)).toBe("aligned");
  });

  test("single pane has no neighbor", () => {
    const one: LeafRect[] = [{ id: "solo", left: 0, right: 100, top: 0, bottom: 100 }];
    expect(nearestLeafInDirection("solo", "right", one)).toBeNull();
  });
});
