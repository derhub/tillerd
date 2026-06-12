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

  const sessionId = routeId === "new" || routeId === null ? null : routeId;
  // Key by session so switching sessions remounts the pane and creates that session's own surface —
  // without it the pane instance is reused and its mount-time `useEffect` never recreates the
  // surface, leaving every session showing the first one's terminal.
  return <DesktopTerminalPane key={sessionId ?? "new"} sessionId={sessionId} cwd="" />;
}
