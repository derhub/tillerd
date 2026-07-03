// The single source of truth for UI command declarations: identity, titles,
// keywords, surfaces, default keybindings per preset, and availability. Handlers
// register by id at their feature sites (see registry `useCommand`). This table
// replaces the split between `ids.ts` titles and `keybindings.ts` presets.

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
    id: SESSION_SEARCH_ACTION_ID,
    title: SESSION_SEARCH_TITLE,
    keywords: ["session", "switch", "go to", "find", "search"],
  },
];

export const COMMAND_DEFS_BY_ID: ReadonlyMap<string, CommandDef> = new Map(
  COMMAND_DEFS.map((def) => [def.id, def]),
);
