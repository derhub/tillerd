import type { SnapshotFrame } from "./messages";

// SnapshotCell fg/bg/attrs encoding. This is a language-neutral WIRE CONTRACT —
// any-language daemon that emits snapshot frames must produce cells in exactly
// this encoding. Canonical definition: the "Snapshot frame cell encoding"
// requirement in openspec/specs/pty-daemon/spec.md. Keep the two in sync.
export const ATTR_BOLD      = 0x01;
export const ATTR_DIM       = 0x02;
export const ATTR_ITALIC    = 0x04;
export const ATTR_UNDERLINE = 0x08;
export const ATTR_BLINK     = 0x10;
export const ATTR_INVERSE   = 0x20;
export const ATTR_INVISIBLE = 0x40;

// SnapshotCell color encoding:
//   0          = default
//   1–8        = ANSI standard (30–37 → 1–8)
//   9–16       = ANSI bright (90–97 → 9–16)
//   17–272     = 256-color (index + 17)
//   0x1000000+ = 24-bit RGB (0x1000000 | r<<16 | g<<8 | b)
export const COLOR_DEFAULT = 0;

function colorToFgSGR(c: number): string {
  if (c === COLOR_DEFAULT) return "39";
  if (c >= 1 && c <= 8) return String(29 + c);
  if (c >= 9 && c <= 16) return String(81 + c);
  if (c >= 17 && c <= 272) return `38;5;${c - 17}`;
  const r = (c >> 16) & 0xff; const g = (c >> 8) & 0xff; const b = c & 0xff;
  return `38;2;${r};${g};${b}`;
}

function colorToBgSGR(c: number): string {
  if (c === COLOR_DEFAULT) return "49";
  if (c >= 1 && c <= 8) return String(39 + c);
  if (c >= 9 && c <= 16) return String(91 + c);
  if (c >= 17 && c <= 272) return `48;5;${c - 17}`;
  const r = (c >> 16) & 0xff; const g = (c >> 8) & 0xff; const b = c & 0xff;
  return `48;2;${r};${g};${b}`;
}

export function charDisplayWidth(cp: number): 1 | 2 {
  if (
    (cp >= 0x1100 && cp <= 0x115f) ||
    (cp >= 0x2e80 && cp <= 0x303f) ||
    (cp >= 0x3040 && cp <= 0x33ff) ||
    (cp >= 0x3400 && cp <= 0x4dbf) ||
    (cp >= 0x4e00 && cp <= 0x9fff) ||
    (cp >= 0xa960 && cp <= 0xa97f) ||
    (cp >= 0xac00 && cp <= 0xd7ff) ||
    (cp >= 0xf900 && cp <= 0xfaff) ||
    (cp >= 0xfe10 && cp <= 0xfe1f) ||
    (cp >= 0xfe30 && cp <= 0xfe4f) ||
    (cp >= 0xfe50 && cp <= 0xfe6f) ||
    (cp >= 0xff01 && cp <= 0xff60) ||
    (cp >= 0xffe0 && cp <= 0xffe6) ||
    (cp >= 0x1b000 && cp <= 0x1bfff) ||
    (cp >= 0x1c000 && cp <= 0x1cfff) ||
    (cp >= 0x20000 && cp <= 0x2fffd) ||
    (cp >= 0x30000 && cp <= 0x3fffd)
  ) return 2;
  return 1;
}

// Convert a snapshot frame's cell grid to ANSI escape-sequence bytes.
// Every cell is absolutely positioned, so column width is used only to skip
// wide-char continuation cells, never to compute cursor position.
export function snapshotToBytes(
  frame: Pick<SnapshotFrame, "rows" | "cols" | "cells" | "cursor">,
): Uint8Array {
  const parts: string[] = [];

  parts.push("\x1b[2J\x1b[H");

  let curFg = COLOR_DEFAULT;
  let curBg = COLOR_DEFAULT;
  let curAttrs = 0;

  for (let row = 0; row < frame.rows; row++) {
    for (let col = 0; col < frame.cols; col++) {
      const cell = frame.cells[row]?.[col];
      if (!cell) continue;

      // Skip wide-char continuation cells
      if (cell.char === "") continue;

      // Skip default empty spaces (common case — saves bytes)
      if (cell.char === " " && cell.fg === COLOR_DEFAULT && cell.bg === COLOR_DEFAULT && cell.attrs === 0) continue;

      parts.push(`\x1b[${row + 1};${col + 1}H`);

      if (cell.fg !== curFg || cell.bg !== curBg || cell.attrs !== curAttrs) {
        const sgr: string[] = [];
        // If any active attr was cleared, reset and re-apply from scratch.
        if ((curAttrs & ~cell.attrs) !== 0) {
          sgr.push("0");
          curAttrs = 0;
          curFg = COLOR_DEFAULT;
          curBg = COLOR_DEFAULT;
        }
        if (cell.attrs & ATTR_BOLD && !(curAttrs & ATTR_BOLD)) sgr.push("1");
        if (cell.attrs & ATTR_DIM && !(curAttrs & ATTR_DIM)) sgr.push("2");
        if (cell.attrs & ATTR_ITALIC && !(curAttrs & ATTR_ITALIC)) sgr.push("3");
        if (cell.attrs & ATTR_UNDERLINE && !(curAttrs & ATTR_UNDERLINE)) sgr.push("4");
        if (cell.attrs & ATTR_BLINK && !(curAttrs & ATTR_BLINK)) sgr.push("5");
        if (cell.attrs & ATTR_INVERSE && !(curAttrs & ATTR_INVERSE)) sgr.push("7");
        if (cell.attrs & ATTR_INVISIBLE && !(curAttrs & ATTR_INVISIBLE)) sgr.push("8");
        if (cell.fg !== curFg) sgr.push(colorToFgSGR(cell.fg));
        if (cell.bg !== curBg) sgr.push(colorToBgSGR(cell.bg));
        if (sgr.length > 0) parts.push(`\x1b[${sgr.join(";")}m`);
        curFg = cell.fg; curBg = cell.bg; curAttrs = cell.attrs;
      }

      parts.push(cell.char);
    }
  }

  if (curFg !== COLOR_DEFAULT || curBg !== COLOR_DEFAULT || curAttrs !== 0) {
    parts.push("\x1b[m");
  }

  parts.push(`\x1b[${frame.cursor.y + 1};${frame.cursor.x + 1}H`);

  return new TextEncoder().encode(parts.join(""));
}
