import { cleanup, fireEvent, render, screen } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";

import { EmptyPanel } from "./EmptyPanel";

afterEach(cleanup);

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
});
