import type { CommandView, LaunchTemplateView, TemplateView } from "@tillerd/client-bindings";

import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { setQueryClient, setReady } from "@tillerd/client-bindings";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

import { CURRENT_SPEC_VERSION, emptySpec, serializeLaunchSpec } from "~/lib/launchSpec";
import { makeQueryClient } from "~/lib/queryClient";

import { NewSessionTemplateDialog } from "./NewSessionTemplateDialog";

let commands: CommandView[] = [];
let templates: TemplateView[] = [];
let launchTemplates: LaunchTemplateView[] = [];

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

function makeLaunchTemplate(overrides: Partial<LaunchTemplateView> = {}): LaunchTemplateView {
  return {
    id: "lt-1",
    projectId: "proj-1",
    specVersion: CURRENT_SPEC_VERSION,
    specJson: serializeLaunchSpec(emptySpec()),
    ...overrides,
  };
}

import { beforeEach } from "bun:test";

beforeEach(() => {
  (globalThis as any).__tillerd_active_invoke = async (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "command_list") return commands;
    if (cmd === "template_list") return templates;
    if (cmd === "launch_template_list") {
      const projectId = args?.["projectId"] as string;
      return launchTemplates.filter((t) => t.projectId === projectId);
    }
    return undefined;
  };
});

function renderDialog(props: Partial<React.ComponentProps<typeof NewSessionTemplateDialog>> = {}) {
  const queryClient = makeQueryClient();
  setQueryClient(queryClient);
  setReady(true);
  return render(
    <QueryClientProvider client={queryClient}>
      <NewSessionTemplateDialog
        target={{ projectId: "proj-1", projectName: "Demo" }}
        onCancel={() => {}}
        onSelect={() => {}}
        {...props}
      />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  cleanup();
  commands = [];
  templates = [];
  launchTemplates = [];
  setReady(false);
  delete (globalThis as any).__tillerd_active_invoke;
});

describe("NewSessionTemplateDialog", () => {
  test("renders nothing when there is no target", () => {
    const queryClient = makeQueryClient();
    setQueryClient(queryClient);
    setReady(true);
    render(
      <QueryClientProvider client={queryClient}>
        <NewSessionTemplateDialog target={null} onCancel={() => {}} onSelect={() => {}} />
      </QueryClientProvider>,
    );
    expect(screen.queryByTestId("new-session-template-picker")).toBeNull();
  });

  test("always offers an empty session option", async () => {
    renderDialog();
    await waitFor(() => expect(screen.queryByTestId("new-session-option-empty")).not.toBeNull());
  });

  test("selecting empty calls onSelect with the empty selection", async () => {
    let selected: unknown = null;
    renderDialog({ onSelect: (s) => (selected = s) });

    await waitFor(() => expect(screen.queryByTestId("new-session-option-empty")).not.toBeNull());
    fireEvent.click(screen.getByTestId("new-session-option-empty"));

    expect(selected).toEqual({ kind: "empty" });
  });

  test("lists the project's launch templates, labeled by their first item's command", async () => {
    commands = [
      {
        id: "cmd-1",
        name: "Build",
        origin: "custom",
        cli: "npm run build",
        args: [],
        env: {},
        pinned: false,
      },
    ];
    launchTemplates = [
      makeLaunchTemplate({
        id: "lt-1",
        specJson: serializeLaunchSpec({
          version: CURRENT_SPEC_VERSION,
          items: [{ target: "terminal", command: { library_ref: "cmd-1" } }],
        }),
      }),
    ];
    renderDialog();

    await waitFor(() => expect(screen.queryByText("Build")).not.toBeNull());
  });

  test("selecting a launch template calls onSelect with its id", async () => {
    launchTemplates = [makeLaunchTemplate({ id: "lt-1" })];
    let selected: unknown = null;
    renderDialog({ onSelect: (s) => (selected = s) });

    await waitFor(() => expect(screen.queryByTestId("new-session-option-launch")).not.toBeNull());
    fireEvent.click(screen.getByTestId("new-session-option-launch"));

    expect(selected).toEqual({ kind: "launch", id: "lt-1" });
  });

  test("lists library templates by name", async () => {
    templates = [makeTemplate({ id: "tpl-1", name: "Rails App" })];
    renderDialog();

    await waitFor(() => expect(screen.queryByText("Rails App")).not.toBeNull());
  });

  test("selecting a library template calls onSelect with its id", async () => {
    templates = [makeTemplate({ id: "tpl-1", name: "Rails App" })];
    let selected: unknown = null;
    renderDialog({ onSelect: (s) => (selected = s) });

    await waitFor(() => expect(screen.queryByTestId("new-session-option-library")).not.toBeNull());
    fireEvent.click(screen.getByTestId("new-session-option-library"));

    expect(selected).toEqual({ kind: "library", id: "tpl-1" });
  });

  test("no project launch templates shows an empty message instead of an empty section", async () => {
    renderDialog();
    await waitFor(() =>
      expect(screen.queryByText("No launch templates for this project")).not.toBeNull(),
    );
  });
});
