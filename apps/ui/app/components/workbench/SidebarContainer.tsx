import React from "react";

import { SessionSidebar } from "~/components/sidebar/SessionSidebar";
import { WorkspaceSwitcher } from "~/components/sidebar/WorkspaceSwitcher";
import { Skeleton } from "~/components/ui/skeleton";
import { useWorkbenchView } from "~/lib/workbench";

import { viewDef } from "./views";

// Hosts the active sidebar view. The `sessions` view keeps the exact project- vs
// workspace-window branch (and its Suspense boundary) from the previous shell; the
// other views render their registry component.
export function SidebarContainer({
  isProjectWindow,
  projectWindowId,
  workspaceWindowId,
}: {
  isProjectWindow: boolean;
  projectWindowId?: string;
  workspaceWindowId?: string;
}) {
  const [view] = useWorkbenchView();

  if (view === "sessions") {
    return (
      <React.Suspense
        fallback={
          <div className="h-full w-full p-3" data-testid="sidebar-skeleton">
            <Skeleton className="h-full w-full" />
          </div>
        }
      >
        {isProjectWindow ? (
          <SessionSidebar activeProjectId={projectWindowId} />
        ) : (
          <WorkspaceSwitcher initialWorkspaceId={workspaceWindowId} />
        )}
      </React.Suspense>
    );
  }

  const Body = viewDef(view).Component;
  return <Body />;
}
