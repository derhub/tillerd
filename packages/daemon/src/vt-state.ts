import type { SnapshotCell } from "@athing/sdk";

// Attrs bitmask
const ATTR_BOLD      = 0x01;
const ATTR_DIM       = 0x02;
const ATTR_ITALIC    = 0x04;
const ATTR_UNDERLINE = 0x08;
const ATTR_BLINK     = 0x10;
const ATTR_INVERSE   = 0x20;
const ATTR_INVISIBLE = 0x40;

// Color encoding:
//   0         = default
//   1–8       = ANSI standard colors (ESC[30m → 1, … ESC[37m → 8)
//   9–16      = ANSI bright colors  (ESC[90m → 9, … ESC[97m → 16)
//   17–272    = 256-color (index + 17, so index 0 = 17)
//   0x1000000+= 24-bit RGB (r<<16 | g<<8 | b | 0x1000000)
const COLOR_DEFAULT = 0;

function ansiToColor(n: number, bright: boolean): number {
  return bright ? (n - 90 + 9) : (n - 30 + 1);
}

function color256(idx: number): number {
  return idx + 17;
}

function colorRgb(r: number, g: number, b: number): number {
  return 0x1000000 | (r << 16) | (g << 8) | b;
}

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

function charDisplayWidth(cp: number): 1 | 2 {
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

export interface SnapshotPayload {
  rows: number;
  cols: number;
  cells: SnapshotCell[][];
  cursor: { x: number; y: number };
}

interface Cell {
  char: string;
  fg: number;
  bg: number;
  attrs: number;
  wide: boolean;
}

function emptyCell(): Cell {
  return { char: " ", fg: COLOR_DEFAULT, bg: COLOR_DEFAULT, attrs: 0, wide: false };
}

function makeGrid(rows: number, cols: number): Cell[][] {
  return Array.from({ length: rows }, () =>
    Array.from({ length: cols }, () => emptyCell()),
  );
}

const enum ParserState {
  NORMAL,
  ESC,
  CSI,
  OSC,
  DCS,
}

export class VtState {
  private rows: number;
  private cols: number;
  private grid: Cell[][];
  private altGrid: Cell[][] | null = null;
  private cursor = { x: 0, y: 0 };
  private savedCursor = { x: 0, y: 0 };
  private altCursor = { x: 0, y: 0 };
  private inAltScreen = false;
  private fg = COLOR_DEFAULT;
  private bg = COLOR_DEFAULT;
  private attrs = 0;

  // Parser state
  private parserState: ParserState = ParserState.NORMAL;
  private paramBuf = "";
  private isPrivate = false;
  private interBuf = "";
  private utf8Buf = new Uint8Array(4);
  private utf8Len = 0;
  private utf8Needed = 0;

  constructor(rows: number, cols: number) {
    this.rows = rows;
    this.cols = cols;
    this.grid = makeGrid(rows, cols);
  }

  feed(bytes: Uint8Array): void {
    for (let i = 0; i < bytes.length; i++) {
      const b = bytes[i]!;

      // UTF-8 continuation
      if (this.utf8Needed > 0) {
        if ((b & 0xc0) === 0x80) {
          this.utf8Buf[this.utf8Len++] = b;
          if (this.utf8Len === this.utf8Needed) {
            const str = new TextDecoder().decode(this.utf8Buf.slice(0, this.utf8Len));
            this.utf8Len = 0;
            this.utf8Needed = 0;
            this.processChar(str, str.codePointAt(0) ?? 0x20);
          }
          continue;
        }
        // Invalid continuation — reset and reprocess
        this.utf8Len = 0;
        this.utf8Needed = 0;
      }

      // UTF-8 start byte detection (only in NORMAL state)
      if (this.parserState === ParserState.NORMAL && (b & 0x80) !== 0) {
        if ((b & 0xe0) === 0xc0) { this.utf8Buf[0] = b; this.utf8Len = 1; this.utf8Needed = 2; continue; }
        if ((b & 0xf0) === 0xe0) { this.utf8Buf[0] = b; this.utf8Len = 1; this.utf8Needed = 3; continue; }
        if ((b & 0xf8) === 0xf0) { this.utf8Buf[0] = b; this.utf8Len = 1; this.utf8Needed = 4; continue; }
        continue; // invalid
      }

      this.processByte(b);
    }
  }

  private processByte(b: number): void {
    switch (this.parserState) {
      case ParserState.NORMAL:
        this.processNormal(b);
        break;
      case ParserState.ESC:
        this.processEsc(b);
        break;
      case ParserState.CSI:
        this.processCsi(b);
        break;
      case ParserState.OSC:
      case ParserState.DCS:
        if (b === 0x07 || b === 0x9c) this.parserState = ParserState.NORMAL;
        else if (b === 0x1b) this.parserState = ParserState.ESC; // may be ESC\
        break;
    }
  }

  private processNormal(b: number): void {
    switch (b) {
      case 0x1b: // ESC
        this.parserState = ParserState.ESC;
        break;
      case 0x0d: // CR
        this.cursor.x = 0;
        break;
      case 0x0a: // LF
      case 0x0b: // VT
      case 0x0c: // FF
        this.cursor.y++;
        if (this.cursor.y >= this.rows) {
          this.scrollUp();
          this.cursor.y = this.rows - 1;
        }
        break;
      case 0x08: // BS
        if (this.cursor.x > 0) this.cursor.x--;
        break;
      case 0x09: // HT (tab)
        this.cursor.x = Math.min(this.cols - 1, (Math.floor(this.cursor.x / 8) + 1) * 8);
        break;
      case 0x07: // BEL — ignore
      case 0x00: // NUL — ignore
        break;
      default:
        if (b >= 0x20) {
          this.processChar(String.fromCharCode(b), b);
        }
    }
  }

  private processChar(ch: string, cp: number): void {
    const width = charDisplayWidth(cp);
    const g = this.activeGrid();

    if (this.cursor.x >= this.cols) {
      // Wrap
      this.cursor.x = 0;
      this.cursor.y++;
      if (this.cursor.y >= this.rows) {
        this.scrollUp();
        this.cursor.y = this.rows - 1;
      }
    }

    const cell = g[this.cursor.y]?.[this.cursor.x];
    if (cell) {
      cell.char = ch;
      cell.fg = this.fg;
      cell.bg = this.bg;
      cell.attrs = this.attrs;
      cell.wide = width === 2;
    }

    if (width === 2 && this.cursor.x + 1 < this.cols) {
      const cont = g[this.cursor.y]![this.cursor.x + 1]!;
      cont.char = "";
      cont.fg = this.fg;
      cont.bg = this.bg;
      cont.attrs = this.attrs;
      cont.wide = false;
    }

    this.cursor.x += width;
  }

  private processEsc(b: number): void {
    this.parserState = ParserState.NORMAL;
    switch (b) {
      case 0x5b: // [  → CSI
        this.parserState = ParserState.CSI;
        this.paramBuf = "";
        this.isPrivate = false;
        this.interBuf = "";
        break;
      case 0x5d: // ] → OSC
        this.parserState = ParserState.OSC;
        break;
      case 0x50: // P → DCS
        this.parserState = ParserState.DCS;
        break;
      case 0x37: // 7 → save cursor
        this.saveCursor();
        break;
      case 0x38: // 8 → restore cursor
        this.restoreCursor();
        break;
      case 0x4d: // M → reverse index (scroll down)
        if (this.cursor.y === 0) {
          this.scrollDown();
        } else {
          this.cursor.y--;
        }
        break;
      case 0x63: // c → full reset
        this.fullReset();
        break;
      case 0x5c: // \ → ST (string terminator)
        break;
    }
  }

  private processCsi(b: number): void {
    if (b >= 0x30 && b <= 0x3f) {
      // param/private byte
      if (b === 0x3f) this.isPrivate = true;
      else this.paramBuf += String.fromCharCode(b);
    } else if (b >= 0x20 && b <= 0x2f) {
      this.interBuf += String.fromCharCode(b);
    } else if (b >= 0x40 && b <= 0x7e) {
      // final byte
      this.dispatchCsi(b);
      this.parserState = ParserState.NORMAL;
    } else {
      this.parserState = ParserState.NORMAL;
    }
  }

  private dispatchCsi(final: number): void {
    const params = this.parseParams();

    if (this.isPrivate) {
      this.dispatchPrivate(final, params);
      return;
    }

    const p0 = params[0] ?? 0;
    const p1 = params[1] ?? 0;

    switch (final) {
      case 0x41: // A - CUU cursor up
        this.cursor.y = Math.max(0, this.cursor.y - Math.max(1, p0));
        break;
      case 0x42: // B - CUD cursor down
        this.cursor.y = Math.min(this.rows - 1, this.cursor.y + Math.max(1, p0));
        break;
      case 0x43: // C - CUF cursor forward
        this.cursor.x = Math.min(this.cols - 1, this.cursor.x + Math.max(1, p0));
        break;
      case 0x44: // D - CUB cursor back
        this.cursor.x = Math.max(0, this.cursor.x - Math.max(1, p0));
        break;
      case 0x45: // E - CNL cursor next line
        this.cursor.y = Math.min(this.rows - 1, this.cursor.y + Math.max(1, p0));
        this.cursor.x = 0;
        break;
      case 0x46: // F - CPL cursor prev line
        this.cursor.y = Math.max(0, this.cursor.y - Math.max(1, p0));
        this.cursor.x = 0;
        break;
      case 0x47: // G - CHA cursor horizontal absolute
        this.cursor.x = Math.min(this.cols - 1, Math.max(0, Math.max(1, p0) - 1));
        break;
      case 0x48: // H or f - CUP cursor position
      case 0x66:
        this.cursor.y = Math.min(this.rows - 1, Math.max(0, Math.max(1, p0) - 1));
        this.cursor.x = Math.min(this.cols - 1, Math.max(0, Math.max(1, p1) - 1));
        break;
      case 0x4a: // J - ED erase display
        this.eraseDisplay(p0);
        break;
      case 0x4b: // K - EL erase line
        this.eraseLine(p0);
        break;
      case 0x4c: // L - IL insert lines
        this.insertLines(Math.max(1, p0));
        break;
      case 0x4d: // M - DL delete lines
        this.deleteLines(Math.max(1, p0));
        break;
      case 0x50: // P - DCH delete characters
        this.deleteChars(Math.max(1, p0));
        break;
      case 0x53: // S - SU scroll up
        for (let i = 0; i < Math.max(1, p0); i++) this.scrollUp();
        break;
      case 0x54: // T - SD scroll down
        for (let i = 0; i < Math.max(1, p0); i++) this.scrollDown();
        break;
      case 0x58: // X - ECH erase characters
        this.eraseChars(Math.max(1, p0));
        break;
      case 0x6d: // m - SGR
        this.processSgr(params);
        break;
      case 0x72: // r - DECSTBM set scrolling region (simplified — ignore for v1)
        break;
      case 0x73: // s - save cursor
        this.saveCursor();
        break;
      case 0x75: // u - restore cursor
        this.restoreCursor();
        break;
      case 0x64: // d - VPA vertical position absolute
        this.cursor.y = Math.min(this.rows - 1, Math.max(0, Math.max(1, p0) - 1));
        break;
    }
  }

  private dispatchPrivate(final: number, params: number[]): void {
    const p0 = params[0] ?? 0;
    switch (final) {
      case 0x68: // h - set mode
        if (p0 === 1049) this.enterAltScreen();
        break;
      case 0x6c: // l - reset mode
        if (p0 === 1049) this.exitAltScreen();
        break;
    }
  }

  private parseParams(): number[] {
    if (!this.paramBuf) return [];
    return this.paramBuf.split(";").map((s) => (s === "" ? 0 : parseInt(s, 10)));
  }

  private processSgr(params: number[]): void {
    if (params.length === 0) params = [0];
    let i = 0;
    while (i < params.length) {
      const p = params[i]!;
      if (p === 0) { this.fg = COLOR_DEFAULT; this.bg = COLOR_DEFAULT; this.attrs = 0; }
      else if (p === 1) this.attrs |= ATTR_BOLD;
      else if (p === 2) this.attrs |= ATTR_DIM;
      else if (p === 3) this.attrs |= ATTR_ITALIC;
      else if (p === 4) this.attrs |= ATTR_UNDERLINE;
      else if (p === 5 || p === 6) this.attrs |= ATTR_BLINK;
      else if (p === 7) this.attrs |= ATTR_INVERSE;
      else if (p === 8) this.attrs |= ATTR_INVISIBLE;
      else if (p === 22) this.attrs &= ~(ATTR_BOLD | ATTR_DIM);
      else if (p === 23) this.attrs &= ~ATTR_ITALIC;
      else if (p === 24) this.attrs &= ~ATTR_UNDERLINE;
      else if (p === 25) this.attrs &= ~ATTR_BLINK;
      else if (p === 27) this.attrs &= ~ATTR_INVERSE;
      else if (p === 28) this.attrs &= ~ATTR_INVISIBLE;
      else if (p >= 30 && p <= 37) this.fg = ansiToColor(p, false);
      else if (p === 38) {
        const { color, consumed } = this.parseSgrColor(params, i + 1);
        this.fg = color; i += consumed; i++; continue;
      }
      else if (p === 39) this.fg = COLOR_DEFAULT;
      else if (p >= 40 && p <= 47) this.bg = ansiToColor(p - 10, false);
      else if (p === 48) {
        const { color, consumed } = this.parseSgrColor(params, i + 1);
        this.bg = color; i += consumed; i++; continue;
      }
      else if (p === 49) this.bg = COLOR_DEFAULT;
      else if (p >= 90 && p <= 97) this.fg = ansiToColor(p, true);
      else if (p >= 100 && p <= 107) this.bg = ansiToColor(p - 10, true);
      i++;
    }
  }

  private parseSgrColor(params: number[], start: number): { color: number; consumed: number } {
    const mode = params[start];
    if (mode === 5) {
      const idx = params[start + 1] ?? 0;
      return { color: color256(idx), consumed: 2 };
    }
    if (mode === 2) {
      const r = params[start + 1] ?? 0;
      const g = params[start + 2] ?? 0;
      const b = params[start + 3] ?? 0;
      return { color: colorRgb(r, g, b), consumed: 4 };
    }
    return { color: COLOR_DEFAULT, consumed: 1 };
  }

  private eraseDisplay(mode: number): void {
    const g = this.activeGrid();
    switch (mode) {
      case 0: // from cursor to end
        for (let c = this.cursor.x; c < this.cols; c++) g[this.cursor.y]![c] = emptyCell();
        for (let r = this.cursor.y + 1; r < this.rows; r++) g[r] = Array.from({ length: this.cols }, () => emptyCell());
        break;
      case 1: // from beginning to cursor
        for (let r = 0; r < this.cursor.y; r++) g[r] = Array.from({ length: this.cols }, () => emptyCell());
        for (let c = 0; c <= this.cursor.x; c++) g[this.cursor.y]![c] = emptyCell();
        break;
      case 2: // entire screen
        if (this.inAltScreen && this.altGrid) {
          this.altGrid = makeGrid(this.rows, this.cols);
        } else {
          this.grid = makeGrid(this.rows, this.cols);
        }
        break;
    }
  }

  private eraseLine(mode: number): void {
    const row = this.activeGrid()[this.cursor.y]!;
    switch (mode) {
      case 0: // to end
        for (let c = this.cursor.x; c < this.cols; c++) row[c] = emptyCell();
        break;
      case 1: // to start
        for (let c = 0; c <= this.cursor.x; c++) row[c] = emptyCell();
        break;
      case 2: // entire line
        for (let c = 0; c < this.cols; c++) row[c] = emptyCell();
        break;
    }
  }

  private eraseChars(n: number): void {
    const row = this.activeGrid()[this.cursor.y]!;
    for (let c = this.cursor.x; c < Math.min(this.cols, this.cursor.x + n); c++) row[c] = emptyCell();
  }

  private insertLines(n: number): void {
    const g = this.activeGrid();
    for (let i = 0; i < n; i++) {
      g.splice(this.rows - 1, 1);
      g.splice(this.cursor.y, 0, Array.from({ length: this.cols }, () => emptyCell()));
    }
  }

  private deleteLines(n: number): void {
    const g = this.activeGrid();
    for (let i = 0; i < n; i++) {
      g.splice(this.cursor.y, 1);
      g.push(Array.from({ length: this.cols }, () => emptyCell()));
    }
  }

  private deleteChars(n: number): void {
    const row = this.activeGrid()[this.cursor.y]!;
    row.splice(this.cursor.x, n);
    while (row.length < this.cols) row.push(emptyCell());
  }

  private scrollUp(): void {
    const g = this.activeGrid();
    g.shift();
    g.push(Array.from({ length: this.cols }, () => emptyCell()));
  }

  private scrollDown(): void {
    const g = this.activeGrid();
    g.pop();
    g.unshift(Array.from({ length: this.cols }, () => emptyCell()));
  }

  private enterAltScreen(): void {
    if (this.inAltScreen) return;
    this.altGrid = makeGrid(this.rows, this.cols);
    this.altCursor = { ...this.cursor };
    this.inAltScreen = true;
    this.cursor = { x: 0, y: 0 };
  }

  private exitAltScreen(): void {
    if (!this.inAltScreen) return;
    this.altGrid = null;
    this.inAltScreen = false;
    this.cursor = { ...this.altCursor };
  }

  private saveCursor(): void {
    this.savedCursor = { ...this.cursor };
  }

  private restoreCursor(): void {
    this.cursor = { ...this.savedCursor };
  }

  private fullReset(): void {
    this.grid = makeGrid(this.rows, this.cols);
    this.altGrid = null;
    this.inAltScreen = false;
    this.cursor = { x: 0, y: 0 };
    this.savedCursor = { x: 0, y: 0 };
    this.altCursor = { x: 0, y: 0 };
    this.fg = COLOR_DEFAULT;
    this.bg = COLOR_DEFAULT;
    this.attrs = 0;
  }

  private activeGrid(): Cell[][] {
    return this.inAltScreen && this.altGrid ? this.altGrid : this.grid;
  }

  getSnapshot(): SnapshotPayload {
    const g = this.activeGrid();
    const cells: SnapshotCell[][] = g.map((row) =>
      row.map((c) => ({ char: c.char, fg: c.fg, bg: c.bg, attrs: c.attrs })),
    );
    return { rows: this.rows, cols: this.cols, cells, cursor: { ...this.cursor } };
  }

  resize(rows: number, cols: number): void {
    const newGrid = makeGrid(rows, cols);
    const g = this.activeGrid();
    const minRows = Math.min(rows, this.rows);
    const minCols = Math.min(cols, this.cols);
    for (let r = 0; r < minRows; r++) {
      for (let c = 0; c < minCols; c++) {
        newGrid[r]![c] = { ...g[r]![c]! };
      }
    }
    this.rows = rows;
    this.cols = cols;
    if (this.inAltScreen && this.altGrid) {
      this.altGrid = newGrid;
    } else {
      this.grid = newGrid;
    }
    this.cursor.x = Math.min(this.cursor.x, cols - 1);
    this.cursor.y = Math.min(this.cursor.y, rows - 1);
  }

  dispose(): void {
    this.grid = [];
    this.altGrid = null;
  }
}

// Convert a snapshot payload to ANSI escape sequences.
// Returns a Buffer of bytes suitable for direct feed to a terminal renderer.
export function snapshotToBytes(snap: SnapshotPayload): Uint8Array {
  const parts: string[] = [];

  // ED2: erase entire display, then move cursor to home
  parts.push("\x1b[2J\x1b[H");

  let curFg = COLOR_DEFAULT;
  let curBg = COLOR_DEFAULT;
  let curAttrs = 0;

  for (let row = 0; row < snap.rows; row++) {
    for (let col = 0; col < snap.cols; col++) {
      const cell = snap.cells[row]?.[col];
      if (!cell) continue;

      // Skip empty continuation cells (wide char right half)
      if (cell.char === "") continue;

      // Skip default empty spaces (common case — saves bytes)
      if (cell.char === " " && cell.fg === COLOR_DEFAULT && cell.bg === COLOR_DEFAULT && cell.attrs === 0) continue;

      // Position cursor
      parts.push(`\x1b[${row + 1};${col + 1}H`);

      // Apply SGR if changed
      if (cell.fg !== curFg || cell.bg !== curBg || cell.attrs !== curAttrs) {
        const sgr: string[] = [];
        if (cell.attrs === 0 && curAttrs !== 0) sgr.push("0");
        if (cell.attrs & ATTR_BOLD && !(curAttrs & ATTR_BOLD)) sgr.push("1");
        if (cell.attrs & ATTR_DIM && !(curAttrs & ATTR_DIM)) sgr.push("2");
        if (cell.attrs & ATTR_ITALIC && !(curAttrs & ATTR_ITALIC)) sgr.push("3");
        if (cell.attrs & ATTR_UNDERLINE && !(curAttrs & ATTR_UNDERLINE)) sgr.push("4");
        if (cell.attrs & ATTR_INVERSE && !(curAttrs & ATTR_INVERSE)) sgr.push("7");
        if (cell.fg !== curFg) sgr.push(colorToFgSGR(cell.fg));
        if (cell.bg !== curBg) sgr.push(colorToBgSGR(cell.bg));
        if (sgr.length > 0) parts.push(`\x1b[${sgr.join(";")}m`);
        curFg = cell.fg; curBg = cell.bg; curAttrs = cell.attrs;
      }

      parts.push(cell.char);
    }
  }

  // Reset SGR to default
  if (curFg !== COLOR_DEFAULT || curBg !== COLOR_DEFAULT || curAttrs !== 0) {
    parts.push("\x1b[m");
  }

  // Restore cursor position
  parts.push(`\x1b[${snap.cursor.y + 1};${snap.cursor.x + 1}H`);

  return Buffer.from(parts.join("")) as unknown as Uint8Array;
}

export { colorToFgSGR, colorToBgSGR, COLOR_DEFAULT };
