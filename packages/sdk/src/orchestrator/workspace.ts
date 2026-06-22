// -- command names -------------------------------------------------------------

export const PROJECT_CREATE = "project_create";
export const PROJECT_RENAME = "project_rename";
export const PROJECT_LIST = "project_list";
export const PROJECT_GET = "project_get";
export const PROJECT_ARCHIVE = "project_archive";
export const PROJECT_DELETE = "project_delete";
export const PROJECT_REORDER = "project_reorder";
export const PROJECT_MOVE = "project_move";

export const SESSION_CREATE = "session_create";
export const SESSION_RENAME = "session_rename";
export const SESSION_LIST = "session_list";
export const SESSION_ARCHIVE = "session_archive";
export const SESSION_DELETE = "session_delete";
export const SESSION_REORDER = "session_reorder";
export const SESSION_LAYOUT_SET = "session_layout_set";
export const SESSION_LAYOUT_GET = "session_layout_get";

export const WORKSPACE_CREATE = "workspace_create";
export const WORKSPACE_LIST = "workspace_list";
export const WORKSPACE_RENAME = "workspace_rename";
export const WORKSPACE_REORDER = "workspace_reorder";
export const WORKSPACE_DELETE = "workspace_delete";

// -- domain types --------------------------------------------------------------

export type SourceKind = "blank" | "local_dir" | "git_repo";

export type TitleSource = "agent-title" | "branch" | "both" | "custom";

export interface Workspace {
  id: string;
  name: string;
}

export interface Project {
  id: string;
  name: string;
  sourceKind: SourceKind;
  rootPath: string | null;
  workspaceId: string;
}

export interface Session {
  id: string;
  projectId: string;
  title: string;
  titleSource: TitleSource;
  createdAt: string;
}

// -- request / response shapes -------------------------------------------------

export interface CreateProjectArgs {
  sourceKind: SourceKind;
  rootPath?: string;
  name?: string;
  workspaceId?: string;
}

export interface RenameProjectArgs {
  id: string;
  name: string;
}

export interface ArchiveProjectArgs {
  id: string;
}

export interface DeleteProjectArgs {
  id: string;
}

export interface ReorderProjectArgs {
  id: string;
  sortOrder: number;
}

export interface ListProjectsArgs {
  workspaceId?: string;
}

export interface GetProjectArgs {
  id: string;
}

export interface MoveProjectArgs {
  id: string;
  workspaceId: string;
}

export interface CreateSessionArgs {
  projectId?: string;
  titleSource?: TitleSource;
  title?: string;
}

export interface RenameSessionArgs {
  id: string;
  title: string;
}

export interface ListSessionsArgs {
  projectId?: string;
}

export interface ArchiveSessionArgs {
  id: string;
}

export interface DeleteSessionArgs {
  id: string;
}

export interface ReorderSessionArgs {
  id: string;
  sortOrder: number;
}

export interface SetSessionLayoutArgs {
  id: string;
  layoutJson: string;
}

export interface GetSessionLayoutArgs {
  id: string;
}

export interface CreateWorkspaceArgs {
  name: string;
}

export interface RenameWorkspaceArgs {
  id: string;
  name: string;
}

export interface ReorderWorkspaceArgs {
  id: string;
  sortOrder: number;
}

export interface DeleteWorkspaceArgs {
  id: string;
}

// -- client interface ----------------------------------------------------------

export interface WorkspaceClient {
  createProject(args: CreateProjectArgs): Promise<Project>;
  renameProject(args: RenameProjectArgs): Promise<void>;
  listProjects(args?: ListProjectsArgs): Promise<Project[]>;
  getProject(args: GetProjectArgs): Promise<Project | null>;
  archiveProject(args: ArchiveProjectArgs): Promise<void>;
  deleteProject(args: DeleteProjectArgs): Promise<void>;
  reorderProject(args: ReorderProjectArgs): Promise<void>;
  moveProject(args: MoveProjectArgs): Promise<void>;

  createSession(args: CreateSessionArgs): Promise<Session>;
  renameSession(args: RenameSessionArgs): Promise<void>;
  listSessions(args?: ListSessionsArgs): Promise<Session[]>;
  archiveSession(args: ArchiveSessionArgs): Promise<void>;
  deleteSession(args: DeleteSessionArgs): Promise<void>;
  reorderSession(args: ReorderSessionArgs): Promise<void>;

  setSessionLayout(args: SetSessionLayoutArgs): Promise<void>;
  getSessionLayout(args: GetSessionLayoutArgs): Promise<string | null>;

  createWorkspace(args: CreateWorkspaceArgs): Promise<Workspace>;
  listWorkspaces(): Promise<Workspace[]>;
  renameWorkspace(args: RenameWorkspaceArgs): Promise<void>;
  reorderWorkspace(args: ReorderWorkspaceArgs): Promise<void>;
  deleteWorkspace(args: DeleteWorkspaceArgs): Promise<void>;
}

export interface WorkspaceTransport {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
}

export function createWorkspaceClient(transport: WorkspaceTransport): WorkspaceClient {
  const call = <T>(cmd: string, args?: unknown) =>
    transport.invoke<T>(cmd, args as Record<string, unknown>);
  return {
    createProject: (args) => call<Project>(PROJECT_CREATE, args),
    renameProject: (args) => call<void>(PROJECT_RENAME, args),
    listProjects: (args) => call<Project[]>(PROJECT_LIST, args),
    getProject: (args) => call<Project | null>(PROJECT_GET, args),
    archiveProject: (args) => call<void>(PROJECT_ARCHIVE, args),
    deleteProject: (args) => call<void>(PROJECT_DELETE, args),
    reorderProject: (args) => call<void>(PROJECT_REORDER, args),
    moveProject: (args) => call<void>(PROJECT_MOVE, args),

    createSession: (args) => call<Session>(SESSION_CREATE, args),
    renameSession: (args) => call<void>(SESSION_RENAME, args),
    listSessions: (args) => call<Session[]>(SESSION_LIST, args),
    archiveSession: (args) => call<void>(SESSION_ARCHIVE, args),
    deleteSession: (args) => call<void>(SESSION_DELETE, args),
    reorderSession: (args) => call<void>(SESSION_REORDER, args),

    setSessionLayout: (args) => call<void>(SESSION_LAYOUT_SET, args),
    getSessionLayout: (args) => call<string | null>(SESSION_LAYOUT_GET, args),

    createWorkspace: (args) => call<Workspace>(WORKSPACE_CREATE, args),
    listWorkspaces: () => call<Workspace[]>(WORKSPACE_LIST),
    renameWorkspace: (args) => call<void>(WORKSPACE_RENAME, args),
    reorderWorkspace: (args) => call<void>(WORKSPACE_REORDER, args),
    deleteWorkspace: (args) => call<void>(WORKSPACE_DELETE, args),
  };
}
