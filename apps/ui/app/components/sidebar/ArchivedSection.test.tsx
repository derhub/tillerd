import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, test } from "bun:test";

import { TooltipProvider } from "~/components/ui/tooltip";

import { ArchivedRow, ArchivedSection } from "./ArchivedSection";

function renderArchived(onRestore = () => {}, onDelete = () => {}) {
  render(
    <TooltipProvider>
      <ArchivedSection count={1}>
        <ArchivedRow name="Old session" onRestore={onRestore} onDelete={onDelete} />
      </ArchivedSection>
    </TooltipProvider>,
  );
  fireEvent.click(screen.getByRole("button", { name: "Archived (1)" }));
}

afterEach(cleanup);

test("icon-only button is named", () => {
  renderArchived();
  const restore = screen.getByRole("button", { name: "Restore Old session" });
  expect(restore.getAttribute("aria-label")).toBe("Restore Old session");
  expect(restore.dataset.slot).toBe("tooltip-trigger");
});

test("nested sidebar action is keyboard reachable", () => {
  renderArchived();
  const restore = screen.getByRole("button", { name: "Restore Old session" });
  expect(restore.tabIndex).toBe(0);
  expect(restore.className).toContain("focus-visible:opacity-100");
});

test("nested action follows its disclosure in Tab order", async () => {
  const user = userEvent.setup();
  renderArchived();
  const toggle = screen.getByRole("button", { name: "Archived (1)" });
  const restore = screen.getByRole("button", { name: "Restore Old session" });
  toggle.focus();

  await user.tab();

  expect(document.activeElement).toBe(restore);
});

test("Enter activates the focused nested action", async () => {
  const user = userEvent.setup();
  let restores = 0;
  renderArchived(() => restores++);
  screen.getByRole("button", { name: "Restore Old session" }).focus();

  await user.keyboard("{Enter}");

  expect(restores).toBe(1);
});

test("focus is visible at every chrome stop", () => {
  renderArchived();
  expect(screen.getByRole("button", { name: "Archived (1)" }).className).toContain(
    "focus-visible:ring-1",
  );
});
