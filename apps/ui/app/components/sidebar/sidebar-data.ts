import { useSuspenseQuery } from "@tanstack/react-query";
import { query } from "@tillerd/client-bindings";
import React from "react";

import { WELL_KNOWN_IDS } from "~/lib/stateModel";

// Single source: the contract-tested state-model mirror owns the well-known ids.
export const UNFILED_ID = WELL_KNOWN_IDS.unfiledProject;
export const DEFAULT_WORKSPACE_ID = WELL_KNOWN_IDS.defaultWorkspace;

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
