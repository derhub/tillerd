import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { TerminalPane } from "~/components/TerminalPane";
import { DesktopTerminalPane } from "~/components/DesktopTerminalPane";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { TauriAppData } from "~/lib/transport";

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
  const navigate = useNavigate();
  const isNew = routeId === "new" || routeId === null;
  const sessionId = isNew ? null : routeId;
  const [cwd, setCwd] = useState<string | null>(null);

  useEffect(() => {
    if (host.status !== "ready") return;
    if (isNew) {
      setCwd(host.host.info.homeDir);
      return;
    }
    let cancelled = false;
    void new TauriAppData(host.core).getCwd(sessionId!).then((c) => {
      if (!cancelled) setCwd(c ?? host.host.info.homeDir);
    });
    return () => {
      cancelled = true;
    };
  }, [host, isNew, sessionId]);

  if (host.status === "error") {
    return (
      <div className="p-6 text-[0.917rem] text-red-400">Desktop host failed: {host.error.message}</div>
    );
  }
  if (host.status !== "ready" || cwd === null) {
    return <div className="p-6 text-[0.917rem] text-muted-foreground/50">Starting…</div>;
  }

  return (
    <DesktopTerminalPane
      key={sessionId ?? "new"}
      sessionId={sessionId}
      cwd={cwd}
      onSessionStart={(newId) => {
        void new TauriAppData(host.core).recordSession(newId, cwd);
        void navigate(`/session/${newId}`, { replace: true });
      }}
    />
  );
}
