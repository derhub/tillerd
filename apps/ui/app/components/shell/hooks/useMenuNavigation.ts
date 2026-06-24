import { useNavigate } from "@tanstack/react-router";
import React from "react";

import { subscribe } from "~/lib/subscribe";
import { isDesktopHost } from "~/lib/transport";
import { loadTauriCore } from "~/lib/transport/core";

function listenMenuNavigate(handler: (payload: string) => void): Promise<() => void> {
  return loadTauriCore().then((core) => core.listen<string>("menu:navigate", handler));
}

export function useMenuNavigation(): void {
  const navigate = useNavigate();
  React.useEffect(() => {
    if (!isDesktopHost()) return;
    return subscribe(
      listenMenuNavigate((payload) => {
        const to = payload === "/logs" ? "/logs" : "/";
        void navigate({ to } as never);
      }),
    );
  }, [navigate]);
}
