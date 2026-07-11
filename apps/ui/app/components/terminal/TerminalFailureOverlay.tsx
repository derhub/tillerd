// Surface failure overlay (ui-terminal-pane spec): terminal-token styled, distinct from the
// service-health indicator. Shared by the desktop channel pane and the web-host fallback pane.
export function TerminalFailureOverlay({
  reason,
  onResume,
  onDismiss,
}: {
  reason: string;
  onResume: () => void;
  onDismiss: () => void;
}) {
  return (
    <div
      data-testid="terminal-failure-overlay"
      className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-2 rounded-md border border-terminal-border bg-terminal-surface px-4 py-3 text-[0.917rem] text-terminal-fg"
    >
      <span className="text-terminal-error">{reason}</span>
      <button
        type="button"
        onClick={onResume}
        className="rounded-sm bg-terminal-success px-3 py-1 text-terminal-fg transition-colors duration-[var(--motion-fast)] ease-standard hover:brightness-110"
      >
        Resume
      </button>
      <button
        type="button"
        onClick={onDismiss}
        className="rounded-sm border border-terminal-border px-3 py-1 text-terminal-muted transition-colors duration-[var(--motion-fast)] ease-standard hover:text-terminal-fg"
      >
        Dismiss
      </button>
    </div>
  );
}
