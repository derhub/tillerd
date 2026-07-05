import type { SpawnCommandRef } from "@tillerd/client-bindings";

import React from "react";

import type { CommandHandler } from "~/lib/commands/registry";

import { ACTION } from "~/lib/commands/ids";
import { collectLeaves, type PanelLeaf, type PanelNode } from "~/lib/panelTree";

interface ShellCommandDeps {
  treeRef: React.RefObject<PanelNode>;
  activeLeafRef: React.RefObject<string | null>;
  detachedRef: React.RefObject<Set<string>>;
  // Returns the id of the leaf the split creates, so a spawn can target it immediately.
  split: (id: string, direction: "horizontal" | "vertical") => string;
  spawn: (leafId: string, commandRef?: SpawnCommandRef) => void;
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
      // Run a library command (dispatched by the out-of-tree commands sidebar): place its PTY in
      // the active/first empty leaf, else split the active/first leaf to make room. Without a leaf
      // placement the spawned surface renders nowhere and leaks on every click.
      [ACTION.surfaceRunCommand]: (args) => {
        const commandRef = args?.commandRef as SpawnCommandRef | undefined;
        const empty = pick((l) => l.content.type === "empty");
        if (empty) {
          spawn(empty.id, commandRef);
          return;
        }
        const target = pick(() => true);
        if (target) spawn(split(target.id, "horizontal"), commandRef);
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
