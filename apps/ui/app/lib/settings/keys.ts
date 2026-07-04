export const THEME_KEY = "theme";
export const TERMINAL_SCHEME_KEY = "terminal.scheme";
export const DEFAULT_COMMAND_KEY = "default.command";
export const DEFAULT_TEMPLATE_KEY = "default.template";

// "Don't ask again" for the close-surface confirmation dialog (ui-panel-compound spec).
export const PANEL_CLOSE_CONFIRM_SKIP_KEY = "panel.closeConfirm.skip";

// View pointers : durable UI position, one settings key per target so
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

// Workbench chrome layout (VS Code-like shell): active sidebar view, sidebar and
// bottom-panel visibility + size, and the bottom-panel active tab. All global-scope
// settings keys so the layout restores on launch. Consumed through lib/workbench.ts;
// the string defaults below mirror the VIEW_DEFS/tab ids in components/workbench.
export const WORKBENCH_PREFIX = "workbench.";
export const WORKBENCH_VIEW_KEY = "workbench.view";
export const WORKBENCH_SIDEBAR_VISIBLE_KEY = "workbench.sidebar.visible";
export const WORKBENCH_SIDEBAR_SIZE_KEY = "workbench.sidebar.size";
export const WORKBENCH_PANEL_VISIBLE_KEY = "workbench.panel.visible";
export const WORKBENCH_PANEL_SIZE_KEY = "workbench.panel.size";
export const WORKBENCH_PANEL_TAB_KEY = "workbench.panel.tab";

// First-launch layout: Sessions view active, sidebar shown, bottom panel hidden.
export const WORKBENCH_DEFAULTS = {
  view: "sessions",
  sidebarVisible: true,
  sidebarSize: 224,
  panelVisible: false,
  panelSize: 200,
  panelTab: "logs",
} as const;

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
