import { test, expect, describe } from "bun:test";
import {
  DEFAULT_LAYOUT,
  splitNode,
  closeNode,
  setContentNode,
  serializeLayout,
  deserializeLayout,
  countLeaves,
  type PanelNode,
  type PanelLeaf,
  type PanelGroupNode,
} from "./panelTree";

const leaf = (id: string): PanelLeaf => ({
  kind: "panel",
  id,
  title: id,
  content: { type: "empty" },
});

describe("splitNode", () => {
  test("replaces leaf with horizontal group", () => {
    const tree = leaf("a");
    const result = splitNode(tree, "a", "horizontal") as PanelGroupNode;
    expect(result.kind).toBe("group");
    expect(result.direction).toBe("horizontal");
    expect(result.displayMode).toBe("split");
    expect(result.children).toHaveLength(2);
    expect(result.children[0]).toMatchObject({ id: "a" });
    expect(result.children[1]).toMatchObject({ kind: "panel", content: { type: "empty" } });
  });

  test("replaces leaf with vertical group", () => {
    const tree = leaf("a");
    const result = splitNode(tree, "a", "vertical") as PanelGroupNode;
    expect(result.direction).toBe("vertical");
  });

  test("nested split finds correct leaf", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "g",
      direction: "horizontal",
      displayMode: "split",
      children: [leaf("a"), leaf("b")],
    };
    const result = splitNode(tree, "b", "horizontal") as PanelGroupNode;
    expect(result.children[0]).toMatchObject({ id: "a" });
    expect((result.children[1] as PanelGroupNode).kind).toBe("group");
  });

  test("no-op when id not found", () => {
    const tree = leaf("a");
    expect(splitNode(tree, "z", "horizontal")).toBe(tree);
  });
});

describe("closeNode", () => {
  test("returns null when closing the only leaf", () => {
    expect(closeNode(leaf("a"), "a")).toBeNull();
  });

  test("removes leaf from group", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "g",
      direction: "horizontal",
      displayMode: "split",
      children: [leaf("a"), leaf("b"), leaf("c")],
    };
    const result = closeNode(tree, "b") as PanelGroupNode;
    expect(result.children).toHaveLength(2);
    expect(result.children.map((c) => c.id)).toEqual(["a", "c"]);
  });

  test("collapses group when 1 child remains", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "g",
      direction: "horizontal",
      displayMode: "split",
      children: [leaf("a"), leaf("b")],
    };
    const result = closeNode(tree, "b");
    expect(result).toMatchObject({ kind: "panel", id: "a" });
  });

  test("no-op when id not found", () => {
    const tree = leaf("a");
    expect(closeNode(tree, "z")).toBe(tree);
  });
});

describe("setContentNode", () => {
  test("updates content on matching leaf", () => {
    const tree = leaf("a");
    const result = setContentNode(tree, "a", { type: "terminal", sessionId: null });
    expect((result as PanelLeaf).content).toEqual({ type: "terminal", sessionId: null });
  });

  test("no-op on non-matching leaf", () => {
    const tree = leaf("a");
    expect(setContentNode(tree, "z", { type: "sidebar" })).toBe(tree);
  });
});

describe("serialize / deserialize", () => {
  test("round-trips DEFAULT_LAYOUT", () => {
    const raw = serializeLayout(DEFAULT_LAYOUT);
    const result = deserializeLayout(raw);
    expect(result).toEqual(DEFAULT_LAYOUT);
  });

  test("falls back to DEFAULT_LAYOUT on corrupt data in usePanelTree context", () => {
    expect(() => deserializeLayout("not json")).toThrow();
    expect(() => deserializeLayout('{"kind":"invalid"}')).toThrow();
  });
});

describe("countLeaves", () => {
  test("counts single leaf", () => {
    expect(countLeaves(leaf("a"))).toBe(1);
  });

  test("counts nested leaves", () => {
    const tree: PanelNode = {
      kind: "group",
      id: "g",
      direction: "horizontal",
      displayMode: "split",
      children: [leaf("a"), leaf("b"), leaf("c")],
    };
    expect(countLeaves(tree)).toBe(3);
  });
});
