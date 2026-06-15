/**
 * Stable action ids shared by the keybinding presets and the command registry. A static action is
 * bindable (appears in presets / overrides); dynamic palette entries (e.g. switch-to-session) use a
 * namespaced id like `session.switch:<sessionId>` and carry no keybinding.
 */
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

/** The static (bindable) action ids, in palette/editor order. */
export const STATIC_ACTION_IDS: readonly ActionId[] = Object.values(ACTION);

/** Human titles, shared by the registry command labels and the keybinding editor. */
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

/** Prefix for dynamic switch-to-session palette entries (unbindable). */
export const SESSION_SWITCH_PREFIX = "session.switch:";
