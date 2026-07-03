import { COMMAND_DEFS } from "./defs";

export type Accelerator = string;

export interface Chord {
  cmdOrCtrl: boolean;
  alt: boolean;
  shift: boolean;
  key: string;
}

const MODIFIER_KEYS = new Set(["Control", "Meta", "Alt", "Shift"]);

function normalizeKey(key: string): string {
  return key.length === 1 ? key.toUpperCase() : key;
}

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

export function formatAccelerator(chord: Chord): Accelerator {
  const parts: string[] = [];
  if (chord.cmdOrCtrl) parts.push("CmdOrCtrl");
  if (chord.alt) parts.push("Alt");
  if (chord.shift) parts.push("Shift");
  parts.push(chord.key);
  return parts.join("+");
}

export function canonicalize(input: string): Accelerator | null {
  const chord = parseAccelerator(input);
  return chord ? formatAccelerator(chord) : null;
}

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

// Preset baselines are derived from the command definitions' per-preset default
// keys -- the definition table is the single source of truth. `default` binds
// every action that declares a default key; flavor presets bind their subset.
// Single chords only -- multi-key sequences are out of scope.
function buildPresets(): Record<PresetName, Partial<Record<string, Accelerator>>> {
  const presets = { default: {}, vscode: {}, vim: {}, tmux: {} } as Record<
    PresetName,
    Partial<Record<string, Accelerator>>
  >;
  for (const def of COMMAND_DEFS) {
    if (!def.defaultKeys) continue;
    for (const name of PRESET_NAMES) {
      const accel = def.defaultKeys[name];
      if (accel) presets[name][def.id] = accel;
    }
  }
  return presets;
}

export const PRESETS: Record<PresetName, Partial<Record<string, Accelerator>>> = buildPresets();

export type Overrides = Partial<Record<string, Accelerator>>;

export function parseOverrides(raw: string): Overrides {
  try {
    const parsed: unknown = JSON.parse(raw);
    if (parsed && typeof parsed === "object") return parsed as Overrides;
  } catch {
    // malformed -- start from none
  }
  return {};
}

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
