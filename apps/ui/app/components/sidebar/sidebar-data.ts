import { useSuspenseQuery } from "@tanstack/react-query";
import { query } from "@tillerd/client-bindings";
import React from "react";

export const UNFILED_ID = "00000000-0000-0000-0000-000000000000";
export const DEFAULT_WORKSPACE_ID = "00000000-0000-0000-0000-000000000001";

export const DRAG_PROJECT = "application/x-tillerd-project";
export const DRAG_SESSION = "application/x-tillerd-session";

// Sessions are not fetched here -- each project loads its own page lazily on expand.
export function useSidebarData(activeWorkspaceId?: string, activeProjectId?: string) {
  const queryOpts = activeProjectId
    ? query("projectGet", { id: activeProjectId })
    : query("projectList", { workspaceId: activeWorkspaceId ?? null });

  const { data } = useSuspenseQuery(queryOpts as any);

  const projects = React.useMemo(() => {
    if (!data) return [];
    return Array.isArray(data) ? data : [data];
  }, [data]);

  return { projects };
}
