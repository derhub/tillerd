import { test, expect } from "bun:test";
import {
  createWorkspaceClient,
  WORKSPACE_CREATE,
  WORKSPACE_LIST,
  WORKSPACE_RENAME,
  WORKSPACE_REORDER,
  WORKSPACE_DELETE,
  PROJECT_MOVE,
  PROJECT_LIST,
  PROJECT_CREATE,
  type WorkspaceTransport,
  type Workspace,
  type Project,
} from "./workspace";

function fakeTransport(result: unknown = null) {
  const calls: { command: string; args?: Record<string, unknown> }[] = [];
  const transport: WorkspaceTransport = {
    invoke: async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      return result as T;
    },
  };
  return { transport, calls };
}

// ── workspace methods ─────────────────────────────────────────────────────────

test("createWorkspace routes to workspace_create with the name arg", async () => {
  const workspace: Workspace = { id: "ws-1", name: "Alpha" };
  const { transport, calls } = fakeTransport(workspace);
  const client = createWorkspaceClient(transport);

  const result = await client.createWorkspace({ name: "Alpha" });

  expect(calls).toEqual([{ command: WORKSPACE_CREATE, args: { name: "Alpha" } }]);
  expect(result).toEqual(workspace);
});

test("listWorkspaces routes to workspace_list with no args and returns the typed array", async () => {
  const workspaces: Workspace[] = [
    { id: "ws-1", name: "Alpha" },
    { id: "ws-2", name: "Beta" },
  ];
  const { transport, calls } = fakeTransport(workspaces);
  const client = createWorkspaceClient(transport);

  const result = await client.listWorkspaces();

  expect(calls).toEqual([{ command: WORKSPACE_LIST, args: undefined }]);
  expect(result).toEqual(workspaces);
});

test("renameWorkspace routes to workspace_rename with id and name", async () => {
  const { transport, calls } = fakeTransport(null);
  const client = createWorkspaceClient(transport);

  await client.renameWorkspace({ id: "ws-1", name: "Renamed" });

  expect(calls).toEqual([{ command: WORKSPACE_RENAME, args: { id: "ws-1", name: "Renamed" } }]);
});

test("reorderWorkspace routes to workspace_reorder with id and sortOrder", async () => {
  const { transport, calls } = fakeTransport(null);
  const client = createWorkspaceClient(transport);

  await client.reorderWorkspace({ id: "ws-1", sortOrder: 5 });

  expect(calls).toEqual([{ command: WORKSPACE_REORDER, args: { id: "ws-1", sortOrder: 5 } }]);
});

test("deleteWorkspace routes to workspace_delete with the id", async () => {
  const { transport, calls } = fakeTransport(null);
  const client = createWorkspaceClient(transport);

  await client.deleteWorkspace({ id: "ws-1" });

  expect(calls).toEqual([{ command: WORKSPACE_DELETE, args: { id: "ws-1" } }]);
});

// ── project scoping ───────────────────────────────────────────────────────────

test("listProjects passes an optional workspaceId through to project_list", async () => {
  const projects: Project[] = [];
  const { transport, calls } = fakeTransport(projects);
  const client = createWorkspaceClient(transport);

  await client.listProjects({ workspaceId: "ws-1" });

  expect(calls).toEqual([{ command: PROJECT_LIST, args: { workspaceId: "ws-1" } }]);
});

test("listProjects with no args still routes to project_list", async () => {
  const { transport, calls } = fakeTransport([]);
  const client = createWorkspaceClient(transport);

  await client.listProjects();

  expect(calls).toEqual([{ command: PROJECT_LIST, args: undefined }]);
});

test("createProject passes an optional workspaceId through to project_create", async () => {
  const project: Project = {
    id: "p-1",
    name: "My project",
    sourceKind: "blank",
    rootPath: null,
    workspaceId: "ws-1",
  };
  const { transport, calls } = fakeTransport(project);
  const client = createWorkspaceClient(transport);

  const result = await client.createProject({ sourceKind: "blank", workspaceId: "ws-1" });

  expect(calls).toEqual([
    { command: PROJECT_CREATE, args: { sourceKind: "blank", workspaceId: "ws-1" } },
  ]);
  expect(result).toEqual(project);
});

// ── moveProject ───────────────────────────────────────────────────────────────

test("moveProject routes to project_move with projectId and workspaceId", async () => {
  const { transport, calls } = fakeTransport(null);
  const client = createWorkspaceClient(transport);

  await client.moveProject({ id: "p-1", workspaceId: "ws-2" });

  expect(calls).toEqual([{ command: PROJECT_MOVE, args: { id: "p-1", workspaceId: "ws-2" } }]);
});
