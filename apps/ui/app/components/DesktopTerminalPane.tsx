/**
 * Desktop terminal surface. In this slice the renderer drives no agent engine
 * (ADR-0022): the terminal surface is driven by the Rust surface runtime through
 * the orchestrator API in a later slice (0.0.2). Until then a blank pane is the
 * accepted bar once the orchestrator reaches `ready`.
 */
export function DesktopTerminalPane(_props: {
  sessionId: string | null;
  cwd: string;
  onSessionStart?: (id: string) => void;
}) {
  return <div className="h-full w-full" style={{ background: "#0d1117" }} />;
}
