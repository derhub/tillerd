import { ACTION, type ActionId } from "./ids";

/** A single accelerator chord in canonical string form, e.g. `CmdOrCtrl+Shift+P`. */
export type Accelerator = string;

export interface Chord {
  /** Cmd on macOS, Ctrl elsewhere -- the two are unified for matching. */
  cmdOrCtrl: boolean;
  alt: boolean;
  shift: boolean;
  /** A single uppercased character or a named key (`Enter`, `ArrowUp`, `,`, `\\`). */
  key: string;
}

const MODIFIER_KEYS = new Set(["Control", "Meta", "Alt", "Shift"]);

function normalizeKey(key: string): string {
  return key.length === 1 ? key.toUpperCase() : key;
}

/** Parse a chord string into its parts, or `null` when it carries no non-modifier key. */
export function parseAccelerator(input: string): Chord | null {
  const tokens = input
    .split("+")
    .map((t) => t.trim())
    .filter(Boolean);
  const chord: Chord = { cmdOrCtrl: false, alt: false, shift: false, key: "" };
  for (const token of tokens) {
    const t = token.toLowerCase();
    if (t === "cmdorctrl" || t === "cmd" || t === "ctrl" || t === "control") chord.cmdOrCtrl = true;
    else if (t === "alt") chord.alt = true;
    else if (t === "shift") chord.shift = true;
    else chord.key = normalizeKey(token);
  }
  return chord.key ? chord : null;
}

/** Render a chord in canonical order: CmdOrCtrl, Alt, Shift, then the key. */
export function formatAccelerator(chord: Chord): Accelerator {
  const parts: string[] = [];
  if (chord.cmdOrCtrl) parts.push("CmdOrCtrl");
  if (chord.alt) parts.push("Alt");
  if (chord.shift) parts.push("Shift");
  parts.push(chord.key);
  return parts.join("+");
}

/** Parse then re-format, yielding the canonical string, or `null` if unparseable. */
export function canonicalize(input: string): Accelerator | null {
  const chord = parseAccelerator(input);
  return chord ? formatAccelerator(chord) : null;
}

/** Render an accelerator for display, using mac glyphs when `mac` is set. */
export function displayAccelerator(accel: Accelerator, mac: boolean): string {
  const chord = parseAccelerator(accel);
  if (!chord) return accel;
  if (mac) {
    return (
      (chord.cmdOrCtrl ? "⌘" : "") + (chord.alt ? "⌥" : "") + (chord.shift ? "⇧" : "") + chord.key
    );
  }
  const parts: string[] = [];
  if (chord.cmdOrCtrl) parts.push("Ctrl");
  if (chord.alt) parts.push("Alt");
  if (chord.shift) parts.push("Shift");
  parts.push(chord.key);
  return parts.join("+");
}

/** Map a keyboard event to its canonical accelerator, or `null` for a modifier-only press. */
export function eventToAccelerator(e: KeyboardEvent): Accelerator | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  return formatAccelerator({
    cmdOrCtrl: e.metaKey || e.ctrlKey,
    alt: e.altKey,
    shift: e.shiftKey,
    key: normalizeKey(e.key),
  });
}

export const PRESET_NAMES = ["default", "vim", "vscode", "tmux"] as const;

export type PresetName = (typeof PRESET_NAMES)[number];

export const DEFAULT_PRESET: PresetName = "default";

export function isPresetName(value: unknown): value is PresetName {
  return typeof value === "string" && (PRESET_NAMES as readonly string[]).includes(value);
}

/**
 * Built-in baselines. `default` binds every static action; `vim` / `vscode` / `tmux` map the wired
 * actions in their own flavor and extend as actions are added. Single chords only -- multi-key
 * sequences are out of scope for this milestone.
 */
export const PRESETS: Record<PresetName, Partial<Record<ActionId, Accelerator>>> = {
  default: {
    [ACTION.projectNew]: "CmdOrCtrl+Shift+N",
    [ACTION.sessionNew]: "CmdOrCtrl+N",
    [ACTION.surfaceSpawn]: "CmdOrCtrl+T",
    [ACTION.surfaceClose]: "CmdOrCtrl+W",
    [ACTION.panelSplitH]: "CmdOrCtrl+\\",
    [ACTION.panelSplitV]: "CmdOrCtrl+Shift+\\",
    [ACTION.surfaceDetach]: "CmdOrCtrl+Shift+D",
    [ACTION.projectOpenNewWindow]: "CmdOrCtrl+Shift+O",
    [ACTION.viewLogs]: "CmdOrCtrl+Shift+L",
    [ACTION.appSettings]: "CmdOrCtrl+,",
  },
  vscode: {
    [ACTION.sessionNew]: "CmdOrCtrl+N",
    [ACTION.surfaceSpawn]: "CmdOrCtrl+Shift+`",
    [ACTION.surfaceClose]: "CmdOrCtrl+W",
    [ACTION.panelSplitH]: "CmdOrCtrl+\\",
    [ACTION.appSettings]: "CmdOrCtrl+,",
  },
  vim: {
    [ACTION.surfaceSpawn]: "CmdOrCtrl+Alt+N",
    [ACTION.surfaceClose]: "CmdOrCtrl+Alt+W",
    [ACTION.panelSplitH]: "CmdOrCtrl+Alt+S",
    [ACTION.panelSplitV]: "CmdOrCtrl+Alt+V",
  },
  tmux: {
    [ACTION.surfaceSpawn]: "CmdOrCtrl+Alt+T",
    [ACTION.surfaceClose]: "CmdOrCtrl+Alt+X",
    [ACTION.panelSplitH]: "CmdOrCtrl+Alt+5",
    [ACTION.panelSplitV]: "CmdOrCtrl+Alt+2",
  },
};

/** A per-action override map (canonical accelerators). Absence of a key means "use the preset". */
export type Overrides = Partial<Record<string, Accelerator>>;

/** Parse a stored overrides blob, tolerating malformed JSON. */
export function parseOverrides(raw: string): Overrides {
  try {
    const parsed: unknown = JSON.parse(raw);
    if (parsed && typeof parsed === "object") return parsed as Overrides;
  } catch {
    // malformed -- start from none
  }
  return {};
}

/** Merge the preset baseline with per-action overrides into the resolved binding map. */
export function resolveBindings(
  preset: PresetName,
  overrides: Overrides,
): Map<string, Accelerator> {
  const resolved = new Map<string, Accelerator>();
  for (const [id, accel] of Object.entries(PRESETS[preset])) {
    if (accel) resolved.set(id, accel);
  }
  for (const [id, accel] of Object.entries(overrides)) {
    if (accel) resolved.set(id, accel);
  }
  return resolved;
}
