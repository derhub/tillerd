import { useSuspenseQuery } from "@tanstack/react-query";
import { query } from "@tillerd/client-bindings";
import React from "react";

export const UNFILED_ID = "00000000-0000-0000-0000-000000000000";
export const DEFAULT_WORKSPACE_ID = "00000000-0000-0000-0000-000000000001";

export const DRAG_PROJECT = "application/x-tillerd-project";
export const DRAG_SESSION = "application/x-tillerd-session";

// Sessions are not fetched here -- each project loads its own page lazily on expand.
export function useSidebarData(activeWorkspaceId?: string, activeProjectId?: string) {
  const { data: allProjects } = useSuspenseQuery(query("projectList", { workspaceId: null }));

  // Stable reference required -- downstream command builders re-register on every new array, causing an infinite loop.
  const projects = React.useMemo(
    () =>
      activeProjectId
        ? allProjects.filter((p) => p.id === activeProjectId)
        : activeWorkspaceId
          ? allProjects.filter((p) => p.workspaceId === activeWorkspaceId)
          : allProjects,
    [allProjects, activeWorkspaceId, activeProjectId],
  );

  return { projects };
}
