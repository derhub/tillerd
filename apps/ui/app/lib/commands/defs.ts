// The single source of truth for UI command declarations: identity, titles,
// keywords, surfaces, default keybindings per preset, and availability. Handlers
// register by id at their feature sites (see registry `useCommand`). This table
// replaces the split between `ids.ts` titles and `keybindings.ts` presets.

import { ACTION, SESSION_SEARCH_ACTION_ID, SESSION_SEARCH_TITLE } from "./ids";
import type { CommandDef } from "./types";

const IN_SESSION = ["hasActiveSession"] as const;

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
    when: IN_SESSION,
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
    when: IN_SESSION,
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
    when: IN_SESSION,
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
    when: IN_SESSION,
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
    when: IN_SESSION,
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
