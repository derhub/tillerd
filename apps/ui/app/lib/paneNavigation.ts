// Directional pane navigation (panel-multiplexer-nav spec): given the on-screen rectangles of the
// panel leaves and the currently focused leaf, pick the nearest leaf in a requested direction. Pure
// and DOM-agnostic -- the caller reads `[data-panel-id]` rects and passes them in -- so the geometry
// heuristic is unit-testable without a layout engine.

export type Direction = "left" | "right" | "up" | "down";

export interface LeafRect {
  id: string;
  // A DOMRect-like box; only these four edges are read.
  left: number;
  right: number;
  top: number;
  bottom: number;
}

interface Center {
  x: number;
  y: number;
}

function center(r: LeafRect): Center {
  return { x: (r.left + r.right) / 2, y: (r.top + r.bottom) / 2 };
}

// True when `cand` lies in `dir` from `from` -- tested by the candidate center being strictly past
// the source center on the direction's axis.
function isInDirection(from: Center, cand: Center, dir: Direction): boolean {
  switch (dir) {
    case "left":
      return cand.x < from.x;
    case "right":
      return cand.x > from.x;
    case "up":
      return cand.y < from.y;
    case "down":
      return cand.y > from.y;
  }
}

// Overlap of the two rects along the axis perpendicular to travel. A candidate that lines up with
// the source (shares rows when moving horizontally, or columns when moving vertically) is preferred
// over one that is diagonally off, matching what the user perceives as "the pane to the right".
function crossOverlap(from: LeafRect, cand: LeafRect, dir: Direction): number {
  if (dir === "left" || dir === "right") {
    return Math.min(from.bottom, cand.bottom) - Math.max(from.top, cand.top);
  }
  return Math.min(from.right, cand.right) - Math.max(from.left, cand.left);
}

// Primary-axis gap between the source and candidate centers -- the travel distance.
function axisDistance(from: Center, cand: Center, dir: Direction): number {
  if (dir === "left" || dir === "right") return Math.abs(cand.x - from.x);
  return Math.abs(cand.y - from.y);
}

// Returns the id of the nearest leaf in `dir` from `focusedId`, or null when none exists there.
// Selection: among candidates whose center is in the direction, prefer the one with the greatest
// cross-axis overlap; break ties by the smallest primary-axis distance. Overlap dominates so a
// perfectly-aligned neighbor always wins over a closer-but-diagonal one.
export function nearestLeafInDirection(
  focusedId: string,
  dir: Direction,
  rects: readonly LeafRect[],
): string | null {
  const from = rects.find((r) => r.id === focusedId);
  if (!from) return null;
  const fromCenter = center(from);

  let best: { id: string; overlap: number; dist: number } | null = null;
  for (const cand of rects) {
    if (cand.id === focusedId) continue;
    if (!isInDirection(fromCenter, center(cand), dir)) continue;
    const overlap = crossOverlap(from, cand, dir);
    const dist = axisDistance(fromCenter, center(cand), dir);
    if (best === null || overlap > best.overlap || (overlap === best.overlap && dist < best.dist)) {
      best = { id: cand.id, overlap, dist };
    }
  }
  return best?.id ?? null;
}
