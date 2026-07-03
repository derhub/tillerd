import { cleanup, fireEvent, render, screen } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, expect, test, describe } from "bun:test";

import type { WorkspaceSwitcherProps } from "./WorkspaceSwitcher";

import { WorkspaceSwitcherList } from "./WorkspaceSwitcher";

afterEach(cleanup);

function renderSwitcher(over: Partial<WorkspaceSwitcherProps> = {}) {
  const props: WorkspaceSwitcherProps = {
    workspaces: [],
    activeId: null,
    detachedIds: new Set(),
    isDesktop: true,
    onSelect: () => {},
    onNewWorkspace: () => {},
    onDetach: () => {},
    onReattach: () => {},
    editingId: null,
    onStartEdit: () => {},
    onCancelEdit: () => {},
    onRename: () => {},
    ...over,
  };
  return render(<WorkspaceSwitcherList {...props} />);
}

const alpha = { id: "ws-1", name: "Alpha" };
const beta = { id: "ws-2", name: "Beta" };

describe("WorkspaceSwitcherList", () => {
  test("renders a button for each workspace", () => {
    renderSwitcher({ workspaces: [alpha, beta] });
    expect(screen.queryByText("Alpha")).not.toBeNull();
    expect(screen.queryByText("Beta")).not.toBeNull();
  });

  test("calls onSelect with the workspace id when a workspace button is clicked", () => {
    const selected: string[] = [];
    renderSwitcher({ workspaces: [alpha], onSelect: (id) => selected.push(id) });

    fireEvent.click(screen.getByText("Alpha"));

    expect(selected).toEqual(["ws-1"]);
  });

  test("the active workspace button carries the active styling", () => {
    renderSwitcher({ workspaces: [alpha, beta], activeId: "ws-1" });

    const items = screen.queryAllByTestId("workspace-item");
    const activeItem = items.find((el) => el.getAttribute("data-workspace-id") === "ws-1");
    const inactiveItem = items.find((el) => el.getAttribute("data-workspace-id") === "ws-2");

    expect(activeItem?.className).toContain("bg-muted");
    expect(inactiveItem?.className).not.toContain("font-medium");
  });

  test("switching workspaces calls onSelect — no navigation, no new window", () => {
    const selections: string[] = [];
    renderSwitcher({
      workspaces: [alpha, beta],
      activeId: "ws-1",
      onSelect: (id) => selections.push(id),
    });

    fireEvent.click(screen.getByText("Beta"));

    expect(selections).toEqual(["ws-2"]);
  });

  test("clicking the new-workspace control calls onNewWorkspace", () => {
    let clicked = 0;
    renderSwitcher({ onNewWorkspace: () => (clicked += 1) });

    fireEvent.click(screen.getByTestId("new-workspace"));

    expect(clicked).toBe(1);
  });

  test("the new-workspace control is hidden in the web host", () => {
    renderSwitcher({ isDesktop: false });
    expect(screen.queryByTestId("new-workspace")).toBeNull();
  });

  test("clicking workspace-detach calls onDetach with the workspace id", () => {
    const detached: string[] = [];
    renderSwitcher({ workspaces: [alpha], onDetach: (id) => detached.push(id) });

    const detachBtn = screen.getByTestId("workspace-detach");
    expect(detachBtn.getAttribute("data-workspace-id")).toBe("ws-1");
    fireEvent.click(detachBtn);

    expect(detached).toEqual(["ws-1"]);
  });

  test("a detached workspace shows the re-attach indicator and hides the detach control", () => {
    const reattached: string[] = [];
    renderSwitcher({
      workspaces: [alpha],
      detachedIds: new Set(["ws-1"]),
      onReattach: (id) => reattached.push(id),
    });

    expect(screen.queryByTestId("workspace-detach")).toBeNull();
    const indicator = screen.getByTestId("workspace-detached-indicator");
    expect(indicator.getAttribute("data-workspace-id")).toBe("ws-1");

    fireEvent.click(indicator);
    expect(reattached).toEqual(["ws-1"]);
  });

  test("an editing workspace shows the inline rename input and renames on Enter", () => {
    const renamed: [string, string][] = [];
    renderSwitcher({
      workspaces: [alpha],
      editingId: "ws-1",
      onRename: (id, name) => renamed.push([id, name]),
    });
    const input = screen.getByTestId("inline-rename-input");
    fireEvent.change(input, { target: { value: "Renamed" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(renamed).toEqual([["ws-1", "Renamed"]]);
  });
});
