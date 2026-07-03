import React from "react";

import type { CommandHandler } from "~/lib/commands/registry";

import { ACTION } from "~/lib/commands/ids";
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

// Handlers for the session-scoped panel/surface commands. Command identity,
// titles, keywords, and default keys live in the command definitions; this hook
// only binds behavior by id against the live panel tree.
export function useShellCommands({
  treeRef,
  activeLeafRef,
  detachedRef,
  split,
  spawn,
  close,
  detach,
}: ShellCommandDeps): Record<string, CommandHandler> {
  return React.useMemo<Record<string, CommandHandler>>(() => {
    const pick = (pred: (l: PanelLeaf) => boolean): PanelLeaf | undefined => {
      const leaves = collectLeaves(treeRef.current);
      return leaves.find((l) => l.id === activeLeafRef.current && pred(l)) ?? leaves.find(pred);
    };
    return {
      [ACTION.panelSplitH]: () => {
        const leaf = pick(() => true);
        if (leaf) split(leaf.id, "horizontal");
      },
      [ACTION.panelSplitV]: () => {
        const leaf = pick(() => true);
        if (leaf) split(leaf.id, "vertical");
      },
      [ACTION.surfaceSpawn]: () => {
        const leaf = pick((l) => l.content.type === "empty");
        if (leaf) spawn(leaf.id);
      },
      [ACTION.surfaceClose]: () => {
        if (collectLeaves(treeRef.current).length <= 1) return;
        const leaf = pick(() => true);
        if (leaf) close(leaf);
      },
      [ACTION.surfaceDetach]: () => {
        const leaf = pick(
          (l) => l.content.type === "terminal" && !detachedRef.current.has(l.content.placement),
        );
        if (leaf) detach(leaf);
      },
    };
    // Refs are stable; only the bound operations drive a rebuild.
  }, [treeRef, activeLeafRef, detachedRef, split, spawn, close, detach]);
}
