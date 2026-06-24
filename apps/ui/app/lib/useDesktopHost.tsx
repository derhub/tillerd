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

async function mountOrchestratorClient(
  onState: (s: DesktopHostState) => void,
  isCancelled: () => boolean,
  setUnlisten: (u: () => void) => void,
): Promise<void> {
  try {
    const client: SimpleOrchestratorClient = createDesktopOrchestratorClient();
    const unlisten = await client.subscribe((status) => {
      if (!isCancelled()) onState(toState(status));
    });
    setUnlisten(unlisten);
    if (isCancelled()) {
      unlisten();
      return;
    }
    const current = await client.status();
    if (!isCancelled()) onState(toState(current));
  } catch (e) {
    if (!isCancelled()) {
      onState({ status: "error", error: e instanceof Error ? e : new Error(String(e)) });
    }
  }
}

export function DesktopHostProvider({ children }: { children: React.ReactNode }) {
  const [state, setState] = React.useState<DesktopHostState>(() =>
    isDesktopHost() ? { status: "booting" } : { status: "web" },
  );

  React.useEffect(() => {
    if (!isDesktopHost()) {
      setReady(false);
      return;
    }
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void mountOrchestratorClient(
      setState,
      () => cancelled,
      (u) => {
        unlisten = u;
      },
    );
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return <DesktopHostContext.Provider value={state}>{children}</DesktopHostContext.Provider>;
}

export function useDesktopHost(): DesktopHostState {
  return React.use(DesktopHostContext);
}
