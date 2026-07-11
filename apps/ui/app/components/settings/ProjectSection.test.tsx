import { QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { setQueryClient, setReady } from "@tillerd/client-bindings";
/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";

import { makeQueryClient } from "~/lib/queryClient";
import { setActiveProject } from "~/lib/store";

// Project settings (ui-settings-editor "Project settings"): only rendered with an active
// project. The default-template picker is the only write path for DEFAULT_TEMPLATE_KEY; its
// read path (resolveDefaultTemplate/projectSettingsQuery) already ships and is unit-tested in
// lib/newSessionTemplate.test.ts and SessionSidebar.test.tsx. ProjectSection routes writes
// through the real command()/useMutation path (unlike the global settings store), so this
// suite mocks the Tauri invoke boundary directly -- see ProfilesSection.test.tsx for the
// same pattern.

const settingSetCalls: {
  scope: string;
  projectId: string | null;
  key: string;
  valueJson: string;
}[] = [];
const settingResetCalls: { scope: string; projectId: string | null; key: string }[] = [];

let projectSettings: { key: string; value: unknown }[] = [];
const launchTemplates = [{ id: "lt-1", projectId: "proj-1", specVersion: 1, specJson: "{}" }];
const libraryTemplates = [
  {
    id: "tpl-1",
    name: "Node service",
    origin: "custom",
    pinned: false,
    specVersion: 1,
    specJson: "{}",
  },
];

import { beforeEach } from "bun:test";

beforeEach(() => {
  (globalThis as any).__tillerd_set_invoke_mock(
    async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "settings_resolve") return projectSettings;
      if (cmd === "launch_template_list") return launchTemplates;
      if (cmd === "template_list") return libraryTemplates;
      if (cmd === "command_list") return [];
      if (cmd === "setting_set") {
        settingSetCalls.push(args as (typeof settingSetCalls)[number]);
        return null;
      }
      if (cmd === "setting_reset") {
        settingResetCalls.push(args as (typeof settingResetCalls)[number]);
        return null;
      }
      return undefined;
    },
  );
});

const { ProjectSection } = await import("./ProjectSection");

afterEach(() => {
  cleanup();
  setActiveProject(null);
  settingSetCalls.length = 0;
  settingResetCalls.length = 0;
  projectSettings = [];
  setReady(false);
  (globalThis as any).__tillerd_clear_invoke_mock();
});

function renderSection() {
  setActiveProject("proj-1");
  const client = makeQueryClient();
  setQueryClient(client);
  setReady(true);
  return render(
    <QueryClientProvider client={client}>
      <ProjectSection />
    </QueryClientProvider>,
  );
}

function pointerActivate(el: HTMLElement): void {
  fireEvent.pointerDown(el, { button: 0 });
  fireEvent.mouseDown(el, { button: 0 });
  fireEvent.pointerUp(el, { button: 0 });
  fireEvent.mouseUp(el, { button: 0 });
  fireEvent.click(el);
}

describe("ProjectSection", () => {
  test("renders nothing without an active project", () => {
    const client = makeQueryClient();
    setQueryClient(client);
    render(
      <QueryClientProvider client={client}>
        <ProjectSection />
      </QueryClientProvider>,
    );
    expect(screen.queryByLabelText("Default template")).toBeNull();
  });

  test("defaults to None with no configured default", async () => {
    renderSection();
    const trigger = await screen.findByLabelText("Default template");
    await waitFor(() => expect(trigger.textContent).toContain("None"));
  });

  test("selecting the project's own launch template persists a tagged launch selection", async () => {
    renderSection();

    const trigger = await screen.findByLabelText("Default template");
    await waitFor(() => expect(trigger.textContent).toContain("None"));

    pointerActivate(trigger);
    await waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
    const option = await screen.findByText("(invalid spec)");
    pointerActivate(option);

    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({
          scope: "project",
          projectId: "proj-1",
          key: "default.template",
          valueJson: JSON.stringify({ kind: "launch", id: "lt-1" }),
        }),
      ),
    );
  });

  test("selecting a library template persists a tagged library selection", async () => {
    renderSection();

    const trigger = await screen.findByLabelText("Default template");
    await waitFor(() => expect(trigger.textContent).toContain("None"));

    pointerActivate(trigger);
    await waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
    const option = await screen.findByText("Node service");
    pointerActivate(option);

    await waitFor(() =>
      expect(settingSetCalls).toContainEqual(
        expect.objectContaining({
          scope: "project",
          projectId: "proj-1",
          key: "default.template",
          valueJson: JSON.stringify({ kind: "library", id: "tpl-1" }),
        }),
      ),
    );
  });

  test("selecting None clears an existing default", async () => {
    projectSettings = [{ key: "default.template", value: { kind: "library", id: "tpl-1" } }];
    renderSection();

    const trigger = await screen.findByLabelText("Default template");
    await waitFor(() => expect(trigger.textContent).toContain("Node service"));

    pointerActivate(trigger);
    await waitFor(() => expect(trigger.getAttribute("aria-expanded")).toBe("true"));
    const option = await screen.findByText("None");
    pointerActivate(option);

    await waitFor(() =>
      expect(settingResetCalls).toContainEqual(
        expect.objectContaining({ scope: "project", projectId: "proj-1", key: "default.template" }),
      ),
    );
  });
});
