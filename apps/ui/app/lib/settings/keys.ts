export const THEME_KEY = "theme";
export const TERMINAL_SCHEME_KEY = "terminal.scheme";
export const DEFAULT_COMMAND_KEY = "default.command";
export const DEFAULT_TEMPLATE_KEY = "default.template";
export const SIDEBAR_EXPANDED_KEY = "sidebar.expanded";

export const KEYBINDINGS_PRESET_KEY = "keybindings.preset";
export const KEYBINDINGS_LEADER_KEY = "keybindings.leader";
export const KEYBINDINGS_OVERRIDES_KEY = "keybindings.overrides";

// Registered natively on desktop as the command center shortcut.
export const DEFAULT_LEADER = "CmdOrCtrl+K";

export type Theme = "light" | "dark";

export const THEMES: readonly Theme[] = ["dark", "light"] as const;

// Matches the hardcoded appearance predating the settings system.
export const DEFAULT_THEME: Theme = "dark";

export function isTheme(value: unknown): value is Theme {
  return value === "light" || value === "dark";
}
