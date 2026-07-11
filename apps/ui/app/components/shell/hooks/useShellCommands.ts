import type { SpawnCommandRef } from "@tillerd/client-bindings";

import React from "react";

import type { CommandHandler } from "~/lib/commands/registry";

import { ACTION } from "~/lib/commands/ids";
import { collectLeaves, type PanelLeaf, type PanelNode } from "~/lib/panelTree";
import { nearestLeafInDirection, type Direction, type LeafRect } from "~/lib/paneNavigation";

interface ShellCommandDeps {
  treeRef: React.RefObject<PanelNode>;
  activeLeafRef: React.RefObject<string | null>;
  detachedRef: React.RefObject<Set<string>>;
  // Returns the id of the leaf the split creates, so a spawn can target it immediately.
  split: (id: string, direction: "horizontal" | "vertical") => string;
  spawn: (leafId: string, commandRef?: SpawnCommandRef) => void;
  close: (leaf: PanelLeaf) => void;
  detach: (leaf: PanelLeaf) => void;
  setFocusedLeaf: (id: string) => void;
  toggleZoom: (id: string) => void;
}

// Reads the on-screen rect of every rendered pane leaf. The DOM is the source of truth for pane
// geometry (nested splits, resizes, swaps), so directional nav hit-tests live rects rather than
// inferring adjacency from the tree structure.
function readLeafRects(): LeafRect[] {
  const rects: LeafRect[] = [];
  for (const el of document.querySelectorAll<HTMLElement>("[data-panel-id]")) {
    const id = el.getAttribute("data-panel-id");
    if (!id) continue;
    const r = el.getBoundingClientRect();
    rects.push({ id, left: r.left, right: r.right, top: r.top, bottom: r.bottom });
  }
  return rects;
}

// Move real keyboard focus onto a leaf so typed input goes there, not just the focus ring: xterm
// only routes input (and pane keybindings, via its key handler) to whichever terminal holds DOM
// focus. Targets the terminal's helper textarea; an empty leaf has none, so focus falls to its
// container so pane shortcuts still resolve against it.
function focusLeafTerminal(id: string): void {
  const pane = document.querySelector<HTMLElement>(`[data-panel-id="${id}"]`);
  if (!pane) return;
  const target = pane.querySelector<HTMLElement>(".xterm-helper-textarea, textarea") ?? pane;
  target.focus();
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
  setFocusedLeaf,
  toggleZoom,
}: ShellCommandDeps): Record<string, CommandHandler> {
  return React.useMemo<Record<string, CommandHandler>>(() => {
    const pick = (pred: (l: PanelLeaf) => boolean): PanelLeaf | undefined => {
      const leaves = collectLeaves(treeRef.current);
      return leaves.find((l) => l.id === activeLeafRef.current && pred(l)) ?? leaves.find(pred);
    };
    const navigate = (dir: Direction) => {
      const from = activeLeafRef.current ?? collectLeaves(treeRef.current)[0]?.id;
      if (!from) return;
      const next = nearestLeafInDirection(from, dir, readLeafRects());
      if (next) {
        setFocusedLeaf(next);
        focusLeafTerminal(next);
      }
    };
    const spawnIntoLeaf = (commandRef?: SpawnCommandRef) => {
      const empty = pick((l) => l.content.type === "empty");
      if (empty) {
        if (commandRef === undefined) {
          spawn(empty.id);
        } else {
          spawn(empty.id, commandRef);
        }
        return;
      }
      const target = pick(() => true);
      if (target) {
        const nextId = split(target.id, "horizontal");
        if (commandRef === undefined) {
          spawn(nextId);
        } else {
          spawn(nextId, commandRef);
        }
      }
    };
    return {
      [ACTION.panelSplitH]: () => {
        const leaf = pick(() => true);
        if (leaf) {
          const newId = split(leaf.id, "horizontal");
          setFocusedLeaf(newId);
          requestAnimationFrame(() => focusLeafTerminal(newId));
        }
      },
      [ACTION.panelSplitV]: () => {
        const leaf = pick(() => true);
        if (leaf) {
          const newId = split(leaf.id, "vertical");
          setFocusedLeaf(newId);
          requestAnimationFrame(() => focusLeafTerminal(newId));
        }
      },
      [ACTION.surfaceSpawn]: () => {
        const leaf = pick((l) => l.content.type === "empty");
        if (leaf) spawn(leaf.id);
      },
      // New surface: place a terminal in the active/first empty leaf, else split the active/first
      // leaf to make room and spawn there (mirrors surfaceRunCommand's placement, without a command).
      [ACTION.surfaceNew]: () => spawnIntoLeaf(),
      // Run a library command (dispatched by the out-of-tree commands sidebar): place its PTY in
      // the active/first empty leaf, else split the active/first leaf to make room. Without a leaf
      // placement the spawned surface renders nowhere and leaks on every click.
      [ACTION.surfaceRunCommand]: (args) => {
        spawnIntoLeaf(args?.commandRef as SpawnCommandRef | undefined);
      },
      // Close acts on the focused/first leaf. The always-one-pane guarantee and the terminal-vs-empty
      // outcome live in PanelContent's handleClose + the tree ops, so no leaf-count guard here.
      [ACTION.surfaceClose]: () => {
        const leaf = pick(() => true);
        if (leaf) close(leaf);
      },
      [ACTION.surfaceDetach]: () => {
        const leaf = pick(
          (l) => l.content.type === "terminal" && !detachedRef.current.has(l.content.placement),
        );
        if (leaf) detach(leaf);
      },
      [ACTION.paneFocusLeft]: () => navigate("left"),
      [ACTION.paneFocusRight]: () => navigate("right"),
      [ACTION.paneFocusUp]: () => navigate("up"),
      [ACTION.paneFocusDown]: () => navigate("down"),
      [ACTION.paneZoomToggle]: () => {
        const leaf = pick(() => true);
        if (leaf) toggleZoom(leaf.id);
      },
    };
    // Refs are stable; only the bound operations drive a rebuild.
  }, [
    treeRef,
    activeLeafRef,
    detachedRef,
    split,
    spawn,
    close,
    detach,
    setFocusedLeaf,
    toggleZoom,
  ]);
}
