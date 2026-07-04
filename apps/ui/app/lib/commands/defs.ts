// The single source of truth for UI command declarations: identity, titles,
// keywords, surfaces, default keybindings per preset, and availability. Handlers
// register by id at their feature sites (see registry `useCommand`). This table
// replaces the split between `ids.ts` titles and `keybindings.ts` presets.

import {
  Archive,
  Command,
  ExternalLink,
  LayoutTemplate,
  MessagesSquare,
  PanelBottom,
  PanelLeft,
  Pencil,
  Search,
  Trash2,
} from "lucide-react";

import type { CommandDef } from "./types";

import { ACTION, SESSION_SEARCH_ACTION_ID, SESSION_SEARCH_TITLE } from "./ids";

// The panel/surface commands are scoped by handler presence: their handlers are
// registered only while the panel host (PanelContent) is mounted, so they are
// absent from every surface otherwise -- no `when` gate is needed or correct
// (an active panel host is not the same as a URL session id).
export const COMMAND_DEFS: readonly CommandDef[] = [
  {
    id: ACTION.projectNew,
    title: "New project",
    keywords: ["project", "create"],
    defaultKeys: { default: "CmdOrCtrl+Shift+N" },
  },
  {
    id: ACTION.sessionNew,
    title: "New session",
    keywords: ["session", "create"],
    defaultKeys: { default: "CmdOrCtrl+N", vscode: "CmdOrCtrl+N" },
  },
  {
    id: ACTION.surfaceSpawn,
    title: "New terminal",
    keywords: ["terminal", "surface", "spawn"],
    defaultKeys: {
      default: "CmdOrCtrl+T",
      vscode: "CmdOrCtrl+Shift+`",
      vim: "CmdOrCtrl+Alt+N",
      tmux: "CmdOrCtrl+Alt+T",
    },
  },
  {
    id: ACTION.surfaceClose,
    title: "Close panel",
    keywords: ["close", "surface"],
    defaultKeys: {
      default: "CmdOrCtrl+W",
      vscode: "CmdOrCtrl+W",
      vim: "CmdOrCtrl+Alt+W",
      tmux: "CmdOrCtrl+Alt+X",
    },
  },
  {
    id: ACTION.panelSplitH,
    title: "Split panel right",
    keywords: ["split", "horizontal", "right"],
    defaultKeys: {
      default: "CmdOrCtrl+\\",
      vscode: "CmdOrCtrl+\\",
      vim: "CmdOrCtrl+Alt+S",
      tmux: "CmdOrCtrl+Alt+5",
    },
  },
  {
    id: ACTION.panelSplitV,
    title: "Split panel down",
    keywords: ["split", "vertical", "down"],
    defaultKeys: {
      default: "CmdOrCtrl+Shift+\\",
      vim: "CmdOrCtrl+Alt+V",
      tmux: "CmdOrCtrl+Alt+2",
    },
  },
  {
    id: ACTION.surfaceDetach,
    title: "Detach panel",
    keywords: ["detach", "window"],
    defaultKeys: { default: "CmdOrCtrl+Shift+D" },
  },
  {
    id: ACTION.projectOpenNewWindow,
    title: "Open project in new window",
    keywords: ["window", "project", "detach"],
    defaultKeys: { default: "CmdOrCtrl+Shift+O" },
  },
  {
    id: ACTION.viewLogs,
    title: "View logs",
    keywords: ["logs", "observability"],
    defaultKeys: { default: "CmdOrCtrl+Shift+L" },
  },
  {
    id: ACTION.appSettings,
    title: "Settings",
    keywords: ["preferences", "theme", "keybindings"],
    defaultKeys: { default: "CmdOrCtrl+,", vscode: "CmdOrCtrl+," },
  },
  // Sidebar view switches, projected onto the activity bar. Their `checked` state
  // marks the active view (useWorkbenchCommands seeds `activeView`); selecting the
  // active view toggles sidebar visibility -- the handler owns that.
  {
    id: ACTION.viewSessions,
    title: "Sessions",
    icon: MessagesSquare,
    surfaces: ["activitybar"],
    group: "view",
    keywords: ["sessions", "projects", "sidebar"],
    toggle: (c) => c.activeView === "sessions",
  },
  {
    id: ACTION.viewSearch,
    title: "Search",
    icon: Search,
    surfaces: ["activitybar"],
    group: "view",
    keywords: ["search", "find", "sidebar"],
    toggle: (c) => c.activeView === "search",
  },
  {
    id: ACTION.viewCommands,
    title: "Commands",
    icon: Command,
    surfaces: ["activitybar"],
    group: "view",
    keywords: ["commands", "library", "sidebar"],
    toggle: (c) => c.activeView === "commands",
  },
  {
    id: ACTION.viewTemplates,
    title: "Templates",
    icon: LayoutTemplate,
    surfaces: ["activitybar"],
    group: "view",
    keywords: ["templates", "launch", "sidebar"],
    toggle: (c) => c.activeView === "templates",
  },
  // Checked state is a straight read: useWorkbenchCommands seeds these context
  // keys with the live boolean every render, so no "unset means visible"
  // special-casing is needed here.
  {
    id: ACTION.panelToggleLeft,
    title: "Toggle sidebar",
    icon: PanelLeft,
    surfaces: ["titlebar", "palette"],
    group: "view",
    keywords: ["sidebar", "left", "panel", "toggle"],
    defaultKeys: { default: "CmdOrCtrl+B" },
    toggle: (c) => Boolean(c.sidebarVisible),
  },
  {
    id: ACTION.panelToggleBottom,
    title: "Toggle bottom panel",
    icon: PanelBottom,
    surfaces: ["titlebar", "palette"],
    group: "view",
    keywords: ["bottom", "panel", "toggle"],
    defaultKeys: { default: "CmdOrCtrl+J" },
    toggle: (c) => Boolean(c.bottomPanelVisible),
  },
  {
    id: ACTION.commandToggle,
    title: "Toggle command palette",
    icon: Command,
    surfaces: ["titlebar", "palette"],
    group: "view",
    keywords: ["command", "palette"],
    defaultKeys: { default: "CmdOrCtrl+Shift+K" },
    toggle: (c) => Boolean(c.commandPaletteOpen),
  },
  {
    id: SESSION_SEARCH_ACTION_ID,
    title: SESSION_SEARCH_TITLE,
    keywords: ["session", "switch", "go to", "find", "search"],
    defaultKeys: { default: "CmdOrCtrl+P" },
  },
  // Row-scoped context-menu actions. `menu.projectRow`/`menu.sessionRow` and the
  // `menu.canRename`/`menu.canDelete` guards are pushed into the context store by
  // EntityContextMenu while a given row's menu is open (context.ts's setContext
  // model) -- these defs stay declarative and EntityContextMenu never special-cases
  // an entity kind or command id.
  {
    id: ACTION.projectRename,
    title: "Rename",
    icon: Pencil,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.projectRow", "menu.canRename"],
  },
  {
    id: ACTION.projectOpenNewWindowRow,
    title: "Open in new window",
    icon: ExternalLink,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.projectRow"],
  },
  {
    id: ACTION.projectDelete,
    title: "Delete",
    icon: Trash2,
    surfaces: ["contextmenu"],
    group: "destructive",
    when: ["menu.projectRow", "menu.canDelete"],
  },
  {
    id: ACTION.sessionRename,
    title: "Rename",
    icon: Pencil,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.sessionRow"],
  },
  {
    id: ACTION.sessionArchive,
    title: "Archive",
    icon: Archive,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.sessionRow"],
  },
  {
    id: ACTION.sessionDelete,
    title: "Delete",
    icon: Trash2,
    surfaces: ["contextmenu"],
    group: "destructive",
    when: ["menu.sessionRow"],
  },
];

export const COMMAND_DEFS_BY_ID: ReadonlyMap<string, CommandDef> = new Map(
  COMMAND_DEFS.map((def) => [def.id, def]),
);
