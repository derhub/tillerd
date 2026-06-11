export function DesktopTerminalPane(_props: {
  sessionId: string | null;
  cwd: string;
  onSessionStart?: (id: string) => void;
}) {
  return <div className="h-full w-full" style={{ background: "#0d1117" }} />;
}
