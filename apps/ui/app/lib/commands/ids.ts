export const ACTION = {
  projectNew: "project.new",
  sessionNew: "session.new",
  surfaceSpawn: "surface.spawn",
  surfaceClose: "surface.close",
  panelSplitH: "panel.split-h",
  panelSplitV: "panel.split-v",
  surfaceDetach: "surface.detach",
  projectOpenNewWindow: "project.open-new-window",
  viewLogs: "view.logs",
  appSettings: "app.settings",
} as const;

export type ActionId = (typeof ACTION)[keyof typeof ACTION];

export const STATIC_ACTION_IDS: readonly ActionId[] = Object.values(ACTION);

export const ACTION_TITLES: Record<ActionId, string> = {
  [ACTION.projectNew]: "New project",
  [ACTION.sessionNew]: "New session",
  [ACTION.surfaceSpawn]: "New terminal",
  [ACTION.surfaceClose]: "Close panel",
  [ACTION.panelSplitH]: "Split panel right",
  [ACTION.panelSplitV]: "Split panel down",
  [ACTION.surfaceDetach]: "Detach panel",
  [ACTION.projectOpenNewWindow]: "Open project in new window",
  [ACTION.viewLogs]: "View logs",
  [ACTION.appSettings]: "Settings",
};

export const SESSION_SEARCH_ACTION_ID = "session.search";
export const SESSION_SEARCH_TITLE = "Search sessions";
