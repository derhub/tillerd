import { cleanup, fireEvent, render, screen } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, expect, test } from "bun:test";

import type { SearchQueryOptions, TerminalSearchController } from "./TerminalSearchOverlay";

import { TerminalSearchOverlay } from "./TerminalSearchOverlay";

afterEach(cleanup);

function fakeController() {
  const calls: { method: "next" | "prev" | "clear"; query?: string; opts?: SearchQueryOptions }[] =
    [];
  const controller: TerminalSearchController = {
    findNext: (query, opts) => calls.push({ method: "next", query, opts }),
    findPrevious: (query, opts) => calls.push({ method: "prev", query, opts }),
    clear: () => calls.push({ method: "clear" }),
  };
  return { controller, calls };
}

test("focuses the search input on open", () => {
  const { controller } = fakeController();
  render(<TerminalSearchOverlay controller={controller} results={null} onClose={() => {}} />);
  expect(document.activeElement).toBe(screen.getByTestId("terminal-search-input"));
});

test("an entered query runs an incremental case-insensitive search", () => {
  const { controller, calls } = fakeController();
  render(<TerminalSearchOverlay controller={controller} results={null} onClose={() => {}} />);
  fireEvent.change(screen.getByTestId("terminal-search-input"), { target: { value: "boom" } });
  const last = calls.at(-1);
  expect(last?.method).toBe("next");
  expect(last?.query).toBe("boom");
  expect(last?.opts?.caseSensitive).toBe(false);
});

test("clearing the query clears decorations", () => {
  const { controller, calls } = fakeController();
  render(
    <TerminalSearchOverlay
      controller={controller}
      results={null}
      initialQuery="x"
      onClose={() => {}}
    />,
  );
  fireEvent.change(screen.getByTestId("terminal-search-input"), { target: { value: "" } });
  expect(calls.at(-1)?.method).toBe("clear");
});

test("the next and previous buttons step matches", () => {
  const { controller, calls } = fakeController();
  render(
    <TerminalSearchOverlay
      controller={controller}
      results={null}
      initialQuery="a"
      onClose={() => {}}
    />,
  );
  fireEvent.click(screen.getByTestId("terminal-search-prev"));
  expect(calls.at(-1)?.method).toBe("prev");
  fireEvent.click(screen.getByTestId("terminal-search-next"));
  expect(calls.at(-1)?.method).toBe("next");
});

test("toggling case sensitivity re-runs the search case-sensitively", () => {
  const { controller, calls } = fakeController();
  render(
    <TerminalSearchOverlay
      controller={controller}
      results={null}
      initialQuery="a"
      onClose={() => {}}
    />,
  );
  fireEvent.click(screen.getByTestId("terminal-search-case"));
  expect(calls.at(-1)?.opts?.caseSensitive).toBe(true);
});

test("Escape dismisses the overlay", () => {
  const { controller } = fakeController();
  let closed = false;
  render(
    <TerminalSearchOverlay
      controller={controller}
      results={null}
      onClose={() => (closed = true)}
    />,
  );
  fireEvent.keyDown(screen.getByTestId("terminal-search-input"), { key: "Escape" });
  expect(closed).toBe(true);
});

test("shows the active match position over the total count", () => {
  const { controller } = fakeController();
  render(
    <TerminalSearchOverlay
      controller={controller}
      results={{ resultIndex: 2, resultCount: 5 }}
      initialQuery="a"
      onClose={() => {}}
    />,
  );
  expect(screen.getByTestId("terminal-search-count").textContent).toBe("3/5");
});

test("shows no-results for a non-empty query with zero matches", () => {
  const { controller } = fakeController();
  render(
    <TerminalSearchOverlay
      controller={controller}
      results={{ resultIndex: -1, resultCount: 0 }}
      initialQuery="zzz"
      onClose={() => {}}
    />,
  );
  expect(screen.getByTestId("terminal-search-count").textContent).toBe("No results");
});

test("runs an initial query immediately when opened from a selection", () => {
  const { controller, calls } = fakeController();
  render(
    <TerminalSearchOverlay
      controller={controller}
      results={null}
      initialQuery="preset"
      onClose={() => {}}
    />,
  );
  expect(calls.some((c) => c.method === "next" && c.query === "preset")).toBe(true);
});
