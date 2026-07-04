import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

import type { CommandHandler } from "~/lib/commands/registry";

import { resetContext } from "~/lib/commands/context";
import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, RegisterHandlers } from "~/lib/commands/registry";

import { EntityContextMenu, type EntityContextMenuProps } from "./EntityContextMenu";

afterEach(() => {
  cleanup();
  resetContext();
});

function renderMenu(
  handlers: Record<string, CommandHandler>,
  props: Partial<EntityContextMenuProps> & Pick<EntityContextMenuProps, "entityId" | "entityKind">,
) {
  return render(
    <CommandRegistryProvider>
      <RegisterHandlers handlers={handlers} />
      <EntityContextMenu {...props}>
        <div>Row</div>
      </EntityContextMenu>
    </CommandRegistryProvider>,
  );
}

describe("EntityContextMenu", () => {
  test("lists nothing until the row is right-clicked", () => {
    renderMenu(
      { [ACTION.projectRename]: () => {} },
      { entityId: "p-1", entityKind: "project", guards: { "menu.canRename": true } },
    );
    expect(screen.queryByText("Rename")).toBeNull();
  });

  test("opens on a native contextmenu event and lists a scoped, allowed command", async () => {
    renderMenu(
      { [ACTION.projectRename]: () => {} },
      { entityId: "p-1", entityKind: "project", guards: { "menu.canRename": true } },
    );

    fireEvent.contextMenu(screen.getByText("Row"));

    await waitFor(() => expect(screen.queryByText("Rename")).not.toBeNull());
  });

  test("excludes a command scoped to a different row kind", async () => {
    renderMenu(
      { [ACTION.projectRename]: () => {}, [ACTION.sessionRename]: () => {} },
      { entityId: "p-1", entityKind: "project", guards: { "menu.canRename": true } },
    );

    fireEvent.contextMenu(screen.getByText("Row"));

    // menu.sessionRow never becomes truthy while this project row's menu is
    // open, so session.rename's `when` fails and only the project's shows.
    await waitFor(() => expect(screen.queryAllByText("Rename")).toHaveLength(1));
  });

  test("hides an item when its per-row guard fails", async () => {
    renderMenu(
      { [ACTION.projectRename]: () => {} },
      { entityId: "p-1", entityKind: "project", guards: { "menu.canRename": false } },
    );

    fireEvent.contextMenu(screen.getByText("Row"));

    // The trigger itself always renders; only the gated item is absent.
    await waitFor(() => expect(screen.queryByText("Row")).not.toBeNull());
    expect(screen.queryByText("Rename")).toBeNull();
  });

  test("invokes the handler with the entity id, kind, and extra args", async () => {
    const spy = mock((_args?: unknown) => {});
    renderMenu(
      { [ACTION.sessionArchive]: spy },
      { entityId: "s-9", entityKind: "session", args: { label: "S9" } },
    );

    fireEvent.contextMenu(screen.getByText("Row"));
    const item = await screen.findByText("Archive");
    fireEvent.click(item);

    expect(spy).toHaveBeenCalledWith({ entityId: "s-9", entityKind: "session", label: "S9" });
  });

  test("a disabled menu renders its children without a context-menu wrapper", () => {
    renderMenu(
      { [ACTION.projectRename]: () => {} },
      {
        entityId: "p-1",
        entityKind: "project",
        guards: { "menu.canRename": true },
        disabled: true,
      },
    );

    fireEvent.contextMenu(screen.getByText("Row"));
    expect(screen.queryByText("Rename")).toBeNull();
  });
});
