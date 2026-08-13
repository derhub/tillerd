import { test, expect, describe } from "bun:test";

import {
  DEFAULT_LAYOUT,
  splitNode,
  closeNode,
  closeLeafSafe,
  setContentNode,
  resetLeafToEmpty,
  serializeLayout,
  deserializeLayout,
  setGroupSizesNode,
  countLeaves,
  collectLeaves,
  findLeaf,
  shouldConfirmClose,
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
  test("New split starts equal", () => {
    const tree = leaf("a");
    const result = splitNode(tree, "a", "horizontal") as PanelGroupNode;
    expect(result.kind).toBe("group");
    expect(result.direction).toBe("horizontal");
    expect(result.displayMode).toBe("split");
    expect(result.children).toHaveLength(2);
    expect(result.sizes).toEqual([50, 50]);
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
      sizes: [50, 50],
      children: [leaf("a"), leaf("b")],
    };
    const result = splitNode(tree, "b", "horizontal") as PanelGroupNode;
    expect(result.children[0]).toMatchObject({ id: "a" });
    expect((result.children[1] as PanelGroupNode).kind).toBe("group");
  });

  test("Nested split sizes stay independent", () => {
    const nested: PanelGroupNode = {
      kind: "group",
      id: "nested",
      direction: "vertical",
      displayMode: "split",
      sizes: [50, 50],
      children: [leaf("b"), leaf("c")],
    };
    const tree: PanelGroupNode = {
      kind: "group",
      id: "root-group",
      direction: "horizontal",
      displayMode: "split",
      sizes: [30, 70],
      children: [leaf("a"), nested],
    };

    const result = setGroupSizesNode(tree, "nested", [25, 75]) as PanelGroupNode;

    expect(result.sizes).toEqual([30, 70]);
    expect((result.children[1] as PanelGroupNode).sizes).toEqual([25, 75]);
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
      sizes: [34, 33, 33],
      children: [leaf("a"), leaf("b"), leaf("c")],
    };
    const result = closeNode(tree, "b") as PanelGroupNode;
    expect(result.children).toHaveLength(2);
    expect(result.children.map((c) => c.id)).toEqual(["a", "c"]);
  });

  test("selects a surviving tab when closing the active child", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "g",
      direction: "horizontal",
      displayMode: "tabbar-top",
      activeTabId: "b",
      sizes: [34, 33, 33],
      children: [leaf("a"), leaf("b"), leaf("c")],
    };

    const result = closeNode(tree, "b") as PanelGroupNode;

    expect(result.activeTabId).toBe("a");
  });

  test("resets surviving zero-share children equally", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "g",
      direction: "horizontal",
      displayMode: "split",
      sizes: [0, 0, 100],
      children: [leaf("a"), leaf("b"), leaf("c")],
    };

    const result = closeNode(tree, "c") as PanelGroupNode;

    expect(result.sizes).toEqual([50, 50]);
  });

  test("collapses group when 1 child remains", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "g",
      direction: "horizontal",
      displayMode: "split",
      sizes: [50, 50],
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

describe("closeLeafSafe", () => {
  test("resets the sole leaf to empty instead of removing it", () => {
    const tree = leaf("a");
    const result = closeLeafSafe(tree, "a");
    expect(result).not.toBeNull();
    expect(collectLeaves(result)).toHaveLength(1);
    expect(result).toMatchObject({ id: "a", content: { type: "empty" } });
  });

  test("removes a leaf and collapses the split when a sibling remains", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "g",
      direction: "horizontal",
      displayMode: "split",
      sizes: [50, 50],
      children: [leaf("a"), leaf("b")],
    };
    const result = closeLeafSafe(tree, "b");
    expect(result).toMatchObject({ kind: "panel", id: "a" });
  });

  test("never returns null", () => {
    expect(closeLeafSafe(leaf("a"), "a")).not.toBeNull();
  });
});

describe("setContentNode", () => {
  test("updates content on matching leaf", () => {
    const tree = leaf("a");
    const result = setContentNode(tree, "a", { type: "terminal", placement: "slot-1" });
    expect((result as PanelLeaf).content).toEqual({ type: "terminal", placement: "slot-1" });
  });

  test("no-op on non-matching leaf", () => {
    const tree = leaf("a");
    expect(setContentNode(tree, "z", { type: "terminal", placement: "slot-1" })).toBe(tree);
  });
});

describe("resetLeafToEmpty", () => {
  test("resets a bound leaf's content to empty", () => {
    const tree = leaf("a");
    const bound = setContentNode(tree, "a", { type: "terminal", placement: "slot-1" });
    const result = resetLeafToEmpty(bound, "a");
    expect((result as PanelLeaf).content).toEqual({ type: "empty" });
  });

  test("leaves sibling leaves untouched", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "g",
      direction: "horizontal",
      displayMode: "split",
      sizes: [50, 50],
      children: [
        { ...leaf("a"), content: { type: "terminal", placement: "slot-1" } },
        { ...leaf("b"), content: { type: "terminal", placement: "slot-2" } },
      ],
    };
    const result = resetLeafToEmpty(tree, "a") as PanelGroupNode;
    expect(result.children[0]).toMatchObject({ content: { type: "empty" } });
    expect(result.children[1]).toMatchObject({
      content: { type: "terminal", placement: "slot-2" },
    });
  });

  test("no-op when id not found", () => {
    const tree = leaf("a");
    expect(resetLeafToEmpty(tree, "z")).toBe(tree);
  });
});

describe("serialize / deserialize", () => {
  test("round-trips DEFAULT_LAYOUT in the versioned envelope", () => {
    const raw = serializeLayout(DEFAULT_LAYOUT);
    expect(JSON.parse(raw)).toMatchObject({ version: 1 });
    expect(deserializeLayout(raw)).toEqual(DEFAULT_LAYOUT);
  });

  test("Versioned nested geometry restores", () => {
    const nested: PanelGroupNode = {
      kind: "group",
      id: "nested",
      direction: "vertical",
      displayMode: "split",
      sizes: [20, 80],
      children: [leaf("b"), leaf("c")],
    };
    const tree: PanelGroupNode = {
      kind: "group",
      id: "root-group",
      direction: "horizontal",
      displayMode: "split",
      sizes: [35, 65],
      children: [leaf("a"), nested],
    };

    expect(deserializeLayout(serializeLayout(tree))).toEqual(tree);
  });

  test("restores a panel resized to zero share", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "group",
      direction: "horizontal",
      displayMode: "split",
      sizes: [0, 100],
      children: [leaf("a"), leaf("b")],
    };

    expect(deserializeLayout(serializeLayout(tree))).toEqual(tree);
  });

  test("Unversioned layout is rejected", () => {
    expect(() => deserializeLayout(JSON.stringify(DEFAULT_LAYOUT))).toThrow(
      "unsupported layout version",
    );
  });

  test("Invalid child sizes are rejected", () => {
    const invalid = (sizes: unknown) =>
      JSON.stringify({
        version: 1,
        root: {
          kind: "group",
          id: "g",
          direction: "horizontal",
          displayMode: "split",
          sizes,
          children: [leaf("a"), leaf("b")],
        },
      });

    for (const sizes of [undefined, [100], [-1, 101], [0, 0], [Number.NaN, 100], ["50", 50]]) {
      expect(() => deserializeLayout(invalid(sizes))).toThrow("invalid panel sizes");
    }
  });

  test("rejects duplicate node identifiers", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "group",
      direction: "horizontal",
      displayMode: "split",
      sizes: [50, 50],
      children: [leaf("same"), leaf("same")],
    };

    expect(() => deserializeLayout(serializeLayout(tree))).toThrow("duplicate node id");
  });

  test("rejects an empty persisted blob", () => {
    expect(() => deserializeLayout("")).toThrow("invalid layout JSON");
  });

  test("rejects empty and duplicate placement bindings", () => {
    const terminal = (id: string, placement: string): PanelLeaf => ({
      ...leaf(id),
      content: { type: "terminal", placement },
    });
    expect(() => deserializeLayout(serializeLayout(terminal("a", "")))).toThrow(
      "invalid panel placement",
    );

    const duplicate: PanelGroupNode = {
      kind: "group",
      id: "group",
      direction: "horizontal",
      displayMode: "split",
      sizes: [50, 50],
      children: [terminal("a", "same"), terminal("b", "same")],
    };
    expect(() => deserializeLayout(serializeLayout(duplicate))).toThrow("invalid panel placement");
  });

  test("rejects corrupt data", () => {
    expect(() => deserializeLayout("not json")).toThrow();
    expect(() =>
      deserializeLayout(
        '{"version":1,"root":{"kind":"panel","id":"root","title":"","content":{"type":"empty"}}}',
      ),
    ).toThrow("invalid panel title");
    expect(() =>
      deserializeLayout(
        JSON.stringify({
          version: 1,
          root: {
            kind: "group",
            id: "group",
            direction: "horizontal",
            displayMode: "tabbar-top",
            activeTabId: "missing",
            sizes: [50, 50],
            children: [leaf("a"), leaf("b")],
          },
        }),
      ),
    ).toThrow("invalid active tab");
    expect(() => deserializeLayout('{"version":1,"root":{"kind":"invalid"}}')).toThrow();
  });
});

describe("findLeaf", () => {
  test("finds a top-level leaf by id", () => {
    const tree = leaf("a");
    expect(findLeaf(tree, "a")).toBe(tree);
  });

  test("finds a nested leaf by id", () => {
    const tree: PanelGroupNode = {
      kind: "group",
      id: "g",
      direction: "horizontal",
      displayMode: "split",
      children: [leaf("a"), leaf("b")],
      sizes: [50, 50],
    };
    expect(findLeaf(tree, "b")).toMatchObject({ id: "b" });
  });

  test("returns undefined when the id is absent", () => {
    expect(findLeaf(leaf("a"), "z")).toBeUndefined();
  });
});

describe("shouldConfirmClose", () => {
  test("confirms a running terminal leaf when no preference is stored", () => {
    const terminal: PanelLeaf = { ...leaf("a"), content: { type: "terminal", placement: "p1" } };
    expect(shouldConfirmClose(terminal, false, true)).toBe(true);
  });

  test("skips confirmation when the terminal's process has already exited", () => {
    const terminal: PanelLeaf = { ...leaf("a"), content: { type: "terminal", placement: "p1" } };
    expect(shouldConfirmClose(terminal, false, false)).toBe(false);
  });

  test("skips confirmation when don't-ask-again is set", () => {
    const terminal: PanelLeaf = { ...leaf("a"), content: { type: "terminal", placement: "p1" } };
    expect(shouldConfirmClose(terminal, true, true)).toBe(false);
  });

  test("never confirms an empty leaf (no PTY to terminate)", () => {
    expect(shouldConfirmClose(leaf("a"), false, true)).toBe(false);
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
      sizes: [34, 33, 33],
      children: [leaf("a"), leaf("b"), leaf("c")],
    };
    expect(countLeaves(tree)).toBe(3);
  });
});
