import type { CommandView, LaunchTemplateView, TemplateView } from "@tillerd/client-bindings";

import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { setQueryClient, setReady } from "@tillerd/client-bindings";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

import { TooltipProvider } from "~/components/ui/tooltip";
import { CommandRegistryProvider } from "~/lib/commands/registry";
import {
  CURRENT_SPEC_VERSION,
  emptySpec,
  serializeLaunchSpec,
  type LaunchSpec,
} from "~/lib/launchSpec";
import { makeQueryClient } from "~/lib/queryClient";
import { resetUiStore, setActiveProject } from "~/lib/store";

// Template manager: portable library section + (when a project is active) that project's launch
// templates, the visual spec editor, and import/export -- exercised through the real registry +
// EntityContextMenu, mocked at the Tauri invoke boundary.

// mock.module is process-global; spread the real module so `desktopHostStore` (imported by
// other suites, e.g. NotificationIndicator) survives once this mock is installed.
const actualDesktopHost = await import("~/lib/useDesktopHost");
void mock.module("~/lib/useDesktopHost", () => ({
  ...actualDesktopHost,
  useDesktopHost: () => ({ status: "ready" }),
}));

const PROJECT_ID = "proj-1";

let commands: CommandView[] = [];
let templates: TemplateView[] = [];
let launchTemplates: LaunchTemplateView[] = [];
const calls: { cmd: string; args: unknown }[] = [];

function makeCommand(overrides: Partial<CommandView> = {}): CommandView {
  return {
    id: "cmd-1",
    name: "Build",
    origin: "custom",
    cli: "npm run build",
    args: [],
    env: {},
    pinned: false,
    ...overrides,
  };
}

function makeTemplate(overrides: Partial<TemplateView> = {}): TemplateView {
  return {
    id: "t-1",
    name: "My Template",
    origin: "custom",
    pinned: false,
    specVersion: CURRENT_SPEC_VERSION,
    specJson: serializeLaunchSpec(emptySpec()),
    ...overrides,
  };
}

function specWithCommand(commandId: string): LaunchSpec {
  return {
    version: CURRENT_SPEC_VERSION,
    items: [{ target: "terminal", command: { library_ref: commandId } }],
  };
}

void mock.module("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string, args?: Record<string, unknown>) => {
    calls.push({ cmd, args });
    if (cmd === "command_list") return commands;
    if (cmd === "template_list") return templates;
    if (cmd === "launch_template_list") {
      const projectId = args?.["projectId"] as string;
      return launchTemplates.filter((t) => t.projectId === projectId);
    }
    if (cmd === "template_pin") {
      const id = args?.["id"] as string;
      templates = templates.map((t) => (t.id === id ? { ...t, pinned: true } : t));
      return null;
    }
    if (cmd === "template_unpin") {
      const id = args?.["id"] as string;
      templates = templates.map((t) => (t.id === id ? { ...t, pinned: false } : t));
      return null;
    }
    if (cmd === "template_discard") {
      const id = args?.["id"] as string;
      templates = templates.filter((t) => t.id !== id);
      return null;
    }
    if (cmd === "template_export") {
      return null;
    }
    if (cmd === "template_import") {
      const created = makeTemplate({
        id: `t-${templates.length + 1}`,
        name: args?.["name"] as string,
        specJson: args?.["specJson"] as string,
        specVersion: args?.["specVersion"] as number,
      });
      templates = [...templates, created];
      return null;
    }
    if (cmd === "launch_template_create") {
      const created: LaunchTemplateView = {
        id: `lt-${launchTemplates.length + 1}`,
        projectId: args?.["projectId"] as string,
        specVersion: args?.["specVersion"] as number,
        specJson: args?.["specJson"] as string,
      };
      launchTemplates = [...launchTemplates, created];
      return created;
    }
    if (cmd === "launch_template_apply_spec") {
      const id = args?.["id"] as string;
      launchTemplates = launchTemplates.map((t) =>
        t.id === id
          ? {
              ...t,
              specVersion: args?.["specVersion"] as number,
              specJson: args?.["specJson"] as string,
            }
          : t,
      );
      return null;
    }
    if (cmd === "launch_template_discard") {
      const id = args?.["id"] as string;
      launchTemplates = launchTemplates.filter((t) => t.id !== id);
      return null;
    }
    return null;
  },
  Channel: class Channel {
    onmessage: ((v: unknown) => void) | null = null;
  },
}));

const { TemplatesView } = await import("./TemplatesView");

function renderView() {
  const queryClient = makeQueryClient();
  setQueryClient(queryClient);
  setReady(true);
  return render(
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>
        <CommandRegistryProvider>
          <React.Suspense fallback={null}>
            <TemplatesView />
          </React.Suspense>
        </CommandRegistryProvider>
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  commands = [];
  templates = [];
  launchTemplates = [];
  calls.length = 0;
  setReady(false);
  resetUiStore();
});

describe("library section", () => {
  test("lists templates with origin and pinned state", async () => {
    templates = [
      makeTemplate({ id: "t-1", name: "Prebuilt Tmpl", origin: "prebuilt" }),
      makeTemplate({ id: "t-2", name: "Custom Tmpl", origin: "custom" }),
    ];
    renderView();

    await waitFor(() => expect(screen.queryByText("Prebuilt Tmpl")).not.toBeNull());
    expect(screen.queryByText("Custom Tmpl")).not.toBeNull();
    expect(screen.getAllByTestId("template-origin-badge")).toHaveLength(2);
  });

  test("a prebuilt template's context menu offers no Delete", async () => {
    templates = [makeTemplate({ id: "t-1", name: "Prebuilt Tmpl", origin: "prebuilt" })];
    renderView();
    await waitFor(() => expect(screen.queryByText("Prebuilt Tmpl")).not.toBeNull());

    fireEvent.contextMenu(screen.getByTestId("template-row"));

    await waitFor(() => expect(screen.queryByText("Pin")).not.toBeNull());
    expect(screen.queryByText("Export")).not.toBeNull();
    expect(screen.queryByText("Delete")).toBeNull();
  });

  test("no project active hides the project section", async () => {
    renderView();
    await waitFor(() => expect(screen.queryByTestId("templates-empty")).not.toBeNull());
    expect(screen.queryByTestId("launch-template-create-button")).toBeNull();
  });
});

describe("project launch templates", () => {
  test("an active project shows its launch templates section", async () => {
    setActiveProject(PROJECT_ID);
    commands = [makeCommand({ id: "cmd-1", name: "Build" })];
    launchTemplates = [
      {
        id: "lt-1",
        projectId: PROJECT_ID,
        specVersion: CURRENT_SPEC_VERSION,
        specJson: serializeLaunchSpec(specWithCommand("cmd-1")),
      },
    ];
    renderView();

    await waitFor(() => expect(screen.queryByTestId("launch-templates-list")).not.toBeNull());
    expect(screen.queryByText("Build")).not.toBeNull();
  });

  test("creating a launch item with a picked command applies through launch_template_create", async () => {
    setActiveProject(PROJECT_ID);
    commands = [makeCommand({ id: "cmd-1", name: "Build" })];
    renderView();
    await waitFor(() =>
      expect(screen.queryByTestId("launch-template-create-button")).not.toBeNull(),
    );

    fireEvent.click(screen.getByTestId("launch-template-create-button"));
    await waitFor(() => expect(screen.queryByTestId("spec-editor-dialog")).not.toBeNull());

    fireEvent.click(screen.getByTestId("spec-add-item"));
    fireEvent.change(screen.getByTestId("spec-item-command"), { target: { value: "cmd-1" } });
    fireEvent.click(screen.getByTestId("spec-editor-save"));

    await waitFor(() => expect(calls.some((c) => c.cmd === "launch_template_create")).toBe(true));
    const created = calls.find((c) => c.cmd === "launch_template_create");
    if (!created) throw new Error("launch_template_create was not called");
    expect(JSON.parse((created.args as { specJson: string }).specJson)).toEqual({
      version: CURRENT_SPEC_VERSION,
      items: [{ target: "terminal", command: { library_ref: "cmd-1" } }],
    });
  });

  test("saving an item without a picked command is rejected inline", async () => {
    setActiveProject(PROJECT_ID);
    renderView();
    await waitFor(() =>
      expect(screen.queryByTestId("launch-template-create-button")).not.toBeNull(),
    );

    fireEvent.click(screen.getByTestId("launch-template-create-button"));
    fireEvent.click(screen.getByTestId("spec-add-item"));
    fireEvent.click(screen.getByTestId("spec-editor-save"));

    expect(screen.queryByTestId("spec-editor-errors")).not.toBeNull();
    expect(calls.some((c) => c.cmd === "launch_template_create")).toBe(false);
  });

  // Confirmation round-trip (AlertDialog + mutation settle) already runs close to Bun's 5s
  // default even before this sweep; the added row Tooltip pushes it closer on a loaded
  // machine, so this one test gets a longer timeout rather than the whole suite.
  test("discarding a launch template removes it from the project", async () => {
    setActiveProject(PROJECT_ID);
    commands = [makeCommand({ id: "cmd-1", name: "Build" })];
    launchTemplates = [
      {
        id: "lt-1",
        projectId: PROJECT_ID,
        specVersion: CURRENT_SPEC_VERSION,
        specJson: serializeLaunchSpec(specWithCommand("cmd-1")),
      },
    ];
    renderView();
    await waitFor(() => expect(screen.queryByText("Build")).not.toBeNull());

    fireEvent.click(screen.getByLabelText("Discard Build"));
    await waitFor(() =>
      expect(screen.queryByTestId("launch-template-discard-confirm")).not.toBeNull(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Discard" }));

    await waitFor(() => expect(screen.queryByText("Build")).toBeNull());
  }, 10000);
});

describe("import", () => {
  test("importing a valid spec file collects a name and adds it to the library", async () => {
    renderView();
    await waitFor(() => expect(screen.queryByTestId("template-import-button")).not.toBeNull());

    const file = new File([serializeLaunchSpec(emptySpec())], "my-template.json", {
      type: "application/json",
    });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() => expect(screen.queryByTestId("template-import-dialog")).not.toBeNull());
    expect((screen.getByTestId("template-import-name") as HTMLInputElement).value).toBe(
      "my-template",
    );

    fireEvent.click(screen.getByTestId("template-import-confirm"));

    await waitFor(() => expect(screen.queryByText("my-template")).not.toBeNull());
  });
});

describe("export", () => {
  test("exporting writes to the supplied destination path", async () => {
    templates = [makeTemplate({ id: "t-1", name: "My Template" })];
    renderView();
    await waitFor(() => expect(screen.queryByText("My Template")).not.toBeNull());

    fireEvent.click(screen.getByLabelText("Export My Template"));
    await waitFor(() => expect(screen.queryByTestId("template-export-dialog")).not.toBeNull());

    fireEvent.change(screen.getByTestId("template-export-path"), {
      target: { value: "/tmp/my-template.json" },
    });
    fireEvent.click(screen.getByTestId("template-export-confirm"));

    await waitFor(() =>
      expect(calls).toContainEqual({
        cmd: "template_export",
        args: { id: "t-1", destPath: "/tmp/my-template.json" },
      }),
    );
  });
});
