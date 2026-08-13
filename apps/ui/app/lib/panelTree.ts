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
  sizes: number[];
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
  return crypto.randomUUID();
}

export const DEFAULT_LAYOUT: PanelLeaf = {
  kind: "panel",
  id: "root",
  title: "Empty",
  content: { type: "empty" },
};

const LAYOUT_VERSION = 1;

export class LayoutFormatError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "LayoutFormatError";
  }
}

export function normalizePanelSizes(sizes: readonly number[], childCount: number): number[] {
  if (
    sizes.length !== childCount ||
    childCount < 2 ||
    sizes.some((size) => !Number.isFinite(size) || size < 0)
  ) {
    throw new LayoutFormatError("invalid panel sizes");
  }
  const total = sizes.reduce((sum, size) => sum + size, 0);
  if (!Number.isFinite(total) || total <= 0) throw new LayoutFormatError("invalid panel sizes");

  const normalized = sizes.map((size) => (size / total) * 100);
  normalized[normalized.length - 1] =
    100 - normalized.slice(0, -1).reduce((sum, size) => sum + size, 0);
  return normalized;
}

export function serializeLayout(node: PanelNode): string {
  return JSON.stringify({ version: LAYOUT_VERSION, root: node });
}

export function deserializeLayout(raw: string): PanelNode {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new LayoutFormatError("invalid layout JSON");
  }
  if (typeof parsed !== "object" || parsed === null) {
    throw new LayoutFormatError("invalid layout envelope");
  }
  const envelope = parsed as Record<string, unknown>;
  if (envelope["version"] !== LAYOUT_VERSION) {
    throw new LayoutFormatError("unsupported layout version");
  }
  const ids = new Set<string>();
  const placements = new Set<string>();
  validateNode(envelope["root"], ids, placements);
  return envelope["root"];
}

function validateNode(
  node: unknown,
  ids: Set<string>,
  placements: Set<string>,
): asserts node is PanelNode {
  if (typeof node !== "object" || node === null) throw new LayoutFormatError("invalid node");
  const value = node as Record<string, unknown>;
  if (typeof value["id"] !== "string" || value["id"].length === 0) {
    throw new LayoutFormatError("invalid node id");
  }
  if (ids.has(value["id"])) throw new LayoutFormatError("duplicate node id");
  ids.add(value["id"]);
  if (value["kind"] === "panel") {
    if (typeof value["title"] !== "string" || value["title"].length === 0) {
      throw new LayoutFormatError("invalid panel title");
    }
    const content = value["content"];
    if (typeof content !== "object" || content === null) {
      throw new LayoutFormatError("invalid panel content");
    }
    const panelContent = content as Record<string, unknown>;
    if (panelContent["type"] === "terminal") {
      const placement = panelContent["placement"];
      if (typeof placement !== "string" || placement.length === 0 || placements.has(placement)) {
        throw new LayoutFormatError("invalid panel placement");
      }
      placements.add(placement);
    } else if (panelContent["type"] !== "empty") {
      throw new LayoutFormatError("invalid panel content");
    }
    if (value["toolbar"] !== undefined) validateToolbar(value["toolbar"]);
    return;
  }
  if (value["kind"] !== "group") throw new LayoutFormatError("invalid kind");
  if (value["direction"] !== "horizontal" && value["direction"] !== "vertical") {
    throw new LayoutFormatError("invalid group direction");
  }
  if (
    value["displayMode"] !== "split" &&
    value["displayMode"] !== "tabbar-top" &&
    value["displayMode"] !== "tabbar-bottom"
  ) {
    throw new LayoutFormatError("invalid display mode");
  }
  if (value["activeTabId"] !== undefined && typeof value["activeTabId"] !== "string") {
    throw new LayoutFormatError("invalid active tab");
  }
  if (!Array.isArray(value["children"])) throw new LayoutFormatError("group missing children");
  if (!Array.isArray(value["sizes"])) throw new LayoutFormatError("invalid panel sizes");
  value["sizes"] = normalizePanelSizes(value["sizes"] as number[], value["children"].length);
  for (const child of value["children"]) validateNode(child, ids, placements);
  if (
    value["activeTabId"] !== undefined &&
    !value["children"].some(
      (child) =>
        typeof child === "object" &&
        child !== null &&
        "id" in child &&
        child.id === value["activeTabId"],
    )
  ) {
    throw new LayoutFormatError("invalid active tab");
  }
}

function validateToolbar(toolbar: unknown): void {
  if (typeof toolbar !== "object" || toolbar === null) {
    throw new LayoutFormatError("invalid panel toolbar");
  }
  const buttons = (toolbar as Record<string, unknown>)["buttons"];
  if (
    !Array.isArray(buttons) ||
    buttons.some(
      (button) =>
        typeof button !== "object" ||
        button === null ||
        ["id", "icon", "label"].some(
          (field) => typeof (button as Record<string, unknown>)[field] !== "string",
        ),
    )
  ) {
    throw new LayoutFormatError("invalid panel toolbar");
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
      sizes: [50, 50],
    };
  }
  return {
    ...tree,
    children: tree.children.map((c) => splitNode(c, targetId, direction, newLeafId)),
  };
}

export function closeNode(tree: PanelNode, targetId: string): PanelNode | null {
  if (tree.kind === "panel") return tree.id === targetId ? null : tree;

  const children: PanelNode[] = [];
  const sizes: number[] = [];
  let changed = false;
  for (let index = 0; index < tree.children.length; index += 1) {
    const child = tree.children[index];
    const next = closeNode(child, targetId);
    if (next) {
      children.push(next);
      sizes.push(tree.sizes[index] as number);
      changed ||= next !== child;
    } else {
      changed = true;
    }
  }
  if (!changed) return tree;
  if (children.length === 0) return null;
  if (children.length === 1) return children[0] as PanelNode;
  const survivingSizes = sizes.some((size) => size > 0) ? sizes : sizes.map(() => 1);
  const next = {
    ...tree,
    children,
    sizes: normalizePanelSizes(survivingSizes, children.length),
  };
  if (next.activeTabId && !children.some((child) => child.id === next.activeTabId)) {
    next.activeTabId = children[0]?.id;
  }
  return next;
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

export function setGroupSizesNode(
  tree: PanelNode,
  targetId: string,
  sizes: readonly number[],
): PanelNode {
  if (tree.kind === "panel") return tree;
  if (tree.id === targetId) {
    return { ...tree, sizes: normalizePanelSizes(sizes, tree.children.length) };
  }
  const children = tree.children.map((child) => setGroupSizesNode(child, targetId, sizes));
  return children.every((child, index) => child === tree.children[index])
    ? tree
    : { ...tree, children };
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
