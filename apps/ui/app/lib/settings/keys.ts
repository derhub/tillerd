export const THEME_KEY = "theme";
export const TERMINAL_SCHEME_KEY = "terminal.scheme";
export const DEFAULT_COMMAND_KEY = "default.command";
export const DEFAULT_TEMPLATE_KEY = "default.template";

// UI zoom (webview zoom factor, General settings / ui-settings-editor spec). 1 is 100%;
// bounds mirror the Tauri webview zoom polyfill's own range so a clamp never fights the
// host's clamp.
export const UI_ZOOM_KEY = "ui.zoom";
export const DEFAULT_UI_ZOOM = 1;
export const UI_ZOOM_MIN = 0.5;
export const UI_ZOOM_MAX = 2;
export const UI_ZOOM_STEP = 0.1;

export function clampUiZoom(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_UI_ZOOM;
  return Math.min(UI_ZOOM_MAX, Math.max(UI_ZOOM_MIN, value));
}

// "Don't ask again" for the close-surface confirmation dialog (ui-panel-compound spec).
export const PANEL_CLOSE_CONFIRM_SKIP_KEY = "panel.closeConfirm.skip";

// Terminal typography and buffer settings (ui-terminal-pane spec). Read on the pane and applied
// live to mounted terminals via useLiveTerminalTypography. Defaults mirror the hardcoded xterm
// construction options predating the settings path.
export const TERMINAL_FONT_SIZE_KEY = "terminal.fontSize";
export const TERMINAL_FONT_FAMILY_KEY = "terminal.fontFamily";
export const TERMINAL_LINE_HEIGHT_KEY = "terminal.lineHeight";
export const TERMINAL_CURSOR_STYLE_KEY = "terminal.cursorStyle";
export const TERMINAL_CURSOR_BLINK_KEY = "terminal.cursorBlink";
export const TERMINAL_SCROLLBACK_KEY = "terminal.scrollback";

// Clipboard hygiene settings (ui-terminal-pane spec).
export const TERMINAL_COPY_ON_SELECT_KEY = "terminal.copyOnSelect";
export const TERMINAL_CONFIRM_PASTE_KEY = "terminal.confirmPaste";

export const DEFAULT_TERMINAL_FONT_SIZE = 13;
export const DEFAULT_TERMINAL_FONT_FAMILY =
  '"Geist Mono Variable", "Cascadia Code", "Fira Code", "JetBrains Mono", monospace';
export const DEFAULT_TERMINAL_LINE_HEIGHT = 1;
export type TerminalCursorStyle = "block" | "underline" | "bar";
export const DEFAULT_TERMINAL_CURSOR_STYLE: TerminalCursorStyle = "block";
export const DEFAULT_TERMINAL_CURSOR_BLINK = true;
export const DEFAULT_TERMINAL_SCROLLBACK = 1000;
export const DEFAULT_TERMINAL_COPY_ON_SELECT = false;
export const DEFAULT_TERMINAL_CONFIRM_PASTE = false;

export function isTerminalCursorStyle(value: unknown): value is TerminalCursorStyle {
  return value === "block" || value === "underline" || value === "bar";
}

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
