import { useState, useCallback, useEffect } from "react";
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
import type { OrchestratorClient } from "@tillerd/sdk/orchestrator";

const LEGACY_STORAGE_KEY = "tillerd:panel-tree";

function discardLegacyLayout(): void {
  try {
    if (typeof localStorage !== "undefined") {
      localStorage.removeItem(LEGACY_STORAGE_KEY);
    }
  } catch {
    // storage unavailable
  }
}

export function usePanelTree(sessionId?: string | null, client?: OrchestratorClient | null) {
  const [tree, setTree] = useState<PanelNode>(DEFAULT_LAYOUT);

  // On mount: discard legacy key and load server layout when session + client are available
  useEffect(() => {
    discardLegacyLayout();
    if (!sessionId || !client) return;

    let cancelled = false;
    void (async () => {
      try {
        const blob = await client.getSessionLayout({ id: sessionId });
        if (cancelled) return;
        // Reset on a null layout -- never inherit the previous session's tree.
        if (!blob) {
          setTree(DEFAULT_LAYOUT);
          return;
        }
        try {
          setTree(deserializeLayout(blob));
        } catch {
          setTree(DEFAULT_LAYOUT);
        }
      } catch {
        setTree(DEFAULT_LAYOUT);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sessionId, client]);

  const persistLayout = useCallback(
    (next: PanelNode) => {
      if (!sessionId || !client) return;
      void client
        .setSessionLayout({ id: sessionId, layoutJson: serializeLayout(next) })
        .catch(() => {
          // non-fatal; layout will be re-persisted on next mutation
        });
    },
    [sessionId, client],
  );

  const update = useCallback(
    (fn: (t: PanelNode) => PanelNode) => {
      setTree((prev) => {
        const next = fn(prev);
        persistLayout(next);
        return next;
      });
    },
    [persistLayout],
  );

  const split = useCallback(
    (id: string, direction: "horizontal" | "vertical") => {
      update((t) => splitNode(t, id, direction));
    },
    [update],
  );

  const close = useCallback(
    (id: string) => {
      update((t) => {
        if (countLeaves(t) <= 1) return t;
        return closeNode(t, id) ?? DEFAULT_LAYOUT;
      });
    },
    [update],
  );

  const setContent = useCallback(
    (id: string, content: PanelContent) => {
      update((t) => setContentNode(t, id, content));
    },
    [update],
  );

  const setActiveTab = useCallback(
    (groupId: string, tabId: string) => {
      update((t) => setActiveTabNode(t, groupId, tabId));
    },
    [update],
  );

  return { tree, split, close, setContent, setActiveTab };
}
