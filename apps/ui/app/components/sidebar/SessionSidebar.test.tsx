import type { Project, Session } from "@tillerd/client-bindings";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createRootRoute,
  createRouter,
  createMemoryHistory,
} from "@tanstack/react-router";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { setReady } from "@tillerd/client-bindings";
import { setQueryClient } from "@tillerd/client-bindings";
/// <reference lib="dom" />
import { afterEach, expect, test, describe, mock } from "bun:test";
import React, { type ReactNode } from "react";

import { CommandRegistryProvider } from "~/lib/commands/registry";
import { resetUiStore, setProjectExpanded } from "~/lib/store";

// Renders real useSidebarData/ProjectRow through the data layer (mocked invoke + Query cache),
// inside a real router + Suspense boundary so no global mocks leak into sibling test files.

void mock.module("~/lib/useDesktopHost", () => ({
  useDesktopHost: () => ({ status: "ready" }),
}));

let fakeProjects: Project[] = [];
let fakeSessions: Session[] = [];
const sessionListedFor: string[] = [];

// Bindings call: typedError(invoke(cmd, args))
// typedError wraps the Promise<T> with { status: "ok", data: T } on success.
// So invoke must return the RAW data (not a typedError shape).
void mock.module("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "project_list") {
      const wsId = args?.["workspaceId"] as string | null;
      return wsId ? fakeProjects.filter((p) => p.workspaceId === wsId) : fakeProjects;
    }
    if (cmd === "project_get") {
      const id = args?.["id"] as string;
      return fakeProjects.find((p) => p.id === id) ?? null;
    }
    if (cmd === "session_list") {
      const projectId = args?.["projectId"] as string | null;
      const offset = (args?.["offset"] as number | null) ?? 0;
      if (projectId) sessionListedFor.push(projectId);
      if (offset) return [];
      return projectId ? fakeSessions.filter((s) => s.projectId === projectId) : fakeSessions;
    }
    return [];
  },
  Channel: class Channel {
    onmessage: ((v: unknown) => void) | null = null;
  },
}));

const { SessionSidebar } = await import("./SessionSidebar");

function project(id: string, name: string, workspaceId: string): Project {
  return { id, name, sourceKind: "blank", rootPath: null, workspaceId };
}
function session(id: string, projectId: string, title: string): Session {
  return { id, projectId, title, titleSource: "agent-title", createdAt: "" };
}

function setTreeData(projects: Project[], sessions: Session[]) {
  fakeProjects = projects;
  fakeSessions = sessions;
  sessionListedFor.length = 0;
}

function renderSidebar(node: ReactNode) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  setQueryClient(queryClient);
  setReady(true);
  const rootRoute = createRootRoute({
    component: () => (
      <QueryClientProvider client={queryClient}>
        <CommandRegistryProvider>
          <React.Suspense fallback={<div data-testid="sidebar-skeleton" />}>{node}</React.Suspense>
        </CommandRegistryProvider>
      </QueryClientProvider>
    ),
  });
  const router = createRouter({
    routeTree: rootRoute,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  return render(<RouterProvider router={router as never} />);
}

afterEach(() => {
  cleanup();
  fakeProjects = [];
  fakeSessions = [];
  sessionListedFor.length = 0;
  setReady(false);
  resetUiStore();
});

describe("workspace scoping", () => {
  test("projects not in the active workspace do not appear", async () => {
    setTreeData(
      [project("p-1", "In WS-A", "ws-a"), project("p-2", "In WS-B", "ws-b")],
      [session("s-1", "p-1", "S1")],
    );

    renderSidebar(<SessionSidebar activeWorkspaceId="ws-a" />);

    await waitFor(() => expect(screen.queryByText("In WS-A")).not.toBeNull());
    expect(screen.queryByText("In WS-B")).toBeNull();
  });

  test("without an active workspace all projects are listed", async () => {
    setTreeData(
      [project("p-1", "In WS-A", "ws-a"), project("p-2", "In WS-B", "ws-b")],
      [session("s-1", "p-1", "S1"), session("s-2", "p-2", "S2")],
    );

    renderSidebar(<SessionSidebar />);

    await waitFor(() => expect(screen.queryByText("In WS-A")).not.toBeNull());
    expect(screen.queryByText("In WS-B")).not.toBeNull();
  });
});

describe("project scoping (a project child window)", () => {
  test("only the scoped project appears", async () => {
    setTreeData(
      [project("p-1", "Scoped Project", "ws-a"), project("p-2", "Other Project", "ws-a")],
      [session("s-1", "p-1", "S1")],
    );

    renderSidebar(<SessionSidebar activeProjectId="p-1" />);

    await waitFor(() => expect(screen.queryByText("Scoped Project")).not.toBeNull());
    expect(screen.queryByText("Other Project")).toBeNull();
  });

  test("the scoped project expands and loads its sessions on mount", async () => {
    setTreeData([project("p-1", "Scoped Project", "ws-a")], [session("s-1", "p-1", "Hello S1")]);

    renderSidebar(<SessionSidebar activeProjectId="p-1" />);

    await waitFor(() => expect(screen.queryByText("Hello S1")).not.toBeNull());
  });
});

describe("guarded actions (stateModel mirror)", () => {
  const UNFILED_ID = "00000000-0000-0000-0000-000000000000";

  function projectRow(name: string): HTMLElement {
    const label = screen.queryByText(name);
    if (!label) throw new Error(`no row for ${name}`);
    return label;
  }

  test("the Unfiled project's context menu offers no Delete", async () => {
    setTreeData([project(UNFILED_ID, "Unfiled", "ws-a")], []);

    renderSidebar(<SessionSidebar />);
    await waitFor(() => expect(screen.queryByText("Unfiled")).not.toBeNull());

    fireEvent.contextMenu(projectRow("Unfiled"));

    await waitFor(() => expect(screen.queryByText("Open in new window")).not.toBeNull());
    expect(screen.queryByText("Delete")).toBeNull();
  });

  test("an ordinary project's context menu offers Delete", async () => {
    setTreeData([project("p-1", "Ordinary", "ws-a")], []);

    renderSidebar(<SessionSidebar />);
    await waitFor(() => expect(screen.queryByText("Ordinary")).not.toBeNull());

    fireEvent.contextMenu(projectRow("Ordinary"));

    await waitFor(() => expect(screen.queryByText("Delete")).not.toBeNull());
  });
});

describe("lazy per-project session loading", () => {
  function expandToggle(projectId: string): HTMLElement {
    const toggle = document.querySelector(
      `[data-testid="project-expand"][data-project-id="${projectId}"]`,
    );
    if (!toggle) throw new Error(`no expand toggle for ${projectId}`);
    return toggle as HTMLElement;
  }

  // Groups default to expanded (spec: default expanded); collapse explicitly to
  // assert the collapsed group mounts no session read.
  test("a collapsed project in the main window fetches no sessions", async () => {
    setTreeData([project("p-1", "Project One", "ws-a")], [session("s-1", "p-1", "Hidden S1")]);
    setProjectExpanded("p-1", false);

    renderSidebar(<SessionSidebar />);

    await waitFor(() => expect(screen.queryByText("Project One")).not.toBeNull());
    expect(screen.queryByText("Hidden S1")).toBeNull();
    expect(sessionListedFor).not.toContain("p-1");
  });

  test("expanding a project loads and renders its first page", async () => {
    setTreeData([project("p-1", "Project One", "ws-a")], [session("s-1", "p-1", "Revealed S1")]);
    setProjectExpanded("p-1", false);

    renderSidebar(<SessionSidebar />);
    await waitFor(() => expect(screen.queryByText("Project One")).not.toBeNull());
    expect(screen.queryByText("Revealed S1")).toBeNull();

    fireEvent.click(expandToggle("p-1"));

    await waitFor(() => expect(screen.queryByText("Revealed S1")).not.toBeNull());
    expect(sessionListedFor).toContain("p-1");
  });
});
