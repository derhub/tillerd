import type { CommandView } from "@tillerd/client-bindings";

import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { setQueryClient, setReady } from "@tillerd/client-bindings";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

import type { CommandArgs } from "~/lib/commands/registry";

import { TooltipProvider } from "~/components/ui/tooltip";
import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, RegisterHandlers } from "~/lib/commands/registry";
import { notificationsStore } from "~/lib/notifications/context";
import { makeQueryClient } from "~/lib/queryClient";
import { SessionContext } from "~/lib/sessionContext";

// Command library: list (pinned first, origin badges), create/edit with inline validation,
// rename/duplicate/pin/delete with prebuilt guards -- exercised through the real registry +
// EntityContextMenu, mocked at the Tauri invoke boundary (mirrors ThemesSection/SessionSidebar).

// mock.module is process-global; spread the real module so `desktopHostStore` (imported by
// other suites, e.g. NotificationIndicator) survives once this mock is installed.
const actualDesktopHost = await import("~/lib/useDesktopHost");
void mock.module("~/lib/useDesktopHost", () => ({
  ...actualDesktopHost,
  useDesktopHost: () => ({ status: "ready" }),
}));

let commands: CommandView[] = [];
const calls: { cmd: string; args: unknown }[] = [];
// Captures dispatches of the panel-spawn command that CommandsView routes Run through.
// PanelContent (the real handler, which places the surface into a leaf) is not in this
// tree, so a stand-in handler records the dispatch to prove Run delegates the spawn.
const runDispatches: CommandArgs[] = [];
let failNextCreate = false;

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

void mock.module("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string, args?: Record<string, unknown>) => {
    calls.push({ cmd, args });
    if (cmd === "command_list") return commands;
    if (cmd === "command_create") {
      if (failNextCreate) throw new Error("name already taken");
      const req = args?.["req"] as { name: string; cli: string; args?: string[]; env?: object };
      const created = makeCommand({
        id: `c-${commands.length + 1}`,
        name: req.name,
        cli: req.cli,
        args: req.args ?? [],
        env: (req.env as Record<string, string>) ?? {},
      });
      commands = [...commands, created];
      return created;
    }
    if (cmd === "command_edit") {
      const id = args?.["id"] as string;
      commands = commands.map((c) =>
        c.id === id ? { ...c, cli: args?.["cli"] as string, args: args?.["args"] as string[] } : c,
      );
      return null;
    }
    if (cmd === "command_rename") {
      const id = args?.["id"] as string;
      commands = commands.map((c) => (c.id === id ? { ...c, name: args?.["name"] as string } : c));
      return null;
    }
    if (cmd === "command_duplicate") {
      const source = commands.find((c) => c.id === args?.["id"]);
      if (source) {
        commands = [
          ...commands,
          makeCommand({
            id: `${source.id}-copy`,
            name: args?.["name"] as string,
            origin: "custom",
            cli: source.cli,
          }),
        ];
      }
      return null;
    }
    if (cmd === "command_pin") {
      const id = args?.["id"] as string;
      commands = commands.map((c) => (c.id === id ? { ...c, pinned: true } : c));
      return null;
    }
    if (cmd === "command_unpin") {
      const id = args?.["id"] as string;
      commands = commands.map((c) => (c.id === id ? { ...c, pinned: false } : c));
      return null;
    }
    if (cmd === "command_delete") {
      const id = args?.["id"] as string;
      commands = commands.filter((c) => c.id !== id);
      return null;
    }
    if (cmd === "surface_spawn") return "p-1";
    return null;
  },
  Channel: class Channel {
    onmessage: ((v: unknown) => void) | null = null;
  },
}));

const { CommandsView } = await import("./CommandsView");

function renderView(sessionId: string | null = null) {
  const queryClient = makeQueryClient();
  setQueryClient(queryClient);
  setReady(true);
  return render(
    <QueryClientProvider client={queryClient}>
      <SessionContext value={{ sessionId, status: "", setStatus: () => {} }}>
        <TooltipProvider>
          <CommandRegistryProvider>
            <RegisterHandlers
              handlers={{ [ACTION.surfaceRunCommand]: (args) => runDispatches.push(args ?? {}) }}
            />
            <React.Suspense fallback={null}>
              <CommandsView />
            </React.Suspense>
          </CommandRegistryProvider>
        </TooltipProvider>
      </SessionContext>
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  commands = [];
  calls.length = 0;
  runDispatches.length = 0;
  failNextCreate = false;
  setReady(false);
  notificationsStore.setState(() => ({ items: [], unread: 0 }));
});

describe("listing", () => {
  test("lists prebuilt and custom commands with origin badges", async () => {
    commands = [
      makeCommand({ id: "c-1", name: "Prebuilt Cmd", origin: "prebuilt" }),
      makeCommand({ id: "c-2", name: "Custom Cmd", origin: "custom" }),
    ];
    renderView();

    await waitFor(() => expect(screen.queryByText("Prebuilt Cmd")).not.toBeNull());
    expect(screen.queryByText("Custom Cmd")).not.toBeNull();
    expect(screen.getAllByTestId("command-origin-badge")).toHaveLength(2);
  });

  test("shows an empty state with no commands", async () => {
    renderView();
    await waitFor(() => expect(screen.queryByTestId("commands-empty")).not.toBeNull());
  });
});

describe("prebuilt guard", () => {
  test("a prebuilt row's context menu offers no Edit, Rename, or Delete", async () => {
    commands = [makeCommand({ id: "c-1", name: "Prebuilt Cmd", origin: "prebuilt" })];
    renderView();
    await waitFor(() => expect(screen.queryByText("Prebuilt Cmd")).not.toBeNull());

    fireEvent.contextMenu(screen.getByTestId("command-row"));

    await waitFor(() => expect(screen.queryByText("Duplicate")).not.toBeNull());
    expect(screen.queryByText("Edit")).toBeNull();
    expect(screen.queryByText("Rename")).toBeNull();
    expect(screen.queryByText("Delete")).toBeNull();
    expect(screen.queryByText("Pin")).not.toBeNull();
  });

  test("a prebuilt row has no Edit or Delete hover button, only Duplicate and Pin", async () => {
    commands = [makeCommand({ id: "c-1", name: "Prebuilt Cmd", origin: "prebuilt" })];
    renderView();
    await waitFor(() => expect(screen.queryByText("Prebuilt Cmd")).not.toBeNull());

    expect(screen.queryByLabelText("Edit Prebuilt Cmd")).toBeNull();
    expect(screen.queryByLabelText("Delete Prebuilt Cmd")).toBeNull();
    expect(screen.getByLabelText("Duplicate Prebuilt Cmd")).not.toBeNull();
    expect(screen.getByLabelText("Pin Prebuilt Cmd")).not.toBeNull();
  });

  test("duplicating a prebuilt command creates an editable custom copy", async () => {
    commands = [makeCommand({ id: "c-1", name: "Prebuilt Cmd", origin: "prebuilt" })];
    renderView();
    await waitFor(() => expect(screen.queryByText("Prebuilt Cmd")).not.toBeNull());

    fireEvent.click(screen.getByLabelText("Duplicate Prebuilt Cmd"));

    await waitFor(() => expect(screen.queryByText("Prebuilt Cmd copy")).not.toBeNull());
    expect(commands.find((c) => c.name === "Prebuilt Cmd copy")?.origin).toBe("custom");
  });
});

describe("create", () => {
  test("creating a command with name and CLI adds it to the list without a manual refresh", async () => {
    renderView();
    await waitFor(() => expect(screen.queryByTestId("commands-empty")).not.toBeNull());

    fireEvent.click(screen.getByTestId("command-create-button"));
    fireEvent.change(screen.getByTestId("command-form-name"), { target: { value: "Lint" } });
    fireEvent.change(screen.getByTestId("command-form-cli"), { target: { value: "npm run lint" } });
    fireEvent.click(screen.getByTestId("command-form-save"));

    await waitFor(() => expect(screen.queryByText("Lint")).not.toBeNull());
  });

  test("saving without a name or CLI surfaces inline validation and does not submit", async () => {
    renderView();
    await waitFor(() => expect(screen.queryByTestId("commands-empty")).not.toBeNull());

    fireEvent.click(screen.getByTestId("command-create-button"));
    fireEvent.click(screen.getByTestId("command-form-save"));

    expect(screen.queryByTestId("command-form-name-error")).not.toBeNull();
    expect(screen.queryByTestId("command-form-cli-error")).not.toBeNull();
    expect(calls.some((c) => c.cmd === "command_create")).toBe(false);
  });

  test("a backend rejection is surfaced inline in the form", async () => {
    failNextCreate = true;
    renderView();
    await waitFor(() => expect(screen.queryByTestId("commands-empty")).not.toBeNull());

    fireEvent.click(screen.getByTestId("command-create-button"));
    fireEvent.change(screen.getByTestId("command-form-name"), { target: { value: "Lint" } });
    fireEvent.change(screen.getByTestId("command-form-cli"), { target: { value: "npm run lint" } });
    fireEvent.click(screen.getByTestId("command-form-save"));

    await waitFor(() => expect(screen.queryByTestId("command-form-error")).not.toBeNull());
    expect(screen.getByTestId("command-form-error").textContent).toContain("name already taken");
  });
});

describe("edit", () => {
  test("editing a custom command's arguments persists and renders in the row", async () => {
    commands = [makeCommand({ id: "c-1", name: "Build", cli: "npm run build", args: [] })];
    renderView();
    await waitFor(() => expect(screen.queryByText("Build")).not.toBeNull());

    fireEvent.click(screen.getByLabelText("Edit Build"));
    await waitFor(() => expect(screen.queryByTestId("command-form-dialog")).not.toBeNull());
    expect((screen.getByTestId("command-form-cli") as HTMLInputElement).value).toBe(
      "npm run build",
    );

    fireEvent.click(screen.getByTestId("command-form-add-arg"));
    fireEvent.change(screen.getByTestId("command-form-arg"), { target: { value: "--verbose" } });
    fireEvent.click(screen.getByTestId("command-form-save"));

    await waitFor(() => expect(commands.find((c) => c.id === "c-1")?.args).toEqual(["--verbose"]));
  });
});

describe("delete", () => {
  // Confirmation round-trip (AlertDialog + mutation settle) already ran close to Bun's 5s
  // default even before this sweep; the added row Tooltips push it over on a loaded machine,
  // so this one test gets a longer timeout rather than the whole suite.
  test("deleting a custom command removes it from the list after confirmation", async () => {
    commands = [makeCommand({ id: "c-1", name: "Build" })];
    renderView();
    await waitFor(() => expect(screen.queryByText("Build")).not.toBeNull());

    fireEvent.click(screen.getByLabelText("Delete Build"));
    await waitFor(() => expect(screen.queryByTestId("command-delete-confirm")).not.toBeNull());
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() => expect(screen.queryByText("Build")).toBeNull());
  }, 10000);
});

describe("pin", () => {
  test("pinning a command shows the pinned indicator", async () => {
    commands = [makeCommand({ id: "c-1", name: "Build", pinned: false })];
    renderView();
    await waitFor(() => expect(screen.queryByText("Build")).not.toBeNull());

    fireEvent.click(screen.getByLabelText("Pin Build"));

    await waitFor(() => expect(screen.queryByTestId("command-pinned-indicator")).not.toBeNull());
  });
});

describe("run", () => {
  test("running a command with an active session routes the spawn to the panel", async () => {
    commands = [makeCommand({ id: "c-1", name: "Build", cli: "npm run build" })];
    renderView("s-1");
    await waitFor(() => expect(screen.queryByText("Build")).not.toBeNull());

    fireEvent.click(screen.getByLabelText("Run Build"));

    await waitFor(() => expect(runDispatches).toHaveLength(1));
    expect(runDispatches[0]).toEqual({ commandRef: { libraryRef: "c-1" } });
    // Delegated to the panel handler, never spawned headless from this view.
    expect(calls.some((c) => c.cmd === "surface_spawn")).toBe(false);
  });

  test("running a command with no active session notifies instead of spawning", async () => {
    commands = [makeCommand({ id: "c-1", name: "Build" })];
    renderView(null);
    await waitFor(() => expect(screen.queryByText("Build")).not.toBeNull());

    fireEvent.click(screen.getByLabelText("Run Build"));

    expect(runDispatches).toHaveLength(0);
    expect(calls.some((c) => c.cmd === "surface_spawn")).toBe(false);
    expect(notificationsStore.state.items.some((i) => i.category === "command-run")).toBe(true);
  });

  test("running a prebuilt command is offered too (Run is unguarded by origin)", async () => {
    commands = [makeCommand({ id: "c-1", name: "Prebuilt Cmd", origin: "prebuilt" })];
    renderView("s-1");
    await waitFor(() => expect(screen.queryByText("Prebuilt Cmd")).not.toBeNull());

    expect(screen.getByLabelText("Run Prebuilt Cmd")).not.toBeNull();
  });
});
