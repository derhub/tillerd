import type { ReactNode } from "react";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterAll, afterEach, beforeEach, describe, expect, mock, test } from "bun:test";

import { delegatingQuery } from "./test/real-bindings";

let active = false;
let layoutJson = "";
let layoutRead: Promise<string>;
const commands: Array<{ key: string; args: { id: string; layoutJson: string } }> = [];
const versionedLayout = JSON.stringify({
  version: 1,
  root: {
    kind: "group",
    id: "group",
    direction: "horizontal",
    displayMode: "split",
    sizes: [30, 70],
    children: [
      { kind: "panel", id: "a", title: "A", content: { type: "empty" } },
      { kind: "panel", id: "b", title: "B", content: { type: "empty" } },
    ],
  },
});

beforeEach(() => {
  active = true;
  layoutJson = JSON.stringify({
    kind: "panel",
    id: "root",
    title: "Empty",
    content: { type: "empty" },
  });
  layoutRead = Promise.resolve(layoutJson);
  commands.length = 0;
});

// Bun must register the module mock before importing the hook that captures these bindings.
const actualBindings = await import("@tillerd/client-bindings");
const realRunCommand = actualBindings.runCommand as unknown as (
  key: string,
  args: unknown,
) => Promise<unknown>;
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  query: delegatingQuery(
    {
      sessionLayoutGet: (_args: unknown) => ({
        queryKey: ["session-layout", "session-1"],
        queryFn: () => layoutRead,
      }),
    },
    () => active,
  ),
  runCommand: (key: string, args: unknown) => {
    if (!active) return realRunCommand(key, args) as never;
    if (
      typeof args !== "object" ||
      args === null ||
      !("id" in args) ||
      typeof args.id !== "string" ||
      !("layoutJson" in args) ||
      typeof args.layoutJson !== "string"
    ) {
      throw new Error("unexpected layout command");
    }
    commands.push({ key, args: { id: args.id, layoutJson: args.layoutJson } });
    return Promise.resolve(null) as never;
  },
}));

const { usePanelTree } = await import("./usePanelTree");

afterEach(() => {
  active = false;
  cleanup();
});

afterAll(() => mock.restore());

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>;
}

describe("usePanelTree", () => {
  test("Unversioned layout is rejected without overwrite", async () => {
    const pendingLayout = Promise.withResolvers<string>();
    layoutRead = pendingLayout.promise;
    const { result } = renderHook(() => usePanelTree("session-1"), { wrapper });

    expect(result.current.layoutPending).toBe(true);
    act(() => {
      result.current.split("root", "horizontal");
    });
    expect(commands).toEqual([]);

    act(() => pendingLayout.resolve(layoutJson));
    await waitFor(() => expect(result.current.layoutError?.name).toBe("LayoutFormatError"));
    expect(result.current.tree).toMatchObject({ kind: "panel", id: "root" });

    act(() => {
      result.current.split("root", "horizontal");
    });
    expect(result.current.tree).toMatchObject({ kind: "panel", id: "root" });
    expect(commands).toEqual([]);
  });

  test("an empty non-null layout is incompatible", async () => {
    layoutRead = Promise.resolve("");
    const { result } = renderHook(() => usePanelTree("session-1"), { wrapper });

    await waitFor(() => expect(result.current.layoutError?.name).toBe("LayoutFormatError"));
  });

  test("Reload restores panel tree", async () => {
    layoutRead = Promise.resolve(versionedLayout);
    const { result } = renderHook(() => usePanelTree("session-1"), { wrapper });

    await waitFor(() => expect(result.current.layoutPending).toBe(false));
    expect(result.current.tree).toMatchObject({ id: "group", sizes: [30, 70] });
  });

  test("Layout written after split", async () => {
    layoutRead = Promise.resolve(versionedLayout);
    const { result } = renderHook(() => usePanelTree("session-1"), { wrapper });
    await waitFor(() => expect(result.current.layoutPending).toBe(false));

    act(() => {
      result.current.split("a", "vertical");
    });

    expect(commands).toHaveLength(1);
    expect(commands[0]?.key).toBe("sessionLayoutSet");
    expect(JSON.parse(commands[0]!.args.layoutJson)).toMatchObject({
      version: 1,
      root: { id: "group", children: [{ kind: "group", sizes: [50, 50] }, { id: "b" }] },
    });
  });

  test("Layout written after resize", async () => {
    layoutRead = Promise.resolve(versionedLayout);
    const { result } = renderHook(() => usePanelTree("session-1"), { wrapper });
    await waitFor(() => expect(result.current.layoutPending).toBe(false));

    act(() => {
      result.current.setGroupSizes("group", [40, 60]);
    });

    expect(commands).toHaveLength(1);
    expect(JSON.parse(commands[0]!.args.layoutJson)).toMatchObject({
      version: 1,
      root: { id: "group", sizes: [40, 60] },
    });
  });
});
