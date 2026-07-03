export const THEME_KEY = "theme";
export const TERMINAL_SCHEME_KEY = "terminal.scheme";
export const DEFAULT_COMMAND_KEY = "default.command";
export const DEFAULT_TEMPLATE_KEY = "default.template";

// View pointers (ADR-0044): durable UI position, one settings key per target so
// concurrent windows never read-modify-write clobber each other.
export const VIEW_ACTIVE_WORKSPACE_KEY = "view.active-workspace";
export const VIEW_LAST_SESSION_PREFIX = "view.last-session.";
export const SIDEBAR_EXPANDED_PREFIX = "sidebar.expanded.";

export function lastSessionKey(projectId: string): string {
  return `${VIEW_LAST_SESSION_PREFIX}${projectId}`;
}

export function sidebarExpandedKey(projectId: string): string {
  return `${SIDEBAR_EXPANDED_PREFIX}${projectId}`;
}

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
