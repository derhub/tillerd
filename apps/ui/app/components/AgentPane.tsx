import { useEffect, useRef, useState, useCallback } from "react";
import { DesktopTerminalPane } from "./DesktopTerminalPane";

type AgentStatus = "IDLE" | "WORKING" | "WAITING_INPUT" | "DONE" | "crashed";

type ContentEntry = {
  toolName: string;
  toolInput: unknown;
};

type ContentEvent = {
  kind: string;
  toolName: string;
  toolInput: unknown;
};

const MAX_CONTENT_ENTRIES = 500;

const STATUS_STYLE: Record<AgentStatus, { dot: string; label: string }> = {
  IDLE: { dot: "#8b949e", label: "idle" },
  WORKING: { dot: "#d29922", label: "working" },
  WAITING_INPUT: { dot: "#58a6ff", label: "waiting" },
  DONE: { dot: "#3fb950", label: "done" },
  crashed: { dot: "#ff7b72", label: "crashed" },
};

type Props = {
  surfaceId: string;
  cwd: string;
};

export function AgentPane({ surfaceId, cwd }: Props) {
  const [agentStatus, setAgentStatus] = useState<AgentStatus>("IDLE");
  const [entries, setEntries] = useState<ContentEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const contentEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      if (cancelled) return;

      const unsubStatus = await listen<{ surfaceId: string; status: string }>(
        "surface://status",
        (e) => {
          if (e.payload.surfaceId !== surfaceId || cancelled) return;
          const s = e.payload.status as AgentStatus;
          setAgentStatus(s);
        },
      );
      unlisteners.push(unsubStatus);

      const unsubExit = await listen<{ surfaceId: string; qualifier: string }>(
        "surface://exit",
        (e) => {
          if (e.payload.surfaceId !== surfaceId || cancelled) return;
          if (e.payload.qualifier !== "ok" && e.payload.qualifier !== "stopped-by-request") {
            setAgentStatus("crashed");
          }
        },
      );
      unlisteners.push(unsubExit);

      const unsubContent = await listen<{ surfaceId: string; event: ContentEvent }>(
        "surface:content",
        (e) => {
          if (e.payload.surfaceId !== surfaceId || cancelled) return;
          const ev = e.payload.event;
          if (ev.kind !== "tool_use") return;
          setEntries((prev) => {
            const next = [...prev, { toolName: ev.toolName, toolInput: ev.toolInput }];
            return next.length > MAX_CONTENT_ENTRIES ? next.slice(-MAX_CONTENT_ENTRIES) : next;
          });
        },
      );
      unlisteners.push(unsubContent);

      const unsubError = await listen<{ surfaceId: string; reason: string }>(
        "surface:error",
        (e) => {
          if (e.payload.surfaceId !== surfaceId || cancelled) return;
          setError(e.payload.reason);
        },
      );
      unlisteners.push(unsubError);
    })();

    return () => {
      cancelled = true;
      for (const u of unlisteners) u();
    };
  }, [surfaceId]);

  useEffect(() => {
    contentEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [entries]);

  const dismissError = useCallback(() => setError(null), []);

  const style = STATUS_STYLE[agentStatus] ?? STATUS_STYLE.IDLE;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", width: "100%" }}>
      <div style={{ flexGrow: 4, position: "relative", minHeight: 0 }}>
        <DesktopTerminalPane sessionId={surfaceId} cwd={cwd} />
        <div
          style={{
            position: "absolute",
            top: "0.5rem",
            right: "0.75rem",
            display: "flex",
            alignItems: "center",
            gap: "0.375rem",
            pointerEvents: "none",
            zIndex: 10,
          }}
        >
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: style.dot,
              display: "inline-block",
            }}
          />
          <span style={{ color: "#8b949e", fontSize: 11, fontFamily: "monospace" }}>
            {style.label}
          </span>
        </div>
      </div>

      <div
        style={{
          flexGrow: 1,
          minHeight: 0,
          display: "flex",
          flexDirection: "column",
          background: "#0d1117",
          borderTop: "1px solid #21262d",
          overflow: "hidden",
        }}
      >
        {error !== null && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "0.5rem",
              padding: "0.375rem 0.75rem",
              background: "#21262d",
              borderBottom: "1px solid #30363d",
              flexShrink: 0,
            }}
          >
            <span style={{ color: "#ff7b72", fontSize: 12 }}>{error}</span>
            <button
              onClick={dismissError}
              style={{
                marginLeft: "auto",
                background: "transparent",
                border: "1px solid #30363d",
                borderRadius: 4,
                color: "#8b949e",
                padding: "0.125rem 0.5rem",
                fontSize: 11,
                cursor: "pointer",
              }}
            >
              Dismiss
            </button>
          </div>
        )}
        <div style={{ flex: 1, overflowY: "auto", padding: "0.375rem 0.5rem" }}>
          {entries.map((entry, i) => (
            <div
              key={i}
              style={{
                padding: "0.25rem 0.375rem",
                marginBottom: "0.25rem",
                background: "#161b22",
                borderRadius: 4,
                fontFamily: "monospace",
                fontSize: 11,
                color: "#e6edf3",
              }}
            >
              <span style={{ color: "#58a6ff" }}>{entry.toolName}</span>
              <span style={{ color: "#8b949e", marginLeft: "0.5rem" }}>
                {JSON.stringify(entry.toolInput)}
              </span>
            </div>
          ))}
          <div ref={contentEndRef} />
        </div>
      </div>
    </div>
  );
}
