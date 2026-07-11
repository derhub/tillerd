import {
  RouterProvider,
  createMemoryHistory,
  createRoute,
  createRootRoute,
  createRouter,
} from "@tanstack/react-router";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

// Section nav only: switching sections is a client-side search-param nav (`?section=`), never
// a full reload, and each section is independently swappable -- stub every section component so
// this test exercises only SettingsEditor's own routing/nav logic.
void mock.module("~/components/settings/GeneralSection", () => ({
  GeneralSection: () => <div data-testid="pane-general" />,
}));
void mock.module("~/components/settings/AppearanceSection", () => ({
  AppearanceSection: () => <div data-testid="pane-appearance" />,
}));
void mock.module("~/components/settings/TerminalSection", () => ({
  TerminalSection: () => <div data-testid="pane-terminal" />,
}));
void mock.module("~/components/settings/KeybindingSettings", () => ({
  KeybindingSettings: () => <div data-testid="pane-keybindings" />,
}));
void mock.module("~/components/settings/ProfilesSection", () => ({
  ProfilesSection: () => <div data-testid="pane-profiles" />,
}));
void mock.module("~/components/settings/ThemesSection", () => ({
  ThemesSection: () => <div data-testid="pane-themes" />,
}));
void mock.module("~/components/settings/ProjectSection", () => ({
  ProjectSection: () => <div data-testid="pane-project" />,
}));

const { SettingsEditor } = await import("./SettingsEditor");
const { setActiveProject } = await import("~/lib/store");

afterEach(() => {
  cleanup();
  setActiveProject(null);
  mock.restore();
});

// Root's real validateSearch is a passthrough (see routes/__root.tsx); mirrored here so
// `?section=` round-trips the same way it does in the app.
function passthroughSearch(raw: Record<string, unknown>): Record<string, string> {
  const search: Record<string, string> = {};
  for (const [k, v] of Object.entries(raw)) if (typeof v === "string") search[k] = v;
  return search;
}

function renderEditor(initialEntry = "/settings") {
  const rootRoute = createRootRoute({ validateSearch: passthroughSearch });
  const settingsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/settings",
    component: SettingsEditor,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([settingsRoute]),
    history: createMemoryHistory({ initialEntries: [initialEntry] }),
  });
  render(<RouterProvider router={router as never} />);
  return router;
}

describe("SettingsEditor", () => {
  test("defaults to the Appearance section", async () => {
    renderEditor();
    await waitFor(() => expect(screen.queryByTestId("pane-appearance")).not.toBeNull());
    expect(screen.queryByTestId("pane-terminal")).toBeNull();
  });

  test("selecting Keybindings swaps the pane without a full reload", async () => {
    renderEditor();
    await waitFor(() => expect(screen.queryByTestId("pane-appearance")).not.toBeNull());

    screen.getByTestId("settings-section-keybindings").click();

    await waitFor(() => expect(screen.queryByTestId("pane-keybindings")).not.toBeNull());
    expect(screen.queryByTestId("pane-appearance")).toBeNull();
  });

  test("the active section button carries aria-current", async () => {
    renderEditor();
    await waitFor(() =>
      expect(screen.getByTestId("settings-section-appearance").getAttribute("aria-current")).toBe(
        "page",
      ),
    );
    expect(screen.getByTestId("settings-section-profiles").getAttribute("aria-current")).toBeNull();

    screen.getByTestId("settings-section-profiles").click();

    await waitFor(() =>
      expect(screen.getByTestId("settings-section-profiles").getAttribute("aria-current")).toBe(
        "page",
      ),
    );
  });

  test("deep-links to a section via the ?section= search param", async () => {
    renderEditor("/settings?section=themes");
    await waitFor(() => expect(screen.queryByTestId("pane-themes")).not.toBeNull());
  });

  test("Project only appears in the nav with an active project", async () => {
    renderEditor();
    await waitFor(() => expect(screen.queryByTestId("pane-appearance")).not.toBeNull());
    expect(screen.queryByTestId("settings-section-project")).toBeNull();

    await act(async () => {
      setActiveProject("proj-1");
    });

    await waitFor(() => expect(screen.queryByTestId("settings-section-project")).not.toBeNull());
  });
});
