import type { CommandView } from "@tillerd/client-bindings";

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";

import { EmptyPanel } from "./EmptyPanel";

afterEach(cleanup);

function makeCommand(overrides: Partial<CommandView> = {}): CommandView {
  return {
    id: "c-1",
    name: "Build",
    origin: "custom",
    cli: "npm run build",
    args: [],
    env: {},
    pinned: false,
    ...overrides,
  };
}

describe("EmptyPanel picker", () => {
  test("lists the terminal kind and keeps the 'New terminal' label the e2e helper matches on", () => {
    render(<EmptyPanel onSpawn={() => {}} />);
    expect(document.querySelector('[data-testid="empty-panel-picker"]')).not.toBeNull();
    const card = document.querySelector('[data-testid="empty-panel-kind-terminal"]');
    expect(card).not.toBeNull();
    expect(card?.textContent).toContain("New terminal");
  });

  test("picking the terminal kind spawns into the leaf", () => {
    const spy = mock(() => {});
    render(<EmptyPanel onSpawn={spy} />);
    fireEvent.click(screen.getByTestId("empty-panel-kind-terminal"));
    expect(spy).toHaveBeenCalledTimes(1);
  });

  test("disabled (no active session) the kind card cannot be picked", () => {
    const spy = mock(() => {});
    render(<EmptyPanel onSpawn={spy} disabled />);
    const card = screen.getByTestId("empty-panel-kind-terminal") as HTMLButtonElement;
    expect(card.disabled).toBe(true);
  });

  test("lists command-library entries pinned first, after the terminal kind", () => {
    const commands = [
      makeCommand({ id: "c-unpinned", name: "Lint", pinned: false }),
      makeCommand({ id: "c-pinned", name: "Build", pinned: true }),
    ];
    render(<EmptyPanel onSpawn={() => {}} commands={commands} />);
    const list = screen.getByTestId("empty-panel-picker");
    const ids = Array.from(list.querySelectorAll("button")).map((b) => b.dataset["testid"]);
    expect(ids).toEqual([
      "empty-panel-kind-terminal",
      "empty-panel-command-c-pinned",
      "empty-panel-command-c-unpinned",
    ]);
  });

  test("picking a command-library entry spawns a terminal running that command", () => {
    const spy = mock(() => {});
    const commands = [makeCommand({ id: "c-1", name: "Build" })];
    render(<EmptyPanel onSpawn={spy} commands={commands} />);
    fireEvent.click(screen.getByTestId("empty-panel-command-c-1"));
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy).toHaveBeenCalledWith({ libraryRef: "c-1" });
  });

  test("disabled (no active session) command-library entries cannot be picked either", () => {
    const commands = [makeCommand({ id: "c-1", name: "Build" })];
    render(<EmptyPanel onSpawn={() => {}} commands={commands} disabled />);
    const card = screen.getByTestId("empty-panel-command-c-1") as HTMLButtonElement;
    expect(card.disabled).toBe(true);
  });
});
