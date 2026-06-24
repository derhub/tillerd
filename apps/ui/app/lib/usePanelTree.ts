import { queryOptions, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, ensureResult, whenReady } from "@tillerd/client-bindings";
import React from "react";

import {
  type PanelNode,
  type PanelContent,
  DEFAULT_LAYOUT,
  serializeLayout,
  deserializeLayout,
  splitNode,
  closeNode,
  setContentNode,
  setActiveTabNode,
  countLeaves,
} from "./panelTree";

export function sessionLayoutQuery(id: string) {
  return queryOptions({
    queryKey: ["sessions", "layout", id] as const,
    queryFn: () =>
      whenReady().then((ok) => (ok ? commands.sessionLayoutGet({ id }).then(ensureResult) : null)),
  });
}

const LEGACY_STORAGE_KEY = "tillerd:panel-tree";

function discardLegacyLayout(): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.removeItem(LEGACY_STORAGE_KEY);
    }
  } catch {
    // localStorage unavailable
  }
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

  React.useEffect(() => discardLegacyLayout(), []);

  const queryClient = useQueryClient();
  const persistLayout = React.useCallback(
    (next: PanelNode) => {
      if (!sessionId) return;
      const layoutJson = serializeLayout(next);
      queryClient.setQueryData(sessionLayoutQuery(sessionId).queryKey, layoutJson);
      void commands.sessionLayoutSet({ id: sessionId, layoutJson }).catch(() => {
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

  const split = React.useCallback(
    (id: string, direction: "horizontal" | "vertical") => {
      update((t) => splitNode(t, id, direction));
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
