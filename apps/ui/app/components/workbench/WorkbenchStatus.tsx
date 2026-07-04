import { useQuery } from "@tanstack/react-query";
import { query } from "@tillerd/client-bindings";
import React from "react";

import { SessionContext } from "~/lib/sessionContext";
import { useActiveWorkspace } from "~/lib/store";
import { useDesktopHost } from "~/lib/useDesktopHost";

// Plain (non-suspense) reads: an unresolved title renders nothing rather than
// suspending the whole shell.
export function WorkbenchStatus() {
  const host = useDesktopHost();
  const { sessionId } = React.use(SessionContext);
  const activeWorkspaceId = useActiveWorkspace();
  const ready = host.status === "ready";

  const { data: workspaces } = useQuery({ ...query("workspaceList"), enabled: ready });
  const { data: session } = useQuery({
    ...query("sessionGet", { id: sessionId ?? "" }),
    enabled: ready && Boolean(sessionId),
  });

  const workspaceName = workspaces?.find((w) => w.id === activeWorkspaceId)?.name;
  const sessionTitle = sessionId ? session?.title : undefined;

  if (!workspaceName && !sessionTitle) return null;

  return (
    <div className="flex min-w-0 items-center gap-1.5 text-[0.75rem] text-muted-foreground select-none">
      {workspaceName ? <span className="truncate max-w-[16ch]">{workspaceName}</span> : null}
      {workspaceName && sessionTitle ? (
        <span aria-hidden className="text-muted-foreground/50">
          /
        </span>
      ) : null}
      {sessionTitle ? (
        <span className="truncate max-w-[24ch] text-foreground/80">{sessionTitle}</span>
      ) : null}
    </div>
  );
}
