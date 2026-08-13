import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, test } from "bun:test";
import React from "react";

import { DeleteDialog, type DeleteTarget } from "./DeleteDialog";

function DialogHost() {
  const [target, setTarget] = React.useState<DeleteTarget | null>(null);
  return (
    <>
      <button
        type="button"
        onClick={() => setTarget({ id: "p-1", name: "Project One", kind: "project" })}
      >
        Open delete dialog
      </button>
      <DeleteDialog target={target} onCancel={() => setTarget(null)} onConfirm={() => {}} />
    </>
  );
}

afterEach(cleanup);

test("dialog focus containment", async () => {
  render(<DialogHost />);
  const trigger = screen.getByRole("button", { name: "Open delete dialog" });
  act(() => trigger.focus());
  fireEvent.click(trigger);

  const dialog = await screen.findByRole("alertdialog");
  await waitFor(() => expect(dialog.contains(document.activeElement)).toBe(true));
  const cancel = screen.getByRole("button", { name: "Cancel" });
  const confirm = screen.getByRole("button", { name: "Delete" });
  confirm.focus();
  await userEvent.tab();
  expect(document.activeElement).toBe(cancel);
  await userEvent.tab({ shift: true });
  expect(document.activeElement).toBe(confirm);
  await userEvent.keyboard("{Escape}");

  await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  expect(document.activeElement).toBe(trigger);
});
