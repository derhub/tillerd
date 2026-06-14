import { useMemo } from "react";
import { AppShell } from "~/components/AppShell";
import { DetachedWindow } from "~/components/DetachedWindow";
import { DesktopHostProvider } from "~/lib/useDesktopHost";
import { parseWindowIntent } from "~/lib/windows";

export default function Shell() {
  // Child windows carry their intent in the URL query (the custom scheme has no deep-route SPA
  // fallback, so every window loads at root and the shell dispatches on `?w=`).
  const intent = useMemo(
    () => parseWindowIntent(typeof window === "undefined" ? "" : window.location.search),
    [],
  );

  if (intent.kind === "detached") {
    return (
      <DesktopHostProvider>
        <DetachedWindow sessionId={intent.sessionId} placement={intent.placement} />
      </DesktopHostProvider>
    );
  }

  return (
    <DesktopHostProvider>
      <AppShell
        projectWindowId={intent.kind === "project" ? intent.projectId : undefined}
        initialSessionId={intent.kind === "project" ? intent.sessionId : undefined}
      />
    </DesktopHostProvider>
  );
}
