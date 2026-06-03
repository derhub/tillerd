import { test, expect, describe } from "bun:test";
import { VtState, snapshotToBytes } from "../src/vt-state";

function enc(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

function cell(vt: VtState, row: number, col: number) {
  return vt.getSnapshot().cells[row]?.[col];
}

function cursor(vt: VtState) {
  return vt.getSnapshot().cursor;
}

describe("VtState — basic character writes", () => {
  test("writes ASCII characters into cell grid", () => {
    const vt = new VtState(5, 10);
    vt.feed(enc("Hello"));
    expect(cell(vt, 0, 0)?.char).toBe("H");
    expect(cell(vt, 0, 1)?.char).toBe("e");
    expect(cell(vt, 0, 4)?.char).toBe("o");
    expect(cursor(vt)).toEqual({ x: 5, y: 0 });
  });

  test("CR moves cursor to col 0", () => {
    const vt = new VtState(5, 10);
    vt.feed(enc("abc\r"));
    expect(cursor(vt)).toEqual({ x: 0, y: 0 });
    expect(cell(vt, 0, 0)?.char).toBe("a");
  });

  test("LF moves cursor down", () => {
    const vt = new VtState(5, 10);
    vt.feed(enc("a\n"));
    expect(cursor(vt).y).toBe(1);
  });

  test("LF at bottom scrolls grid up", () => {
    const vt = new VtState(3, 5);
    vt.feed(enc("a\r\nb\r\nc\r\n"));
    const snap = vt.getSnapshot();
    expect(snap.cells[0]?.[0]?.char).toBe("b");
    expect(snap.cells[1]?.[0]?.char).toBe("c");
    expect(snap.cells[2]?.[0]?.char).toBe(" ");
  });

  test("wraps at column boundary", () => {
    const vt = new VtState(3, 4);
    vt.feed(enc("abcde"));
    expect(cell(vt, 0, 0)?.char).toBe("a");
    expect(cell(vt, 0, 3)?.char).toBe("d");
    expect(cell(vt, 1, 0)?.char).toBe("e");
    expect(cursor(vt)).toEqual({ x: 1, y: 1 });
  });

  test("partial writes — sequence split across two feed calls", () => {
    const vt = new VtState(5, 10);
    const seq = enc("\x1b[32m");
    vt.feed(seq.slice(0, 2));  // ESC[
    vt.feed(seq.slice(2));     // 32m
    vt.feed(enc("X"));
    expect(cell(vt, 0, 0)?.char).toBe("X");
    expect(cell(vt, 0, 0)?.fg).not.toBe(0);
  });
});

describe("VtState — cursor movement (CUP/CSI)", () => {
  test("ESC[H moves cursor to home (0,0)", () => {
    const vt = new VtState(5, 10);
    vt.feed(enc("abc\x1b[H"));
    expect(cursor(vt)).toEqual({ x: 0, y: 0 });
  });

  test("ESC[2;3H moves cursor to row 1, col 2", () => {
    const vt = new VtState(5, 10);
    vt.feed(enc("\x1b[2;3H"));
    expect(cursor(vt)).toEqual({ x: 2, y: 1 });
  });

  test("ESC[A moves cursor up", () => {
    const vt = new VtState(5, 10);
    vt.feed(enc("\x1b[3;1H\x1b[A"));
    expect(cursor(vt).y).toBe(1);
  });

  test("ESC[B moves cursor down", () => {
    const vt = new VtState(5, 10);
    vt.feed(enc("\x1b[B"));
    expect(cursor(vt).y).toBe(1);
  });

  test("ESC[C moves cursor forward", () => {
    const vt = new VtState(5, 10);
    vt.feed(enc("\x1b[3C"));
    expect(cursor(vt).x).toBe(3);
  });

  test("ESC[D moves cursor back", () => {
    const vt = new VtState(5, 10);
    vt.feed(enc("\x1b[5C\x1b[2D"));
    expect(cursor(vt).x).toBe(3);
  });
});

describe("VtState — erase sequences", () => {
  test("ESC[2J erases entire screen", () => {
    const vt = new VtState(3, 5);
    vt.feed(enc("hello\r\nworld\x1b[2J"));
    const snap = vt.getSnapshot();
    for (let r = 0; r < 3; r++) {
      for (let c = 0; c < 5; c++) {
        expect(snap.cells[r]?.[c]?.char).toBe(" ");
      }
    }
  });

  test("ESC[K erases from cursor to end of line", () => {
    const vt = new VtState(3, 5);
    vt.feed(enc("hello\x1b[1;3H\x1b[K"));
    expect(cell(vt, 0, 0)?.char).toBe("h");
    expect(cell(vt, 0, 1)?.char).toBe("e");
    expect(cell(vt, 0, 2)?.char).toBe(" ");
    expect(cell(vt, 0, 4)?.char).toBe(" ");
  });

  test("ESC[2K erases entire line", () => {
    const vt = new VtState(3, 5);
    vt.feed(enc("hello\x1b[1;1H\x1b[2K"));
    for (let c = 0; c < 5; c++) {
      expect(cell(vt, 0, c)?.char).toBe(" ");
    }
  });
});

describe("VtState — SGR attributes", () => {
  test("ESC[1m sets bold", () => {
    const vt = new VtState(3, 10);
    vt.feed(enc("\x1b[1mX"));
    expect(cell(vt, 0, 0)?.attrs).toBe(0x01);
  });

  test("ESC[m resets all attributes", () => {
    const vt = new VtState(3, 10);
    vt.feed(enc("\x1b[1;32mX\x1b[mY"));
    expect(cell(vt, 0, 0)?.attrs).toBe(0x01);
    expect(cell(vt, 0, 1)?.attrs).toBe(0);
    expect(cell(vt, 0, 1)?.fg).toBe(0);
  });

  test("ESC[32m sets green fg color", () => {
    const vt = new VtState(3, 10);
    vt.feed(enc("\x1b[32mX"));
    expect(cell(vt, 0, 0)?.fg).not.toBe(0);
  });

  test("ESC[41m sets red bg color", () => {
    const vt = new VtState(3, 10);
    vt.feed(enc("\x1b[41mX"));
    expect(cell(vt, 0, 0)?.bg).not.toBe(0);
  });
});

describe("VtState — alternate screen (DECSET 1049)", () => {
  test("entering alt screen clears and switches grid", () => {
    const vt = new VtState(3, 10);
    vt.feed(enc("hello"));
    vt.feed(enc("\x1b[?1049h"));
    // Alt screen is blank, cursor at 0,0
    expect(cursor(vt)).toEqual({ x: 0, y: 0 });
    expect(cell(vt, 0, 0)?.char).toBe(" ");
  });

  test("writing to alt screen does not affect primary", () => {
    const vt = new VtState(3, 10);
    vt.feed(enc("hello"));
    vt.feed(enc("\x1b[?1049h"));
    vt.feed(enc("alt"));
    vt.feed(enc("\x1b[?1049l"));
    // Primary screen restored
    expect(cell(vt, 0, 0)?.char).toBe("h");
  });
});

describe("VtState — resize", () => {
  test("resize preserves cells in overlapping region", () => {
    const vt = new VtState(3, 10);
    vt.feed(enc("abc"));
    vt.resize(5, 15);
    expect(cell(vt, 0, 0)?.char).toBe("a");
    expect(cell(vt, 0, 2)?.char).toBe("c");
    expect(vt.getSnapshot().rows).toBe(5);
    expect(vt.getSnapshot().cols).toBe(15);
  });

  test("resize clears newly exposed cells", () => {
    const vt = new VtState(3, 5);
    vt.feed(enc("abc"));
    vt.resize(3, 10);
    expect(cell(vt, 0, 5)?.char).toBe(" ");
  });

  test("resize drops content beyond new bounds", () => {
    const vt = new VtState(3, 10);
    vt.feed(enc("abcdefghij"));
    vt.resize(3, 5);
    expect(vt.getSnapshot().cols).toBe(5);
    expect(cell(vt, 0, 0)?.char).toBe("a");
  });
});

describe("VtState — CJK double-width characters", () => {
  test("CJK char occupies two columns", () => {
    const vt = new VtState(3, 10);
    vt.feed(new TextEncoder().encode("好"));
    // Wide char: cursor advances by 2
    expect(cursor(vt).x).toBe(2);
    expect(cell(vt, 0, 0)?.char).toBe("好");
    expect(cell(vt, 0, 1)?.char).toBe("");
  });

  test("ASCII after CJK char positions correctly", () => {
    const vt = new VtState(3, 10);
    vt.feed(new TextEncoder().encode("好A"));
    expect(cell(vt, 0, 0)?.char).toBe("好");
    expect(cell(vt, 0, 2)?.char).toBe("A");
    expect(cursor(vt).x).toBe(3);
  });
});

  test("ED2 in alt screen does not clobber primary buffer", () => {
    const vt = new VtState(3, 10);
    vt.feed(enc("primary"));
    vt.feed(enc("\x1b[?1049h"));   // enter alt
    vt.feed(enc("alt content\x1b[2J")); // write + ED2 on alt
    vt.feed(enc("\x1b[?1049l"));   // exit alt
    // primary content must survive
    expect(cell(vt, 0, 0)?.char).toBe("p");
    expect(cell(vt, 0, 6)?.char).toBe("y");
  });

describe("snapshotToBytes — escape sequence conversion", () => {
  test("produces ED2+home prefix", () => {
    const vt = new VtState(3, 5);
    const snap = vt.getSnapshot();
    const bytes = snapshotToBytes(snap);
    const str = new TextDecoder().decode(bytes);
    expect(str.startsWith("\x1b[2J\x1b[H")).toBe(true);
  });

  test("encodes non-default cells with cursor positioning", () => {
    const vt = new VtState(3, 5);
    vt.feed(enc("A"));
    const snap = vt.getSnapshot();
    const bytes = snapshotToBytes(snap);
    const str = new TextDecoder().decode(bytes);
    expect(str).toContain("A");
    expect(str).toContain("\x1b[1;1H");
  });

  test("restores cursor position at end", () => {
    const vt = new VtState(3, 5);
    vt.feed(enc("\x1b[2;3HA"));
    const snap = vt.getSnapshot();
    const bytes = snapshotToBytes(snap);
    const str = new TextDecoder().decode(bytes);
    // Cursor was at row 1, col 3 (0-indexed) → ESC[2;4H
    expect(str.endsWith("\x1b[2;4H")).toBe(true);
  });

  test("wide-char cursor advance is correct in output", () => {
    const vt = new VtState(3, 10);
    vt.feed(new TextEncoder().encode("好X"));
    const snap = vt.getSnapshot();
    const str = new TextDecoder().decode(snapshotToBytes(snap));
    expect(str).toContain("好");
    expect(str).toContain("X");
    // X must be positioned at column 3 (after wide char + continuation)
    expect(str).toContain("\x1b[1;3H");
  });
});
