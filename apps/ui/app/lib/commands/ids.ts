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
  // Sidebar view-switch actions (activity bar + palette). Selecting the active
  // view toggles the sidebar (see useWorkbenchCommands).
  viewSessions: "view.sessions",
  viewSearch: "view.search",
  viewCommands: "view.commands",
  viewTemplates: "view.templates",
  panelToggleLeft: "panel.toggle-left",
  panelToggleBottom: "panel.toggle-bottom",
  commandToggle: "command.toggle",
  // Row-scoped context-menu actions. Distinct from `projectOpenNewWindow`
  // (palette/keyboard, acts on the active project) because a def's `when` gates
  // every surface it's tagged for -- a shared id/def would either hide the
  // shortcut outside an open menu or require a second def with the same id,
  // which corrupts the id-keyed maps `useGlobalShortcuts`/`COMMAND_DEFS_BY_ID`
  // build (last entry silently wins). See EntityContextMenu.
  projectOpenNewWindowRow: "project.open-new-window-row",
  projectRename: "project.rename",
  projectDelete: "project.delete",
  sessionRename: "session.rename",
  sessionArchive: "session.archive",
  sessionDelete: "session.delete",
} as const;

export type ActionId = (typeof ACTION)[keyof typeof ACTION];

export const SESSION_SEARCH_ACTION_ID = "session.search";
export const SESSION_SEARCH_TITLE = "Search sessions";
