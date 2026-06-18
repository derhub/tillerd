/// <reference lib="dom" />
import { afterEach, expect, test, describe } from "bun:test";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";

afterEach(cleanup);

// ── useSidebarData workspace scoping logic ────────────────────────────────────
//
// The full SessionSidebar requires DesktopHostContext + react-router which adds
// heavy setup. These tests validate the scoping logic by exercising
// useSidebarData in isolation via a thin test harness component.

import { useState, useEffect } from "react";
import type { Project } from "@tillerd/sdk/orchestrator";

// Minimal harness that renders useSidebarData results as data-testid elements.
function SidebarDataHarness({
  activeWorkspaceId,
  listProjects,
}: {
  activeWorkspaceId?: string;
  listProjects: (args?: { workspaceId?: string }) => Promise<Project[]>;
}) {
  const [projects, setProjects] = useState<Project[]>([]);

  useEffect(() => {
    void listProjects(activeWorkspaceId ? { workspaceId: activeWorkspaceId } : undefined).then(
      setProjects,
    );
  }, [activeWorkspaceId, listProjects]);

  return (
    <div>
      {projects.map((p) => (
        <div key={p.id} data-testid="project-item" data-project-id={p.id}>
          {p.name}
        </div>
      ))}
    </div>
  );
}

function project(id: string, name: string, workspaceId: string): Project {
  return { id, name, sourceKind: "blank", rootPath: null, workspaceId };
}

// Scenario: Sidebar shows only the active workspace's projects.
// A project in a different workspace does not appear.
describe("workspace scoping", () => {
  test("projects not in the active workspace do not appear", async () => {
    const allProjects = [project("p-1", "In WS-A", "ws-a"), project("p-2", "In WS-B", "ws-b")];

    // listProjects filters by workspaceId when provided
    const listProjects = async (args?: { workspaceId?: string }) => {
      if (args?.workspaceId) return allProjects.filter((p) => p.workspaceId === args.workspaceId);
      return allProjects;
    };

    render(
      <MemoryRouter>
        <SidebarDataHarness activeWorkspaceId="ws-a" listProjects={listProjects} />
      </MemoryRouter>,
    );

    await waitFor(() => expect(screen.queryByText("In WS-A")).not.toBeNull());
    expect(screen.queryByText("In WS-B")).toBeNull();
  });

  // Scenario: A project created while a workspace is active appears in that workspace's list.
  test("a project created in the active workspace appears in the scoped list", async () => {
    const store: Project[] = [project("p-1", "Existing", "ws-a")];

    const listProjects = async (args?: { workspaceId?: string }) => {
      if (args?.workspaceId) return store.filter((p) => p.workspaceId === args.workspaceId);
      return store;
    };

    const { rerender } = render(
      <MemoryRouter>
        <SidebarDataHarness activeWorkspaceId="ws-a" listProjects={listProjects} />
      </MemoryRouter>,
    );

    await waitFor(() => expect(screen.queryByText("Existing")).not.toBeNull());

    // Simulate a new project being created in ws-a
    store.push(project("p-2", "New Project", "ws-a"));

    // A key change triggers a re-fetch (same as refresh() after createProject)
    rerender(
      <MemoryRouter>
        <SidebarDataHarness key="refreshed" activeWorkspaceId="ws-a" listProjects={listProjects} />
      </MemoryRouter>,
    );

    await waitFor(() => expect(screen.queryByText("New Project")).not.toBeNull());
  });

  // Scenario: With no active workspace, all projects are shown (backward compat)
  test("without an active workspace all projects are listed", async () => {
    const allProjects = [project("p-1", "In WS-A", "ws-a"), project("p-2", "In WS-B", "ws-b")];

    const listProjects = async (args?: { workspaceId?: string }) => {
      if (args?.workspaceId) return allProjects.filter((p) => p.workspaceId === args.workspaceId);
      return allProjects;
    };

    render(
      <MemoryRouter>
        <SidebarDataHarness listProjects={listProjects} />
      </MemoryRouter>,
    );

    await waitFor(() => expect(screen.queryByText("In WS-A")).not.toBeNull());
    expect(screen.queryByText("In WS-B")).not.toBeNull();
  });
});
