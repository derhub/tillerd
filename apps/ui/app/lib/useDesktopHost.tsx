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

const DesktopHostContext = React.createContext<DesktopHostState>({ status: "web" });

function toState(status: StatusWire): DesktopHostState {
  if (status.state === "ready") {
    setReady(true);
    return { status: "ready" };
  }
  setReady(false);
  if (status.state === "failed") return { status: "error", error: new Error(status.reason) };
  return { status: "booting" };
}

// Global state to eagerly track the desktop host status and avoid deadlocks in router loaders
let globalState: DesktopHostState = isDesktopHost() ? { status: "booting" } : { status: "web" };
const listeners = new Set<(s: DesktopHostState) => void>();

function setGlobalState(newState: DesktopHostState) {
  globalState = newState;
  for (const listener of listeners) {
    listener(newState);
  }
}

if (isDesktopHost()) {
  void (async () => {
    try {
      const client: SimpleOrchestratorClient = createDesktopOrchestratorClient();
      const _unlisten = await client.subscribe((status) => {
        setGlobalState(toState(status));
      });
      const current = await client.status();
      setGlobalState(toState(current));
    } catch (e) {
      setGlobalState({ status: "error", error: e instanceof Error ? e : new Error(String(e)) });
    }
  })();
} else {
  setReady(false);
}

export function DesktopHostProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = React.useState<DesktopHostState>(globalState);

  React.useEffect(() => {
    setState(globalState);
    listeners.add(setState);
    return () => {
      listeners.delete(setState);
    };
  }, []);

  return <DesktopHostContext.Provider value={state}>{children}</DesktopHostContext.Provider>;
}

export function useDesktopHost(): DesktopHostState {
  return React.use(DesktopHostContext);
}
