const encoder = new TextEncoder();

export interface KeyEncodeOptions {
  /**
   * Encode arrow keys in application-cursor mode (`ESC O A` instead of `ESC [ A`).
   * Programs that enable DECCKM (vim, less, readline in some modes) expect this.
   */
  applicationCursor?: boolean;
}

/** Arrow keys: [normal, application-cursor]. */
const ARROWS: Record<string, readonly [string, string]> = {
  up: ["\x1b[A", "\x1bOA"],
  arrowup: ["\x1b[A", "\x1bOA"],
  down: ["\x1b[B", "\x1bOB"],
  arrowdown: ["\x1b[B", "\x1bOB"],
  right: ["\x1b[C", "\x1bOC"],
  arrowright: ["\x1b[C", "\x1bOC"],
  left: ["\x1b[D", "\x1bOD"],
  arrowleft: ["\x1b[D", "\x1bOD"],
};

/** Named keys -> the bytes a terminal sends for them. */
const NAMED: Record<string, string> = {
  enter: "\r",
  return: "\r",
  tab: "\t",
  escape: "\x1b",
  esc: "\x1b",
  backspace: "\x7f",
  delete: "\x1b[3~",
  del: "\x1b[3~",
  space: " ",
  plus: "+",
  home: "\x1b[H",
  end: "\x1b[F",
  pageup: "\x1b[5~",
  pgup: "\x1b[5~",
  pagedown: "\x1b[6~",
  pgdn: "\x1b[6~",
  insert: "\x1b[2~",
  ins: "\x1b[2~",
  f1: "\x1bOP",
  f2: "\x1bOQ",
  f3: "\x1bOR",
  f4: "\x1bOS",
  f5: "\x1b[15~",
  f6: "\x1b[17~",
  f7: "\x1b[18~",
  f8: "\x1b[19~",
  f9: "\x1b[20~",
  f10: "\x1b[21~",
  f11: "\x1b[23~",
  f12: "\x1b[24~",
};

/** Non-alphabetic Ctrl combos -> control byte. */
const CTRL_SYMBOLS: Record<string, number> = {
  "[": 0x1b,
  "3": 0x1b,
  "\\": 0x1c,
  "4": 0x1c,
  "]": 0x1d,
  "5": 0x1d,
  "^": 0x1e,
  "6": 0x1e,
  _: 0x1f,
  "7": 0x1f,
  "@": 0x00,
  "2": 0x00,
  " ": 0x00,
  "?": 0x7f,
};

function namedToString(key: string, applicationCursor: boolean): string | null {
  const k = key.toLowerCase();
  const arrow = ARROWS[k];
  if (arrow) return applicationCursor ? arrow[1] : arrow[0];
  const named = NAMED[k];
  return named ?? null;
}

function comboToString(combo: string, applicationCursor: boolean): string | null {
  let ctrl = false;
  let alt = false;
  let shift = false;
  let keyPart = "";
  for (const part of combo.split("+")) {
    switch (part.toLowerCase()) {
      case "ctrl":
      case "control":
        ctrl = true;
        break;
      case "alt":
      case "meta":
      case "option":
        alt = true;
        break;
      case "shift":
        shift = true;
        break;
      default:
        keyPart = part;
    }
  }
  if (!keyPart) return null;

  if (ctrl && keyPart.toLowerCase() === "space") return (alt ? "\x1b" : "") + "\x00";

  const named = namedToString(keyPart, applicationCursor);
  if (named !== null) return alt ? "\x1b" + named : named;

  const chars = [...keyPart];
  if (chars.length !== 1) return null;
  let c = chars[0] as string;

  if (shift && c >= "a" && c <= "z") c = c.toUpperCase();

  if (ctrl) {
    let code: number;
    if ((c >= "a" && c <= "z") || (c >= "A" && c <= "Z")) {
      code = c.toUpperCase().charCodeAt(0) - 0x40;
    } else {
      const sym = CTRL_SYMBOLS[c];
      if (sym === undefined) return null;
      code = sym;
    }
    return (alt ? "\x1b" : "") + String.fromCharCode(code);
  }

  if (alt) return "\x1b" + c;
  return c;
}

function partToString(part: string, applicationCursor: boolean): string | null {
  const combo = comboToString(part, applicationCursor);
  if (combo !== null) return combo;
  const named = namedToString(part, applicationCursor);
  if (named !== null) return named;
  return [...part].length === 1 ? part : null;
}

/**
 * Encode a key sequence into one byte buffer per key.
 *
 * A spec is space-separated keys, each a combo (`Ctrl+C`, `Ctrl+Alt+X`), a named
 * key (`Enter`, `Escape`, `F1`), or a single character (`a`, `:`). Returning one
 * buffer per key lets callers interleave delays for chord-sensitive programs.
 *
 * @throws if any key in the spec is unrecognized.
 */
export function encodeKeySequence(spec: string, opts: KeyEncodeOptions = {}): Uint8Array[] {
  const applicationCursor = opts.applicationCursor ?? false;
  const parts = spec.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) throw new Error(`empty key spec: ${JSON.stringify(spec)}`);
  return parts.map((part) => {
    const s = partToString(part, applicationCursor);
    if (s === null) throw new Error(`unrecognized key: ${JSON.stringify(part)}`);
    return encoder.encode(s);
  });
}

/**
 * Encode a key spec into a single byte buffer (sequence parts concatenated, no
 * inter-key delay). Convenience over {@link encodeKeySequence} for the common
 * single-key case.
 *
 * @throws if any key in the spec is unrecognized.
 */
export function encodeKey(spec: string, opts: KeyEncodeOptions = {}): Uint8Array {
  const chunks = encodeKeySequence(spec, opts);
  if (chunks.length === 1) return chunks[0] as Uint8Array;
  const total = chunks.reduce((n, c) => n + c.length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const c of chunks) {
    out.set(c, offset);
    offset += c.length;
  }
  return out;
}
