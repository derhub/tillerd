import { Store, useSelector } from "@tanstack/react-store";
import { setReady } from "@tillerd/client-bindings";
import React from "react";

import { isDesktopHost } from "~/lib/transport";
import {
  createDesktopOrchestratorClient,
  type SimpleOrchestratorClient,
  type StatusWire,
} from "~/lib/transport/orchestrator";

export type DesktopHostState =
  | { status: "web" }
  | { status: "booting" }
  | { status: "ready" }
  | { status: "error"; error: Error };

function toState(status: StatusWire): DesktopHostState {
  if (status.state === "ready") {
    setReady(true);
    return { status: "ready" };
  }
  setReady(false);
  if (status.state === "failed") return { status: "error", error: new Error(status.reason) };
  return { status: "booting" };
}

// Module-level store so the boot status is tracked eagerly (before any component mounts) and
// router loaders never deadlock waiting for a provider. TanStack Store is the one client-state
// mechanism (client-engine spec); components subscribe via useSelector.
export const desktopHostStore = new Store<DesktopHostState>(
  isDesktopHost() ? { status: "booting" } : { status: "web" },
);

if (isDesktopHost()) {
  void (async () => {
    try {
      const client: SimpleOrchestratorClient = createDesktopOrchestratorClient();
      await client.subscribe((status) => {
        desktopHostStore.setState(() => toState(status));
      });
      const current = await client.status();
      desktopHostStore.setState(() => toState(current));
    } catch (e) {
      desktopHostStore.setState(() => ({
        status: "error",
        error: e instanceof Error ? e : new Error(String(e)),
      }));
    }
  })();
} else {
  setReady(false);
}

export function DesktopHostProvider({ children }: { children: React.ReactNode }) {
  return children;
}

export function useDesktopHost(): DesktopHostState {
  return useSelector(desktopHostStore, (s) => s);
}
