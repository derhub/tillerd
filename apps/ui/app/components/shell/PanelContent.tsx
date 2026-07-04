import type { SpawnCommandRef } from "@tillerd/client-bindings";

import { useMutation, useQuery } from "@tanstack/react-query";
import { command, query } from "@tillerd/client-bindings";
import React from "react";

import { useShellCommands } from "~/components/shell/hooks/useShellCommands";
import { PanelTree } from "~/components/shell/PanelTree";
import { DetachedPanelsContext } from "~/components/shell/shellContext";
import { TerminalPane } from "~/components/terminal/TerminalPane";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogTitle,
} from "~/components/ui/alert-dialog";
import { Checkbox } from "~/components/ui/checkbox";
import { Skeleton } from "~/components/ui/skeleton";
import { RegisterHandlers } from "~/lib/commands/registry";
import { bootContent } from "~/lib/health/boot-content";
import { type PanelLeaf, findLeaf, shouldConfirmClose } from "~/lib/panelTree";
import { countLeaves } from "~/lib/panelTree";
import { SessionContext } from "~/lib/sessionContext";
import { useBoolGlobalSetting } from "~/lib/settings/context";
import { PANEL_CLOSE_CONFIRM_SKIP_KEY } from "~/lib/settings/keys";
import { useDelayedTrue } from "~/lib/useDelayedTrue";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { usePanelTree } from "~/lib/usePanelTree";

// Mirrors --motion-fast (app.css): the closing leaf must finish its fade-out before the caller
// actually drops it from the tree, or the removal is an instant cut instead of a fade.
const CLOSE_FADE_MS = 100;

export function PanelContent() {
  const { sessionId } = React.use(SessionContext);
  const { detached, detach, reattach } = React.use(DetachedPanelsContext);

  const surfaceSpawn = useMutation(command("surfaceSpawn"));
  const surfaceClose = useMutation(command("surfaceClose"));
  const surfaceSwapPlacement = useMutation(command("surfaceSwapPlacement"));

  const host = useDesktopHost();
  const ready = host.status === "ready";
  const { data: session } = useQuery({
    ...query("sessionGet", { id: sessionId ?? "" }),
    enabled: ready && Boolean(sessionId),
  });
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
    (leafId: string, commandRef?: SpawnCommandRef) => {
      if (!sessionId) return;
      surfaceSpawn.mutate(
        { sessionId, command: commandRef ?? null },
        { onSuccess: (placement) => setContent(leafId, { type: "terminal", placement }) },
      );
    },
    [sessionId, setContent, surfaceSpawn],
  );

  // Lifecycle motion (ui-panel-compound "Panel lifecycle motion"): a leaf marked closing keeps
  // rendering (fading to opacity 0 via Panel.Frame) for one fade cadence before it actually leaves
  // the tree, so destroy fades at the same rate as create instead of cutting instantly.
  const [closingLeafIds, setClosingLeafIds] = React.useState<Set<string>>(new Set());

  const runClose = React.useCallback(
    (leaf: PanelLeaf) => {
      if (leaf.content.type === "terminal" && sessionId) {
        surfaceClose.mutate({ id: leaf.content.placement });
      }
      setClosingLeafIds((prev) => new Set(prev).add(leaf.id));
      window.setTimeout(() => {
        close(leaf.id);
        setClosingLeafIds((prev) => {
          if (!prev.has(leaf.id)) return prev;
          const next = new Set(prev);
          next.delete(leaf.id);
          return next;
        });
      }, CLOSE_FADE_MS);
    },
    [sessionId, close, surfaceClose],
  );

  // Close-surface confirmation (ui-panel-compound spec): a running terminal prompts before its
  // PTY is terminated, unless "don't ask again" is set. Non-terminal (empty) leaves close at once.
  const { value: skipCloseConfirm, setValue: setSkipCloseConfirm } = useBoolGlobalSetting(
    PANEL_CLOSE_CONFIRM_SKIP_KEY,
    false,
  );
  const [pendingClose, setPendingClose] = React.useState<PanelLeaf | null>(null);
  const [dontAskAgain, setDontAskAgain] = React.useState(false);

  const handleClose = React.useCallback(
    (leaf: PanelLeaf) => {
      if (shouldConfirmClose(leaf, skipCloseConfirm)) {
        setPendingClose(leaf);
        setDontAskAgain(false);
        return;
      }
      runClose(leaf);
    },
    [skipCloseConfirm, runClose],
  );

  const confirmClose = React.useCallback(() => {
    if (!pendingClose) return;
    if (dontAskAgain) setSkipCloseConfirm(true);
    runClose(pendingClose);
    setPendingClose(null);
  }, [pendingClose, dontAskAgain, setSkipCloseConfirm, runClose]);

  // Placement swap (panel-placement-swap spec): geometry is unchanged, only which surface backs
  // each placement swaps server-side. The two affected DesktopTerminalPane instances must reattach
  // their data channel to the (possibly new) surface behind their placement without a full remount
  // -- reloadEpoch is that reconnect signal (see DesktopTerminalPane's split bind/rebind effects).
  const [reloadEpoch, setReloadEpoch] = React.useState<Record<string, number>>({});
  const bumpReloadEpoch = React.useCallback((placements: string[]) => {
    setReloadEpoch((prev) => {
      const next = { ...prev };
      for (const p of placements) next[p] = (next[p] ?? 0) + 1;
      return next;
    });
  }, []);

  const handleSwapPlacements = React.useCallback(
    (sourceLeafId: string, targetLeafId: string) => {
      if (!sessionId) return;
      const source = findLeaf(treeRef.current, sourceLeafId);
      const target = findLeaf(treeRef.current, targetLeafId);
      if (!source || !target) return;
      if (source.content.type !== "terminal" || target.content.type !== "terminal") return;
      const placementA = source.content.placement;
      const placementB = target.content.placement;
      surfaceSwapPlacement.mutate(
        { session: sessionId, placementA, placementB },
        { onSuccess: () => bumpReloadEpoch([placementA, placementB]) },
      );
    },
    [sessionId, surfaceSwapPlacement, bumpReloadEpoch],
  );

  const panelHandlers = useShellCommands({
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
      <RegisterHandlers handlers={panelHandlers} />
      {bootRegion === "content" ? (
        <PanelTree
          tree={tree}
          totalPanels={totalPanels}
          sessionId={sessionId}
          sessionTitle={session?.title}
          detached={detached}
          closingLeafIds={closingLeafIds}
          reloadEpoch={reloadEpoch}
          onSplit={split}
          onSetActiveTab={setActiveTab}
          onClose={handleClose}
          onSpawn={handleSpawn}
          onDetach={detach}
          onReattach={reattach}
          onSwapPlacements={handleSwapPlacements}
        />
      ) : bootRegion === "skeleton" ? (
        <div className="h-full w-full p-3" data-testid="content-skeleton">
          <Skeleton className="h-full w-full" />
        </div>
      ) : null}
      <AlertDialog
        open={pendingClose !== null}
        onOpenChange={(open) => !open && setPendingClose(null)}
      >
        <AlertDialogContent data-testid="close-confirm-dialog">
          <AlertDialogTitle>Close terminal?</AlertDialogTitle>
          <AlertDialogDescription>
            This terminates the running process. Unsaved output in this terminal cannot be
            recovered.
          </AlertDialogDescription>
          <label
            className="flex items-center gap-2 text-[0.833rem] text-muted-foreground select-none"
            data-testid="close-confirm-dont-ask"
          >
            <Checkbox
              checked={dontAskAgain}
              onCheckedChange={(checked) => setDontAskAgain(checked === true)}
            />
            Don&apos;t ask again
          </label>
          <div className="flex justify-end gap-2">
            <AlertDialogCancel onClick={() => setPendingClose(null)}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              data-testid="close-confirm-confirm"
              className="bg-destructive hover:bg-destructive/90"
              onClick={confirmClose}
            >
              Close & terminate
            </AlertDialogAction>
          </div>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
