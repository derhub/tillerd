// Clean-exit inline bar (ui-terminal-pane spec): terminal-token styled, the clean-exit
// counterpart to TerminalFailureOverlay. Renders as a bottom-anchored strip rather than a
// full-screen overlay so the preserved scrollback stays visible above it.
export function TerminalExitBar({
  qualifier,
  onRestart,
  onNewSurface,
}: {
  // The runtime's clean-exit qualifier ("ok" | "stopped-by-request"); the pane only mounts this
  // bar on a clean exit, so the dot is never the error color here.
  qualifier: string;
  onRestart: () => void;
  onNewSurface: () => void;
}) {
  const stoppedByRequest = qualifier === "stopped-by-request";
  const dotColorClass = stoppedByRequest ? "bg-terminal-muted" : "bg-terminal-success";
  const label = stoppedByRequest ? "Process stopped" : "Process exited";

  return (
    <div
      data-testid="terminal-exit-bar"
      className="absolute inset-x-0 bottom-0 flex items-center gap-2 border-t border-terminal-border bg-terminal-surface px-4 py-2 text-[0.917rem] text-terminal-fg"
    >
      <span className={`w-2 h-2 rounded-full inline-block ${dotColorClass}`} />
      <span className="text-terminal-muted">{label}</span>
      <div className="ml-auto flex items-center gap-2">
        <button
          type="button"
          onClick={onRestart}
          data-testid="terminal-exit-restart"
          className="rounded-sm bg-terminal-success px-3 py-1 text-terminal-fg transition-colors duration-[var(--motion-fast)] ease-standard hover:brightness-110 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          Restart
        </button>
        <button
          type="button"
          onClick={onNewSurface}
          data-testid="terminal-exit-new"
          className="rounded-sm border border-terminal-border px-3 py-1 text-terminal-muted transition-colors duration-[var(--motion-fast)] ease-standard hover:text-terminal-fg focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          New surface
        </button>
      </div>
    </div>
  );
}
