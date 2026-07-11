import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, waitFor } from "@testing-library/react";
import { setQueryClient } from "@tillerd/client-bindings";
/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";
import React from "react";

import { SettingsProvider } from "~/lib/settings/context";

import { resetContext } from "./context";
import { ACTION } from "./ids";
import { CommandRegistryProvider, RegisterHandlers } from "./registry";
import { usePaneShortcutDispatch } from "./usePaneShortcuts";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
    },
  },
});
setQueryClient(queryClient);

afterEach(() => {
  cleanup();
  resetContext();
  queryClient.clear();
});

interface DispatchRef {
  dispatch?: (e: KeyboardEvent) => boolean;
}

function TestComponent({
  handlers,
  dispatchRef,
}: {
  handlers: Record<string, () => void>;
  dispatchRef: DispatchRef;
}) {
  const dispatch = usePaneShortcutDispatch();
  dispatchRef.dispatch = dispatch;
  return <RegisterHandlers handlers={handlers} />;
}

describe("usePaneShortcutDispatch", () => {
  test("returns false for unbound shortcut", async () => {
    const dispatchRef: DispatchRef = {};
    render(
      <QueryClientProvider client={queryClient}>
        <SettingsProvider resolve={() => Promise.resolve([])}>
          <CommandRegistryProvider>
            <TestComponent handlers={{}} dispatchRef={dispatchRef} />
          </CommandRegistryProvider>
        </SettingsProvider>
      </QueryClientProvider>,
    );

    // Wait for settings hydration
    await waitFor(() => {
      expect(dispatchRef.dispatch).toBeDefined();
    });

    const event = new KeyboardEvent("keydown", { key: "x", metaKey: true });
    expect(dispatchRef.dispatch!(event)).toBe(false);
  });

  test("runs the command and returns true when bound shortcut is triggered", async () => {
    let ran = false;
    const dispatchRef: DispatchRef = {};
    render(
      <QueryClientProvider client={queryClient}>
        <SettingsProvider resolve={() => Promise.resolve([])}>
          <CommandRegistryProvider>
            <TestComponent
              handlers={{
                [ACTION.surfaceClose]: () => {
                  ran = true;
                },
              }}
              dispatchRef={dispatchRef}
            />
          </CommandRegistryProvider>
        </SettingsProvider>
      </QueryClientProvider>,
    );

    // Wait for settings hydration
    await waitFor(() => {
      expect(dispatchRef.dispatch).toBeDefined();
    });

    // Trigger CMD+W (default for surfaceClose)
    const event = new KeyboardEvent("keydown", { key: "w", metaKey: true });
    expect(dispatchRef.dispatch!(event)).toBe(true);
    expect(ran).toBe(true);
  });

  test("does not stop at first inactive matching shortcut if a later matching one is active", async () => {
    let ranSplitV = false;
    const dispatchRef: DispatchRef = {};
    render(
      <QueryClientProvider client={queryClient}>
        <SettingsProvider
          resolve={() =>
            Promise.resolve([
              {
                key: "keybindings.overrides",
                value: JSON.stringify({
                  [ACTION.panelSplitH]: "CmdOrCtrl+M",
                  [ACTION.panelSplitV]: "CmdOrCtrl+M",
                }),
              },
            ])
          }
        >
          <CommandRegistryProvider>
            <TestComponent
              handlers={{
                [ACTION.panelSplitV]: () => {
                  ranSplitV = true;
                },
              }}
              dispatchRef={dispatchRef}
            />
          </CommandRegistryProvider>
        </SettingsProvider>
      </QueryClientProvider>,
    );

    // Wait for settings hydration and check dispatch is bound
    await waitFor(() => {
      expect(dispatchRef.dispatch).toBeDefined();
    });

    const event = new KeyboardEvent("keydown", { key: "m", metaKey: true });
    expect(dispatchRef.dispatch!(event)).toBe(true);
    expect(ranSplitV).toBe(true);
  });
});
