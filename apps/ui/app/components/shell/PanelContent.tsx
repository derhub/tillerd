import { useMutation } from "@tanstack/react-query";
import { command } from "@tillerd/client-bindings";
import React from "react";

import type { PanelLeaf } from "~/lib/panelTree";

import { useShellCommands } from "~/components/shell/hooks/useShellCommands";
import { PanelTree } from "~/components/shell/PanelTree";
import { DetachedPanelsContext } from "~/components/shell/shellContext";
import { TerminalPane } from "~/components/terminal/TerminalPane";
import { Skeleton } from "~/components/ui/skeleton";
import { RegisterCommands } from "~/lib/commands/registry";
import { bootContent } from "~/lib/health/boot-content";
import { countLeaves } from "~/lib/panelTree";
import { SessionContext } from "~/lib/sessionContext";
import { useDelayedTrue } from "~/lib/useDelayedTrue";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { usePanelTree } from "~/lib/usePanelTree";

export function PanelContent() {
  const { sessionId } = React.use(SessionContext);
  const { detached, detach, reattach } = React.use(DetachedPanelsContext);

  const surfaceSpawn = useMutation(command("surfaceSpawn"));
  const surfaceClose = useMutation(command("surfaceClose"));

  const host = useDesktopHost();
  const graceElapsed = useDelayedTrue(host.status === "booting", 200);
  const bootRegion = bootContent(host.status, graceElapsed);

  const { tree, split, close, setContent, setActiveTab } = usePanelTree(sessionId);
  const totalPanels = countLeaves(tree);

  const treeRef = React.useRef(tree);
  treeRef.current = tree;
  const detachedRef = React.useRef(detached);
  detachedRef.current = detached;
  const activeLeafRef = React.useRef<string | null>(null);

  const onContentPointerDown = React.useCallback((e: React.PointerEvent) => {
    const el = (e.target as HTMLElement).closest("[data-panel-id]");
    const id = el?.getAttribute("data-panel-id");
    if (id) activeLeafRef.current = id;
  }, []);

  const handleSpawn = React.useCallback(
    (leafId: string) => {
      if (!sessionId) return;
      surfaceSpawn.mutate(
        { sessionId },
        { onSuccess: (placement) => setContent(leafId, { type: "terminal", placement }) },
      );
    },
    [sessionId, setContent, surfaceSpawn],
  );

  const handleClose = React.useCallback(
    (leaf: PanelLeaf) => {
      if (leaf.content.type === "terminal" && sessionId) {
        surfaceClose.mutate({ id: leaf.content.placement });
      }
      close(leaf.id);
    },
    [sessionId, close, surfaceClose],
  );

  const panelCommands = useShellCommands({
    treeRef,
    activeLeafRef,
    detachedRef,
    split,
    spawn: handleSpawn,
    close: handleClose,
    detach,
  });

  if (host.status === "web") return <TerminalPane sessionId={sessionId} />;

  return (
    <div className="h-full w-full" onPointerDownCapture={onContentPointerDown}>
      <RegisterCommands commands={panelCommands} />
      {bootRegion === "content" ? (
        <PanelTree
          tree={tree}
          totalPanels={totalPanels}
          sessionId={sessionId}
          detached={detached}
          onSplit={split}
          onSetActiveTab={setActiveTab}
          onClose={handleClose}
          onSpawn={handleSpawn}
          onDetach={detach}
          onReattach={reattach}
        />
      ) : bootRegion === "skeleton" ? (
        <div className="h-full w-full p-3" data-testid="content-skeleton">
          <Skeleton className="h-full w-full" />
        </div>
      ) : null}
    </div>
  );
}
