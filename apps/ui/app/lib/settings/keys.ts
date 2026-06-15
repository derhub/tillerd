/** Setting keys stored in the orchestrator `setting` table (global scope unless noted). */
export const THEME_KEY = "theme";
export const TERMINAL_SCHEME_KEY = "terminal.scheme";
export const DEFAULT_COMMAND_KEY = "default.command";
export const DEFAULT_TEMPLATE_KEY = "default.template";
export const SIDEBAR_EXPANDED_KEY = "sidebar.expanded";

/** Command center: active keybinding preset, the leader-key chord, and the per-action override map. */
export const KEYBINDINGS_PRESET_KEY = "keybindings.preset";
export const KEYBINDINGS_LEADER_KEY = "keybindings.leader";
export const KEYBINDINGS_OVERRIDES_KEY = "keybindings.overrides";

/** Opens the command center; rebindable. Common palette convention, registered natively on desktop. */
export const DEFAULT_LEADER = "CmdOrCtrl+K";

/** Prefix for "don't ask again" confirmation-suppression keys (e.g. `confirm.close-surface`). */
export const CONFIRM_PREFIX = "confirm.";

export const confirmKey = (name: string): string => `${CONFIRM_PREFIX}${name}`;

/** Light / dark appearance. */
export type Theme = "light" | "dark";

export const THEMES: readonly Theme[] = ["dark", "light"] as const;

/** Matches the hardcoded appearance the app shipped before settings existed. */
export const DEFAULT_THEME: Theme = "dark";

export function isTheme(value: unknown): value is Theme {
  return value === "light" || value === "dark";
}
