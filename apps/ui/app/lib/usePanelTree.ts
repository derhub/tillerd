import { useState, useCallback } from "react";
import {
  type PanelNode,
  type PanelContent,
  type DisplayMode,
  DEFAULT_LAYOUT,
  serializeLayout,
  deserializeLayout,
  splitNode,
  closeNode,
  setContentNode,
  setDisplayModeNode,
  setActiveTabNode,
  countLeaves,
} from "./panelTree";

const STORAGE_KEY = "tillerd:panel-tree";

function loadTree(): PanelNode {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_LAYOUT;
    return deserializeLayout(raw);
  } catch {
    return DEFAULT_LAYOUT;
  }
}

function saveTree(tree: PanelNode): void {
  try {
    localStorage.setItem(STORAGE_KEY, serializeLayout(tree));
  } catch {
    // storage unavailable
  }
}

export function usePanelTree() {
  const [tree, setTree] = useState<PanelNode>(() => loadTree());

  const update = useCallback((fn: (t: PanelNode) => PanelNode) => {
    setTree((prev) => {
      const next = fn(prev);
      saveTree(next);
      return next;
    });
  }, []);

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

  const setDisplayMode = useCallback(
    (groupId: string, displayMode: DisplayMode) => {
      update((t) => setDisplayModeNode(t, groupId, displayMode));
    },
    [update],
  );

  const setActiveTab = useCallback(
    (groupId: string, tabId: string) => {
      update((t) => setActiveTabNode(t, groupId, tabId));
    },
    [update],
  );

  return { tree, split, close, setContent, setDisplayMode, setActiveTab };
}
