import type { AgentSession } from "@tillerd/sdk";

export interface TerminalLike {
  write(data: Uint8Array | string): void;
  onData(cb: (data: string) => void): { dispose(): void };
  readonly cols: number;
  readonly rows: number;
}

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
