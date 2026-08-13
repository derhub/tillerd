import { useQuery, useQueryClient } from "@tanstack/react-query";
import { query, runCommand } from "@tillerd/client-bindings";
import React from "react";

import {
  type PanelNode,
  type PanelContent,
  DEFAULT_LAYOUT,
  serializeLayout,
  deserializeLayout,
  splitNode,
  makeId,
  closeLeafSafe,
  setContentNode,
  resetLeafToEmpty,
  setActiveTabNode,
  setGroupSizesNode,
} from "./panelTree";

export function sessionLayoutQuery(id: string) {
  return query("sessionLayoutGet", { id });
}

function layoutToTree(blob: string | null | undefined): {
  tree: PanelNode;
  error: Error | null;
} {
  if (blob === null || blob === undefined) return { tree: DEFAULT_LAYOUT, error: null };
  try {
    return { tree: deserializeLayout(blob), error: null };
  } catch (error) {
    return {
      tree: DEFAULT_LAYOUT,
      error: error instanceof Error ? error : new Error(String(error)),
    };
  }
}

export function usePanelTree(sessionId?: string | null) {
  const layoutQuery = useQuery({
    ...sessionLayoutQuery(sessionId ?? ""),
    enabled: !!sessionId,
  });

  const key = sessionId ?? null;
  const [panelState, setPanelState] = React.useState<{
    key: string | null;
    seeded: boolean;
    tree: PanelNode;
    error: Error | null;
  }>({ key: null, seeded: true, tree: DEFAULT_LAYOUT, error: null });
  if (panelState.key !== key) {
    const restored =
      key && layoutQuery.data !== undefined
        ? layoutToTree(layoutQuery.data)
        : { tree: DEFAULT_LAYOUT, error: null };
    setPanelState({
      key,
      seeded: !key || layoutQuery.data !== undefined,
      tree: restored.tree,
      error: restored.error,
    });
  } else if (key && !panelState.seeded && layoutQuery.data !== undefined) {
    const restored = layoutToTree(layoutQuery.data);
    setPanelState({ key, seeded: true, tree: restored.tree, error: restored.error });
  }
  const tree = panelState.tree;
  const layoutError = panelState.error;
  const layoutPending = Boolean(key) && !panelState.seeded;
  const setTree = React.useCallback((update: React.SetStateAction<PanelNode>) => {
    setPanelState((current) => ({
      ...current,
      tree: typeof update === "function" ? update(current.tree) : update,
    }));
  }, []);

  const queryClient = useQueryClient();
  const persistLayout = React.useCallback(
    (next: PanelNode) => {
      if (!sessionId) return;
      const layoutJson = serializeLayout(next);
      queryClient.setQueryData(sessionLayoutQuery(sessionId).queryKey, layoutJson);
      void runCommand("sessionLayoutSet", { id: sessionId, layoutJson }).catch(() => {
        // non-fatal; layout re-persists on next mutation
      });
    },
    [sessionId, queryClient],
  );
  const update = React.useCallback(
    (fn: (t: PanelNode) => PanelNode) => {
      if (layoutPending || layoutError) return;
      setTree((prev) => {
        const next = fn(prev);
        persistLayout(next);
        return next;
      });
    },
    [layoutError, layoutPending, persistLayout],
  );
  // target it (e.g. spawn a surface into the freshly made empty pane).
  const split = React.useCallback(
    (id: string, direction: "horizontal" | "vertical") => {
      const newLeafId = makeId();
      update((t) => splitNode(t, id, direction, newLeafId));
      return newLeafId;
    },
    [update],
  );

  // Remove a leaf and collapse its parent split. closeLeafSafe keeps the always-one-leaf
  // guarantee (surface-lifecycle spec): removing the sole leaf resets it to a single empty leaf
  // rather than emptying the tree, so the former countLeaves<=1 block is gone.
  const close = React.useCallback(
    (id: string) => {
      update((t) => closeLeafSafe(t, id));
    },
    [update],
  );

  const setContent = React.useCallback(
    (id: string, content: PanelContent) => {
      update((t) => setContentNode(t, id, content));
    },
    [update],
  );

  // Unbind a leaf back to the empty picker in place (surface-lifecycle spec): closing a terminal
  // pane terminates its surface and resets this leaf, keeping its geometry in the tree.
  const resetToEmpty = React.useCallback(
    (id: string) => {
      update((t) => resetLeafToEmpty(t, id));
    },
    [update],
  );

  const setActiveTab = React.useCallback(
    (groupId: string, tabId: string) => {
      update((t) => setActiveTabNode(t, groupId, tabId));
    },
    [update],
  );

  const setGroupSizes = React.useCallback(
    (groupId: string, sizes: readonly number[]) => {
      update((tree) => setGroupSizesNode(tree, groupId, sizes));
    },
    [update],
  );

  return {
    tree,
    layoutError,
    layoutPending,
    split,
    close,
    setContent,
    resetToEmpty,
    setActiveTab,
    setGroupSizes,
  };
}
