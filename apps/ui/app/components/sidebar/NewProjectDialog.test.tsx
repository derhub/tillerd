import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, test } from "bun:test";
import React from "react";

import { NewProjectDialog } from "./NewProjectDialog";

function Harness({ onCreate }: { onCreate: (name: string) => void }) {
  const [open, setOpen] = React.useState(true);
  return <NewProjectDialog open={open} onOpenChange={setOpen} onCreate={onCreate} />;
}

afterEach(cleanup);

test("Cancel dismisses without invoking project creation", async () => {
  const created: string[] = [];
  render(<Harness onCreate={(name) => created.push(name)} />);

  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));

  await waitFor(() => expect(screen.queryByRole("dialog", { name: "New project" })).toBeNull());
  expect(created).toHaveLength(0);
});

test("confirmation trims the project name before invoking creation", () => {
  const created: string[] = [];
  render(<Harness onCreate={(name) => created.push(name)} />);

  fireEvent.change(screen.getByRole("textbox", { name: "Project name" }), {
    target: { value: "  Alpha  " },
  });
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));

  expect(created).toEqual(["Alpha"]);
});
