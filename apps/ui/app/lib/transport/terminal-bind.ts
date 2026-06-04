import type { AgentSession } from "@athing/sdk";

/** The slice of `@xterm/xterm`'s `Terminal` the binder touches. */
export interface TerminalLike {
  write(data: Uint8Array | string): void;
  onData(cb: (data: string) => void): { dispose(): void };
  readonly cols: number;
  readonly rows: number;
}

/**
 * Drive an xterm terminal from an engine `AgentSession`: raw daemon bytes -> `term.write`,
 * keystrokes -> `session.input`, with an initial resize. Returns a cleanup that detaches both.
 */
export function bindSessionToTerminal(session: AgentSession, term: TerminalLike): () => void {
  const encoder = new TextEncoder();
  const offData = session.onData((bytes) => term.write(bytes));
  const sub = term.onData((data) => session.input(encoder.encode(data)));
  if (term.cols && term.rows) session.resize(term.cols, term.rows);
  return () => {
    offData();
    sub.dispose();
  };
}
