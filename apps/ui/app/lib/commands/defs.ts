// The single source of truth for UI command declarations: identity, titles,
// keywords, surfaces, default keybindings per preset, and availability. Handlers
// register by id at their feature sites (see registry `useCommand`). This table
// replaces the split between `ids.ts` titles and `keybindings.ts` presets.

import {
  Archive,
  ClipboardPaste,
  Command,
  Copy,
  Download,
  Eraser,
  ExternalLink,
  FolderInput,
  LayoutTemplate,
  MessagesSquare,
  PanelBottom,
  PanelLeft,
  Pencil,
  Pin,
  PinOff,
  Play,
  RotateCcw,
  Search,
  Square,
  TextSelect,
  Trash2,
  ZoomIn,
  ZoomOut,
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
  {
    id: ACTION.viewZoomIn,
    title: "Zoom in",
    icon: ZoomIn,
    keywords: ["zoom", "scale", "font size", "bigger"],
    defaultKeys: { default: "CmdOrCtrl+=" },
  },
  {
    id: ACTION.viewZoomOut,
    title: "Zoom out",
    icon: ZoomOut,
    keywords: ["zoom", "scale", "font size", "smaller"],
    defaultKeys: { default: "CmdOrCtrl+-" },
  },
  {
    id: ACTION.viewZoomReset,
    title: "Reset zoom",
    icon: RotateCcw,
    keywords: ["zoom", "scale", "font size", "reset"],
    defaultKeys: { default: "CmdOrCtrl+0" },
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
  // Find in terminal (ui-terminal-pane spec). The accelerator also fires from inside the pane:
  // useGlobalShortcuts skips events while `.xterm` holds focus, so the pane catches CmdOrCtrl+F
  // via xterm's key handler and the palette entry covers the unfocused case.
  {
    id: ACTION.terminalFind,
    title: "Find in terminal",
    icon: Search,
    keywords: ["find", "search", "terminal", "scrollback"],
    defaultKeys: { default: "CmdOrCtrl+F" },
  },
  // Row-scoped context-menu actions. `menu.<kind>Row` and the per-row guards
  // (`menu.canRename`, `menu.canDelete`, `menu.canArchive`, `menu.canMove`,
  // `menu.pinned`, ...) are pushed into the context store by EntityContextMenu
  // while a given row's menu is open (context.ts's setContext model) -- these defs
  // stay declarative and EntityContextMenu never special-cases a kind or id. Pin
  // and Unpin are one toggle rendered as two guarded defs so the row need only
  // publish its `menu.pinned` flag. Menu order here is the on-screen order; group
  // changes drive separators (primary | lifecycle | destructive).

  // -- project row --
  {
    id: ACTION.projectNewSessionFromTemplate,
    title: "New session from template...",
    icon: LayoutTemplate,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.projectRow"],
  },
  {
    id: ACTION.projectRename,
    title: "Rename",
    icon: Pencil,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.projectRow", "menu.canRename"],
  },
  {
    id: ACTION.projectDuplicate,
    title: "Duplicate",
    icon: Copy,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.projectRow", "menu.canDuplicate"],
  },
  {
    id: ACTION.projectPin,
    title: "Pin",
    icon: Pin,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.projectRow", "menu.canPin", "!menu.pinned"],
  },
  {
    id: ACTION.projectUnpin,
    title: "Unpin",
    icon: PinOff,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.projectRow", "menu.canPin", "menu.pinned"],
  },
  {
    id: ACTION.projectMove,
    title: "Move to workspace",
    icon: FolderInput,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.projectRow", "menu.canMove"],
  },
  {
    id: ACTION.projectStopSurfaces,
    title: "Stop surfaces",
    icon: Square,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.projectRow"],
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
    id: ACTION.projectArchive,
    title: "Archive",
    icon: Archive,
    surfaces: ["contextmenu"],
    group: "lifecycle",
    when: ["menu.projectRow", "menu.canArchive"],
  },
  {
    id: ACTION.projectDelete,
    title: "Delete",
    icon: Trash2,
    surfaces: ["contextmenu"],
    group: "destructive",
    when: ["menu.projectRow", "menu.canDelete"],
  },

  // -- session row --
  {
    id: ACTION.sessionRename,
    title: "Rename",
    icon: Pencil,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.sessionRow"],
  },
  {
    id: ACTION.sessionDuplicate,
    title: "Duplicate",
    icon: Copy,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.sessionRow"],
  },
  {
    id: ACTION.sessionPin,
    title: "Pin",
    icon: Pin,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.sessionRow", "!menu.pinned"],
  },
  {
    id: ACTION.sessionUnpin,
    title: "Unpin",
    icon: PinOff,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.sessionRow", "menu.pinned"],
  },
  {
    id: ACTION.sessionMove,
    title: "Move to project",
    icon: FolderInput,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.sessionRow"],
  },
  {
    id: ACTION.sessionStopSurfaces,
    title: "Stop surfaces",
    icon: Square,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.sessionRow"],
  },
  {
    id: ACTION.sessionArchive,
    title: "Archive",
    icon: Archive,
    surfaces: ["contextmenu"],
    group: "lifecycle",
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

  // -- workspace row (sessions-view switcher) --
  {
    id: ACTION.workspaceRename,
    title: "Rename",
    icon: Pencil,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.workspaceRow"],
  },
  {
    id: ACTION.workspacePin,
    title: "Pin",
    icon: Pin,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.workspaceRow", "!menu.pinned"],
  },
  {
    id: ACTION.workspaceUnpin,
    title: "Unpin",
    icon: PinOff,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.workspaceRow", "menu.pinned"],
  },
  {
    id: ACTION.workspaceStopSurfaces,
    title: "Stop surfaces",
    icon: Square,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.workspaceRow"],
  },
  {
    id: ACTION.workspaceArchive,
    title: "Archive",
    icon: Archive,
    surfaces: ["contextmenu"],
    group: "lifecycle",
    when: ["menu.workspaceRow", "menu.canArchive"],
  },
  {
    id: ACTION.workspaceDelete,
    title: "Delete",
    icon: Trash2,
    surfaces: ["contextmenu"],
    group: "destructive",
    when: ["menu.workspaceRow", "menu.canDelete"],
  },

  // -- command library row -- `menu.canEdit` is false for prebuilt rows, gating
  // Edit/Rename/Delete off; Run, Duplicate, and Pin/Unpin stay available on every origin.
  {
    id: ACTION.commandRun,
    title: "Run",
    icon: Play,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.commandRow"],
  },
  {
    id: ACTION.commandEdit,
    title: "Edit",
    icon: Pencil,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.commandRow", "menu.canEdit"],
  },
  {
    id: ACTION.commandRename,
    title: "Rename",
    icon: Pencil,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.commandRow", "menu.canEdit"],
  },
  {
    id: ACTION.commandDuplicate,
    title: "Duplicate",
    icon: Copy,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.commandRow"],
  },
  {
    id: ACTION.commandPin,
    title: "Pin",
    icon: Pin,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.commandRow", "!menu.pinned"],
  },
  {
    id: ACTION.commandUnpin,
    title: "Unpin",
    icon: PinOff,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.commandRow", "menu.pinned"],
  },
  {
    id: ACTION.commandDelete,
    title: "Delete",
    icon: Trash2,
    surfaces: ["contextmenu"],
    group: "destructive",
    when: ["menu.commandRow", "menu.canEdit"],
  },

  // -- portable template library row -- prebuilt rows guard off Delete only
  // (`menu.canEdit`); Pin/Unpin/Export stay available on every origin.
  {
    id: ACTION.templatePin,
    title: "Pin",
    icon: Pin,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.templateRow", "!menu.pinned"],
  },
  {
    id: ACTION.templateUnpin,
    title: "Unpin",
    icon: PinOff,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.templateRow", "menu.pinned"],
  },
  {
    id: ACTION.templateExport,
    title: "Export",
    icon: Download,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.templateRow"],
  },
  {
    id: ACTION.templateDelete,
    title: "Delete",
    icon: Trash2,
    surfaces: ["contextmenu"],
    group: "destructive",
    when: ["menu.templateRow", "menu.canEdit"],
  },

  // -- project launch-template row -- unlike library templates these carry no
  // name/origin (no prebuilt guard needed) and have no pin/export operation.
  {
    id: ACTION.launchTemplateEdit,
    title: "Edit",
    icon: Pencil,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.launchTemplateRow"],
  },
  {
    id: ACTION.launchTemplateDiscard,
    title: "Discard",
    icon: Trash2,
    surfaces: ["contextmenu"],
    group: "destructive",
    when: ["menu.launchTemplateRow"],
  },

  // -- terminal pane context menu (ui-terminal-pane spec). Scoped by `menu.terminalRow`; the
  // registered handlers act on the focused terminal via activeTerminalStore. Copy and
  // "Search selection" only apply with a live selection, gated by `menu.hasSelection`.
  {
    id: ACTION.terminalCopy,
    title: "Copy",
    icon: Copy,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.terminalRow", "menu.hasSelection"],
  },
  {
    id: ACTION.terminalPaste,
    title: "Paste",
    icon: ClipboardPaste,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.terminalRow"],
  },
  {
    id: ACTION.terminalSelectAll,
    title: "Select all",
    icon: TextSelect,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.terminalRow"],
  },
  {
    id: ACTION.terminalSearchSelection,
    title: "Search selection",
    icon: Search,
    surfaces: ["contextmenu"],
    group: "primary",
    when: ["menu.terminalRow", "menu.hasSelection"],
  },
  {
    id: ACTION.terminalClear,
    title: "Clear",
    icon: Eraser,
    surfaces: ["contextmenu"],
    group: "lifecycle",
    when: ["menu.terminalRow"],
  },
];

export const COMMAND_DEFS_BY_ID: ReadonlyMap<string, CommandDef> = new Map(
  COMMAND_DEFS.map((def) => [def.id, def]),
);
