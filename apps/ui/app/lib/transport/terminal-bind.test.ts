import { test, expect, describe } from "bun:test";
import { bindSessionToTerminal, type TerminalLike } from "./terminal-bind";
import type { AgentSession } from "@tillerd/sdk";

function fakeSession() {
  let dataCb: ((b: Uint8Array) => void) | null = null;
  const inputs: Uint8Array[] = [];
  const resizes: Array<[number, number]> = [];
  let dataOff = 0;
  const session = {
    sessionId: "s",
    send() {},
    input: (b: Uint8Array) => inputs.push(b),
    interrupt() {},
    resize: (c: number, r: number) => resizes.push([c, r]),
    async kill() {
      return {} as never;
    },
    async stop() {
      return {} as never;
    },
    onData: (h: (b: Uint8Array) => void) => {
      dataCb = h;
      return () => {
        dataOff++;
      };
    },
    onStatus: () => () => {},
    onContent: () => () => {},
    onError: () => () => {},
    onExit: () => () => {},
  } as unknown as AgentSession;
  return {
    session,
    emit: (b: Uint8Array) => dataCb?.(b),
    inputs,
    resizes,
    dataOff: () => dataOff,
  };
}

function fakeTerm() {
  let inputCb: ((d: string) => void) | null = null;
  const written: Array<Uint8Array | string> = [];
  let disposed = 0;
  const term: TerminalLike = {
    write: (d) => written.push(d),
    onData: (cb) => {
      inputCb = cb;
      return { dispose: () => disposed++ };
    },
    cols: 80,
    rows: 24,
  };
  return { term, type: (d: string) => inputCb?.(d), written, disposed: () => disposed };
}

describe("bindSessionToTerminal", () => {
  test("pipes session output to the terminal and keystrokes to the session, with initial resize", () => {
    const s = fakeSession();
    const t = fakeTerm();
    bindSessionToTerminal(s.session, t.term);

    expect(s.resizes).toEqual([[80, 24]]);

    s.emit(new Uint8Array([65, 66]));
    expect(t.written).toHaveLength(1);
    expect([...(t.written[0] as Uint8Array)]).toEqual([65, 66]);

    t.type("x");
    expect([...s.inputs[0]!]).toEqual([120]);
  });

  test("cleanup detaches both directions", () => {
    const s = fakeSession();
    const t = fakeTerm();
    const cleanup = bindSessionToTerminal(s.session, t.term);
    cleanup();
    expect(s.dataOff()).toBe(1);
    expect(t.disposed()).toBe(1);
  });
});
