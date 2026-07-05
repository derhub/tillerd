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
  closeNode,
  setContentNode,
  setActiveTabNode,
  countLeaves,
} from "./panelTree";

export function sessionLayoutQuery(id: string) {
  return query("sessionLayoutGet", { id });
}

function layoutToTree(blob: string | null | undefined): PanelNode {
  if (!blob) return DEFAULT_LAYOUT;
  try {
    return deserializeLayout(blob);
  } catch {
    return DEFAULT_LAYOUT;
  }
}

export function usePanelTree(sessionId?: string | null) {
  const layoutQuery = useQuery({
    ...sessionLayoutQuery(sessionId ?? ""),
    enabled: !!sessionId,
  });

  // Seeded during render, not in an effect, so refetches never clobber in-progress edits.
  const [tree, setTree] = React.useState<PanelNode>(DEFAULT_LAYOUT);
  const seededFor = React.useRef<string | null>(null);
  const key = sessionId ?? null;
  if (key && layoutQuery.data !== undefined && seededFor.current !== key) {
    seededFor.current = key;
    setTree(layoutToTree(layoutQuery.data));
  }

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
      setTree((prev) => {
        const next = fn(prev);
        persistLayout(next);
        return next;
      });
    },
    [persistLayout],
  );

  // Returns the id of the leaf created by the split so a caller can immediately
  // target it (e.g. spawn a surface into the freshly made empty pane).
  const split = React.useCallback(
    (id: string, direction: "horizontal" | "vertical") => {
      const newLeafId = makeId();
      update((t) => splitNode(t, id, direction, newLeafId));
      return newLeafId;
    },
    [update],
  );

  const close = React.useCallback(
    (id: string) => {
      update((t) => {
        if (countLeaves(t) <= 1) return t;
        return closeNode(t, id) ?? DEFAULT_LAYOUT;
      });
    },
    [update],
  );

  const setContent = React.useCallback(
    (id: string, content: PanelContent) => {
      update((t) => setContentNode(t, id, content));
    },
    [update],
  );

  const setActiveTab = React.useCallback(
    (groupId: string, tabId: string) => {
      update((t) => setActiveTabNode(t, groupId, tabId));
    },
    [update],
  );

  return { tree, split, close, setContent, setActiveTab };
}
