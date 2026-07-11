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
import { type PanelLeaf, collectLeaves, findLeaf, shouldConfirmClose } from "~/lib/panelTree";
import { countLeaves } from "~/lib/panelTree";
import { SessionContext } from "~/lib/sessionContext";
import { useBoolGlobalSetting } from "~/lib/settings/context";
import { PANEL_CLOSE_CONFIRM_SKIP_KEY } from "~/lib/settings/keys";
import { terminalCommandHandlers } from "~/lib/terminal/activeTerminal";
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

  const mountedRef = React.useRef(true);
  React.useEffect(() => {
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const { tree, split, close, setContent, resetToEmpty, setActiveTab } = usePanelTree(sessionId);
  const totalPanels = countLeaves(tree);

  // Per-placement live process status, fed by each terminal pane's onStatusChange. Backs the
  // confirm-if-running gate (surface-lifecycle spec): a terminal whose process has exited must
  // close without a prompt. A placement absent from the map is treated as still running (its pane
  // has not reported an exit), so the confirm defaults on for a live-but-unreported surface.
  const [statusByPlacement, setStatusByPlacement] = React.useState<Record<string, string>>({});

  const statusByPlacementRef = React.useRef(statusByPlacement);
  statusByPlacementRef.current = statusByPlacement;

  // Reset placements status when session changes to avoid leaking statuses of previous sessions
  React.useEffect(() => {
    setStatusByPlacement({});
  }, [sessionId]);

  const handleStatusChange = React.useCallback((placement: string, status: string) => {
    setStatusByPlacement((prev) =>
      prev[placement] === status ? prev : { ...prev, [placement]: status },
    );
  }, []);
  const isPlacementRunning = React.useCallback((placement: string) => {
    const s = statusByPlacementRef.current[placement];
    return s !== "exited" && s !== "error";
  }, []);

  const treeRef = React.useRef(tree);
  treeRef.current = tree;
  const detachedRef = React.useRef(detached);
  detachedRef.current = detached;
  const activeLeafRef = React.useRef<string | null>(null);

  // Focused pane (panel-multiplexer-nav spec): drives the focus ring and is the target for
  // pane keybindings and directional nav. activeLeafRef mirrors it so useShellCommands' pick()
  // reads the current focus synchronously. Zoom is transient view state (never persisted).
  const [focusedLeafId, setFocusedLeafId] = React.useState<string | null>(null);
  const [zoomedLeafId, setZoomedLeafId] = React.useState<string | null>(null);
  const setFocusedLeaf = React.useCallback((id: string) => {
    activeLeafRef.current = id;
    setFocusedLeafId(id);
  }, []);
  const toggleZoom = React.useCallback((id: string) => {
    setZoomedLeafId((z) => (z === id ? null : id));
  }, []);

  const onContentPointerDown = React.useCallback(
    (e: React.PointerEvent) => {
      const el = (e.target as HTMLElement).closest("[data-panel-id]");
      const id = el?.getAttribute("data-panel-id");
      if (id) setFocusedLeaf(id);
    },
    [setFocusedLeaf],
  );

  // Keep focus and zoom valid as the tree changes. Focus is seeded to the first leaf on mount (so
  // the ring is visible without a first click) and moves to the first remaining leaf whenever the
  // focused leaf is gone; a stale zoom is dropped. resetToEmpty keeps the leaf id, so a
  // reset-to-empty pane stays focused/zoomed.
  React.useEffect(() => {
    const leaves = collectLeaves(tree);
    const ids = new Set(leaves.map((l) => l.id));
    if (!focusedLeafId || !ids.has(focusedLeafId)) {
      const first = leaves[0]?.id ?? null;
      activeLeafRef.current = first;
      setFocusedLeafId(first);
    }
    if (zoomedLeafId && !ids.has(zoomedLeafId)) setZoomedLeafId(null);
  }, [tree, focusedLeafId, zoomedLeafId]);

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

  // Unbind a leaf back to the empty picker (surface-lifecycle spec): used by the exit bar's
  // "New surface" and the failure overlay's Dismiss. Terminates the (exited/failed) surface to free
  // its placement, then resets the leaf, keeping its geometry in the tree.
  const handleRequestReset = React.useCallback(
    (leafId: string) => {
      const leaf = findLeaf(treeRef.current, leafId);
      if (leaf && leaf.content.type === "terminal") {
        const placement = leaf.content.placement;
        if (sessionId) surfaceClose.mutate({ id: placement });
        setStatusByPlacement((prev) => {
          const next = { ...prev };
          delete next[placement];
          return next;
        });
      }
      resetToEmpty(leafId);
    },
    [sessionId, resetToEmpty, surfaceClose],
  );

  // Content-dependent close (surface-lifecycle spec). A terminal leaf terminates its surface and
  // resets to the empty picker in place (the leaf stays, even as the only pane). An empty leaf is
  // removed, fading out first, and the tree's always-one-leaf guarantee keeps the last pane alive.
  const runClose = React.useCallback(
    (leaf: PanelLeaf) => {
      if (leaf.content.type === "terminal") {
        const placement = leaf.content.placement;
        if (sessionId) surfaceClose.mutate({ id: placement });
        resetToEmpty(leaf.id);
        setStatusByPlacement((prev) => {
          const next = { ...prev };
          delete next[placement];
          return next;
        });
        return;
      }
      setClosingLeafIds((prev) => new Set(prev).add(leaf.id));
      window.setTimeout(() => {
        if (!mountedRef.current) return;
        close(leaf.id);
        setClosingLeafIds((prev) => {
          if (!prev.has(leaf.id)) return prev;
          const next = new Set(prev);
          next.delete(leaf.id);
          return next;
        });
      }, CLOSE_FADE_MS);
    },
    [sessionId, close, resetToEmpty, surfaceClose],
  );

  // Close confirmation fires only when the terminal's process is still running (surface-lifecycle
  // spec): an exited terminal or an empty leaf closes at once. "Don't ask again" still suppresses it.
  const { value: skipCloseConfirm, setValue: setSkipCloseConfirm } = useBoolGlobalSetting(
    PANEL_CLOSE_CONFIRM_SKIP_KEY,
    false,
  );
  const [pendingClose, setPendingClose] = React.useState<PanelLeaf | null>(null);
  const [dontAskAgain, setDontAskAgain] = React.useState(false);

  const handleClose = React.useCallback(
    (leaf: PanelLeaf) => {
      const isRunning =
        leaf.content.type === "terminal" && isPlacementRunning(leaf.content.placement);
      if (shouldConfirmClose(leaf, skipCloseConfirm, isRunning)) {
        setPendingClose(leaf);
        setDontAskAgain(false);
        return;
      }
      runClose(leaf);
    },
    [skipCloseConfirm, runClose, isPlacementRunning],
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
    setFocusedLeaf,
    toggleZoom,
  });

  if (host.status === "web") {
    return (
      <>
        <RegisterHandlers handlers={terminalCommandHandlers} />
        <TerminalPane sessionId={sessionId} />
      </>
    );
  }

  return (
    <div className="h-full w-full" onPointerDownCapture={onContentPointerDown}>
      <RegisterHandlers handlers={panelHandlers} />
      <RegisterHandlers handlers={terminalCommandHandlers} />
      {bootRegion === "content" ? (
        <PanelTree
          tree={tree}
          totalPanels={totalPanels}
          sessionId={sessionId}
          sessionTitle={session?.title}
          detached={detached}
          closingLeafIds={closingLeafIds}
          reloadEpoch={reloadEpoch}
          focusedLeafId={focusedLeafId}
          zoomedLeafId={zoomedLeafId}
          onSplit={split}
          onSetActiveTab={setActiveTab}
          onClose={handleClose}
          onSpawn={handleSpawn}
          onDetach={detach}
          onReattach={reattach}
          onSwapPlacements={handleSwapPlacements}
          onStatusChange={handleStatusChange}
          onRequestReset={handleRequestReset}
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
