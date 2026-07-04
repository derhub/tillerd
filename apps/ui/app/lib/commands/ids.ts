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
  projectDuplicate: "project.duplicate",
  projectPin: "project.pin",
  projectUnpin: "project.unpin",
  projectMove: "project.move",
  projectStopSurfaces: "project.stop-surfaces",
  projectArchive: "project.archive",
  projectDelete: "project.delete",
  sessionRename: "session.rename",
  sessionDuplicate: "session.duplicate",
  sessionPin: "session.pin",
  sessionUnpin: "session.unpin",
  sessionMove: "session.move",
  sessionStopSurfaces: "session.stop-surfaces",
  sessionArchive: "session.archive",
  sessionDelete: "session.delete",
  workspaceRename: "workspace.rename",
  workspacePin: "workspace.pin",
  workspaceUnpin: "workspace.unpin",
  workspaceStopSurfaces: "workspace.stop-surfaces",
  workspaceArchive: "workspace.archive",
  workspaceDelete: "workspace.delete",
  // -- command library row (see CommandsView) --
  commandEdit: "command.edit",
  commandRename: "command.rename",
  commandDuplicate: "command.duplicate",
  commandPin: "command.pin",
  commandUnpin: "command.unpin",
  commandDelete: "command.delete",
  // -- portable template library row (see TemplatesView) --
  templatePin: "template.pin",
  templateUnpin: "template.unpin",
  templateExport: "template.export",
  templateDelete: "template.delete",
  // -- project launch-template row (see TemplatesView) --
  launchTemplateEdit: "launch-template.edit",
  launchTemplateDiscard: "launch-template.discard",
} as const;

export type ActionId = (typeof ACTION)[keyof typeof ACTION];

export const SESSION_SEARCH_ACTION_ID = "session.search";
export const SESSION_SEARCH_TITLE = "Search sessions";
