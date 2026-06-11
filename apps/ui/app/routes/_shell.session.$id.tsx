import { useParams } from "react-router";
import { TerminalPane } from "~/components/TerminalPane";
import { DesktopTerminalPane } from "~/components/DesktopTerminalPane";
import { useDesktopHost } from "~/lib/useDesktopHost";

export default function SessionPage() {
  const { id } = useParams();
  const host = useDesktopHost();

  if (host.status === "web") {
    return <TerminalPane sessionId={id ?? null} />;
  }
  return <DesktopSession routeId={id ?? null} />;
}

function DesktopSession({ routeId }: { routeId: string | null }) {
  const host = useDesktopHost();

  if (host.status === "error") {
    return (
      <div className="p-6 text-[0.917rem] text-red-400">Backend failed: {host.error.message}</div>
    );
  }
  if (host.status !== "ready") {
    return <div className="p-6 text-[0.917rem] text-muted-foreground/50">Starting…</div>;
  }

  // The orchestrator is ready; the surface itself is driven by the Rust surface
  // runtime in a later slice. A blank pane is acceptable here.
  const sessionId = routeId === "new" || routeId === null ? null : routeId;
  return <DesktopTerminalPane sessionId={sessionId} cwd="" />;
}
