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
  panelToggleLeft: "panel.toggle-left",
  panelToggleRight: "panel.toggle-right",
  panelToggleBottom: "panel.toggle-bottom",
  commandToggle: "command.toggle",
} as const;

export type ActionId = (typeof ACTION)[keyof typeof ACTION];

export const SESSION_SEARCH_ACTION_ID = "session.search";
export const SESSION_SEARCH_TITLE = "Search sessions";
