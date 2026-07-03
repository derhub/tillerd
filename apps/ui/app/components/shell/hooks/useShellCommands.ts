import React from "react";

import type { Command } from "~/lib/commands/registry";

import { ACTION, ACTION_TITLES } from "~/lib/commands/ids";
import { collectLeaves, type PanelLeaf, type PanelNode } from "~/lib/panelTree";

interface ShellCommandDeps {
  treeRef: React.RefObject<PanelNode>;
  activeLeafRef: React.RefObject<string | null>;
  detachedRef: React.RefObject<Set<string>>;
  split: (id: string, direction: "horizontal" | "vertical") => void;
  spawn: (leafId: string) => void;
  close: (leaf: PanelLeaf) => void;
  detach: (leaf: PanelLeaf) => void;
}

export function useShellCommands({
  treeRef,
  activeLeafRef,
  detachedRef,
  split,
  spawn,
  close,
  detach,
}: ShellCommandDeps): Command[] {
  return React.useMemo<Command[]>(() => {
    const pick = (pred: (l: PanelLeaf) => boolean): PanelLeaf | undefined => {
      const leaves = collectLeaves(treeRef.current);
      return leaves.find((l) => l.id === activeLeafRef.current && pred(l)) ?? leaves.find(pred);
    };
    return [
      {
        id: ACTION.panelSplitH,
        title: ACTION_TITLES[ACTION.panelSplitH],
        keywords: ["split", "horizontal", "right"],
        run: () => {
          const leaf = pick(() => true);
          if (leaf) split(leaf.id, "horizontal");
        },
      },
      {
        id: ACTION.panelSplitV,
        title: ACTION_TITLES[ACTION.panelSplitV],
        keywords: ["split", "vertical", "down"],
        run: () => {
          const leaf = pick(() => true);
          if (leaf) split(leaf.id, "vertical");
        },
      },
      {
        id: ACTION.surfaceSpawn,
        title: ACTION_TITLES[ACTION.surfaceSpawn],
        keywords: ["terminal", "surface", "spawn"],
        run: () => {
          const leaf = pick((l) => l.content.type === "empty");
          if (leaf) spawn(leaf.id);
        },
      },
      {
        id: ACTION.surfaceClose,
        title: ACTION_TITLES[ACTION.surfaceClose],
        keywords: ["close", "surface"],
        run: () => {
          if (collectLeaves(treeRef.current).length <= 1) return;
          const leaf = pick(() => true);
          if (leaf) close(leaf);
        },
      },
      {
        id: ACTION.surfaceDetach,
        title: ACTION_TITLES[ACTION.surfaceDetach],
        keywords: ["detach", "window"],
        run: () => {
          const leaf = pick(
            (l) => l.content.type === "terminal" && !detachedRef.current.has(l.content.placement),
          );
          if (leaf) detach(leaf);
        },
      },
    ];
    // Refs are stable; only the bound operations drive a rebuild.
  }, [treeRef, activeLeafRef, detachedRef, split, spawn, close, detach]);
}
