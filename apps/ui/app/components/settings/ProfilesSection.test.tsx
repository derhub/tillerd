import { QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { setQueryClient, setReady } from "@tillerd/client-bindings";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

import { TooltipProvider } from "~/components/ui/tooltip";
import { makeQueryClient } from "~/lib/queryClient";

// Profile management: activating/renaming/duplicating/deleting a profile, and the spec's guard
// on deleting the ACTIVE profile (a confirmation, not a block -- unlike the themes prebuilt guard).

let profiles: { id: string; name: string }[] = [
  { id: "p-1", name: "Default" },
  { id: "p-2", name: "Work" },
];
let activeId: string | null = "p-1";
const discarded: string[] = [];

import { beforeEach } from "bun:test";

beforeEach(() => {
  (globalThis as any).__tillerd_active_invoke = async (
    cmd: string,
    args?: Record<string, unknown>,
  ) => {
    if (cmd === "profile_list") return profiles;
    if (cmd === "profile_get_active") return profiles.find((p) => p.id === activeId) ?? null;
    if (cmd === "profile_discard") {
      discarded.push(args?.["id"] as string);
      profiles = profiles.filter((p) => p.id !== args?.["id"]);
      return null;
    }
    // Deleting the active profile re-hydrates the settings store (see ProfilesSection); its
    // fetchQuery hits settingList unconditionally.
    if (cmd === "setting_list") return [];
    return undefined;
  };
});

const { ProfilesSection } = await import("./ProfilesSection");

afterEach(() => {
  cleanup();
  mock.restore();
  delete (globalThis as any).__tillerd_active_invoke;
  profiles = [
    { id: "p-1", name: "Default" },
    { id: "p-2", name: "Work" },
  ];
  activeId = "p-1";
  discarded.length = 0;
  setReady(false);
});

function renderSection() {
  const client = makeQueryClient();
  setQueryClient(client);
  setReady(true);
  return render(
    <QueryClientProvider client={client}>
      <TooltipProvider>
        <ProfilesSection />
      </TooltipProvider>
    </QueryClientProvider>,
  );
}

describe("ProfilesSection", () => {
  test("lists profiles with the active one badged", async () => {
    renderSection();
    await waitFor(() => expect(screen.queryByText("Default")).not.toBeNull());
    expect(screen.queryByText("Work")).not.toBeNull();
    expect(screen.getByTestId("profile-active-badge")).not.toBeNull();
  });

  test("deleting the active profile shows the active-profile warning before it discards", async () => {
    renderSection();
    await waitFor(() => expect(screen.queryByText("Default")).not.toBeNull());

    act(() => {
      screen.getByLabelText("Delete Default").click();
    });

    // Guarded by a confirmation naming the consequence -- not silently deleted.
    expect(await screen.findByText(/is active/i)).not.toBeNull();
    expect(discarded).toHaveLength(0);

    act(() => {
      screen.getByRole("button", { name: "Delete" }).click();
    });
    await waitFor(() => expect(discarded).toEqual(["p-1"]));
  });

  test("deleting a non-active profile shows the plain confirmation", async () => {
    renderSection();
    await waitFor(() => expect(screen.queryByText("Work")).not.toBeNull());

    act(() => {
      screen.getByLabelText("Delete Work").click();
    });

    expect(await screen.findByText(/permanently delete the profile/i)).not.toBeNull();
    expect(screen.queryByText(/is active/i)).toBeNull();
  });
});
