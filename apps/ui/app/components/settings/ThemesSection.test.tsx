import { QueryClientProvider } from "@tanstack/react-query";
import { setQueryClient, setReady } from "@tillerd/client-bindings";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

import { makeQueryClient } from "~/lib/queryClient";

// Theme management: list with active indicated, activate/export/delete -- prebuilt themes are
// never deletable (no affordance at all, per spec, not merely a disabled button).

let themes: { id: string; name: string; origin: "prebuilt" | "custom" }[] = [
  { id: "t-1", name: "Default Dark", origin: "prebuilt" },
  { id: "t-2", name: "My Theme", origin: "custom" },
];
let activeId: string | null = "t-1";
const activated: string[] = [];
const discarded: string[] = [];

void mock.module("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === "theme_list") return themes;
    if (cmd === "theme_get_active") return themes.find((t) => t.id === activeId) ?? null;
    if (cmd === "theme_activate") {
      activated.push(args?.["id"] as string);
      activeId = args?.["id"] as string;
      return null;
    }
    if (cmd === "theme_discard") {
      discarded.push(args?.["id"] as string);
      themes = themes.filter((t) => t.id !== args?.["id"]);
      return null;
    }
    return null;
  },
  Channel: class Channel {
    onmessage: ((v: unknown) => void) | null = null;
  },
}));

const { ThemesSection } = await import("./ThemesSection");

afterEach(() => {
  cleanup();
  mock.restore();
  themes = [
    { id: "t-1", name: "Default Dark", origin: "prebuilt" },
    { id: "t-2", name: "My Theme", origin: "custom" },
  ];
  activeId = "t-1";
  activated.length = 0;
  discarded.length = 0;
  setReady(false);
});

function renderSection() {
  const client = makeQueryClient();
  setQueryClient(client);
  setReady(true);
  return render(
    <QueryClientProvider client={client}>
      <ThemesSection />
    </QueryClientProvider>,
  );
}

describe("ThemesSection", () => {
  test("lists themes with the active one badged", async () => {
    renderSection();
    await waitFor(() => expect(screen.queryByText("Default Dark")).not.toBeNull());
    expect(screen.getByTestId("theme-active-badge")).not.toBeNull();
  });

  test("a prebuilt theme offers no delete affordance", async () => {
    renderSection();
    await waitFor(() => expect(screen.queryByText("Default Dark")).not.toBeNull());
    expect(screen.queryByLabelText("Delete Default Dark")).toBeNull();
    expect(screen.getByLabelText("Delete My Theme")).not.toBeNull();
  });

  test("activating a custom theme persists it as active", async () => {
    renderSection();
    await waitFor(() => expect(screen.queryByText("My Theme")).not.toBeNull());

    act(() => {
      screen.getByText("My Theme").click();
    });

    await waitFor(() => expect(activated).toEqual(["t-2"]));
  });

  test("deleting a custom theme discards it", async () => {
    renderSection();
    await waitFor(() => expect(screen.queryByText("My Theme")).not.toBeNull());

    act(() => {
      screen.getByLabelText("Delete My Theme").click();
    });

    await waitFor(() => expect(discarded).toEqual(["t-2"]));
  });
});
