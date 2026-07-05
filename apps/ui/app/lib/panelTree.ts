export type PanelContent = { type: "terminal"; placement: string } | { type: "empty" };

export type ToolbarButtonConfig = {
  id: string;
  icon: string;
  label: string;
};

export type ToolbarConfig = {
  buttons: ToolbarButtonConfig[];
};

export type PanelLeaf = {
  kind: "panel";
  id: string;
  title: string;
  content: PanelContent;
  toolbar?: ToolbarConfig;
};

export type DisplayMode = "split" | "tabbar-top" | "tabbar-bottom";

export type PanelGroupNode = {
  kind: "group";
  id: string;
  direction: "horizontal" | "vertical";
  displayMode: DisplayMode;
  activeTabId?: string;
  children: PanelNode[];
};

export type PanelNode = PanelGroupNode | PanelLeaf;

// dataTransfer key for the panel-header drag source (placement swap, panel-placement-swap spec).
export const DRAG_PANEL_LEAF = "application/x-tillerd-panel-leaf";

export function makeId(): string {
  return Math.random().toString(36).slice(2, 10);
}

export const DEFAULT_LAYOUT: PanelLeaf = {
  kind: "panel",
  id: "root",
  title: "Empty",
  content: { type: "empty" },
};

export function serializeLayout(node: PanelNode): string {
  return JSON.stringify(node);
}

export function deserializeLayout(raw: string): PanelNode {
  const parsed = JSON.parse(raw) as PanelNode;
  validateNode(parsed);
  return parsed;
}

function validateNode(node: unknown): asserts node is PanelNode {
  if (typeof node !== "object" || node === null) throw new Error("invalid node");
  const n = node as Record<string, unknown>;
  if (n["kind"] !== "panel" && n["kind"] !== "group") throw new Error("invalid kind");
  if (n["kind"] === "group") {
    if (!Array.isArray(n["children"])) throw new Error("group missing children");
    for (const c of n["children"] as unknown[]) validateNode(c);
  }
}

export function splitNode(
  tree: PanelNode,
  targetId: string,
  direction: "horizontal" | "vertical",
  newLeafId: string = makeId(),
): PanelNode {
  if (tree.kind === "panel") {
    if (tree.id !== targetId) return tree;
    const newLeaf: PanelLeaf = {
      kind: "panel",
      id: newLeafId,
      title: "Empty",
      content: { type: "empty" },
    };
    return {
      kind: "group",
      id: makeId(),
      direction,
      displayMode: "split",
      children: [tree, newLeaf],
    };
  }
  return {
    ...tree,
    children: tree.children.map((c) => splitNode(c, targetId, direction, newLeafId)),
  };
}

export function closeNode(tree: PanelNode, targetId: string): PanelNode | null {
  if (tree.kind === "panel") {
    return tree.id === targetId ? null : tree;
  }
  const newChildren = tree.children
    .map((c) => closeNode(c, targetId))
    .filter((c): c is PanelNode => c !== null);
  if (newChildren.length === 0) return null;
  if (newChildren.length === 1) return newChildren[0];
  return { ...tree, children: newChildren };
}

export function setContentNode(
  tree: PanelNode,
  targetId: string,
  content: PanelContent,
): PanelNode {
  if (tree.kind === "panel") {
    return tree.id === targetId ? { ...tree, content } : tree;
  }
  return { ...tree, children: tree.children.map((c) => setContentNode(c, targetId, content)) };
}

export function resetLeafToEmpty(tree: PanelNode, targetId: string): PanelNode {
  return setContentNode(tree, targetId, { type: "empty" });
}

// Guarantees the tree always keeps at least one leaf (surface-lifecycle spec): closing the
// last remaining leaf empties it instead of removing it, and closeNode's defensive null case
// (should be unreachable given the sole-leaf check above) falls back to a fresh empty layout.
export function closeLeafSafe(tree: PanelNode, targetId: string): PanelNode {
  const leaves = collectLeaves(tree);
  if (leaves.length === 1 && leaves[0].id === targetId) {
    return resetLeafToEmpty(tree, targetId);
  }
  return closeNode(tree, targetId) ?? DEFAULT_LAYOUT;
}

export function setDisplayModeNode(
  tree: PanelNode,
  targetId: string,
  displayMode: DisplayMode,
): PanelNode {
  if (tree.kind === "panel") return tree;
  if (tree.id === targetId) return { ...tree, displayMode };
  return {
    ...tree,
    children: tree.children.map((c) => setDisplayModeNode(c, targetId, displayMode)),
  };
}

export function setActiveTabNode(tree: PanelNode, groupId: string, tabId: string): PanelNode {
  if (tree.kind === "panel") return tree;
  if (tree.id === groupId) return { ...tree, activeTabId: tabId };
  return { ...tree, children: tree.children.map((c) => setActiveTabNode(c, groupId, tabId)) };
}

export function countLeaves(tree: PanelNode): number {
  if (tree.kind === "panel") return 1;
  return tree.children.reduce((s, c) => s + countLeaves(c), 0);
}

export function collectLeaves(tree: PanelNode): PanelLeaf[] {
  if (tree.kind === "panel") return [tree];
  return tree.children.flatMap(collectLeaves);
}

// Pure predicate for the close-surface confirmation gate (ui-panel-compound spec): only a
// surface-bound leaf has a PTY to terminate, an exited process has nothing left to interrupt,
// and the "don't ask again" preference short-circuits it.
export function shouldConfirmClose(
  leaf: PanelLeaf,
  skipConfirm: boolean,
  isRunning: boolean,
): boolean {
  return leaf.content.type === "terminal" && isRunning && !skipConfirm;
}

export function findLeaf(tree: PanelNode, targetId: string): PanelLeaf | undefined {
  if (tree.kind === "panel") return tree.id === targetId ? tree : undefined;
  for (const child of tree.children) {
    const found = findLeaf(child, targetId);
    if (found) return found;
  }
  return undefined;
}
