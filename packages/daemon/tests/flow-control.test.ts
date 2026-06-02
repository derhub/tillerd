import { test, expect, describe } from "bun:test";
import { PtySession } from "../src/pty-session";
import { PtyTransport } from "../src/pty-transport";
import { createLogger } from "@athing/logger";

// Minimal mock of PtyTransport for unit-testing flow control logic.
class MockTransport extends PtyTransport {
  paused = false;
  private dataHandlers: Set<(bytes: Uint8Array) => void> = new Set();

  constructor() {
    super({
      command: "",
      args: [],
      cwd: "/",
      env: {},
      logger: createLogger("mock"),
      shutdownGraceMs: 100,
    });
  }

  override spawn(): number {
    return 1;
  }
  override pause(): void {
    this.paused = true;
  }
  override resume(): void {
    this.paused = false;
  }
  override getMasterFd(): number {
    return 0;
  }
  override write(_bytes: Uint8Array): void {}
  override sendInterrupt(): void {}
  override resize(_cols: number, _rows: number): void {}
  override async kill() {
    return { code: null, signal: null };
  }

  override onData(handler: (bytes: Uint8Array) => void): () => void {
    this.dataHandlers.add(handler);
    return () => this.dataHandlers.delete(handler);
  }

  override onExit(
    _handler: (event: { code: number | null; signal: string | null }) => void,
  ): () => void {
    return () => {};
  }

  emit(bytes: Uint8Array): void {
    for (const h of this.dataHandlers) h(bytes);
  }
}

function makeSession(): { session: PtySession; transport: MockTransport } {
  const transport = new MockTransport();
  const session = PtySession.fromAdoptedTransport("test", transport, {
    replayBuffer: new Uint8Array(0),
    cwd: "/",
    cols: 80,
    rows: 24,
    pid: 1,
  });
  return { session, transport };
}

describe("flow control", () => {
  test("credit exhaustion pauses PTY fd, ack resumes it", () => {
    const { session, transport } = makeSession();
    const received: Uint8Array[] = [];

    // Start with exactly 10 bytes of credit.
    session.addSubscriber("key", (b) => received.push(b), 10);

    // Send 10 bytes — credit hits exactly 0, should pause.
    transport.emit(new Uint8Array(10));
    expect(transport.paused).toBe(true);
    expect(received).toHaveLength(1);

    // Ack restores credit — should resume.
    session.addCredit("key", 10);
    expect(transport.paused).toBe(false);
  });

  test("two subscribers independent: pausing one does not pause PTY while other has credit", () => {
    const { session, transport } = makeSession();
    const recvA: Uint8Array[] = [];
    const recvB: Uint8Array[] = [];

    session.addSubscriber("A", (b) => recvA.push(b), 5);
    session.addSubscriber("B", (b) => recvB.push(b), 100);

    // 6 bytes exhausts A's credit (5 bytes) but B still has 94 left.
    transport.emit(new Uint8Array(6));

    // PTY should NOT be paused because B still has credit.
    expect(transport.paused).toBe(false);
    expect(recvA).toHaveLength(1); // A received (was not yet paused when emitted)
    expect(recvB).toHaveLength(1); // B received

    // Send another chunk — A is paused now, only B should receive.
    transport.emit(new Uint8Array(1));
    expect(recvA).toHaveLength(1); // A skipped (paused)
    expect(recvB).toHaveLength(2); // B received
  });

  test("PTY paused only when ALL subscribers exhausted", () => {
    const { session, transport } = makeSession();

    session.addSubscriber("A", () => {}, 5);
    session.addSubscriber("B", () => {}, 5);

    // 6 bytes exhausts both A and B.
    transport.emit(new Uint8Array(6));

    // Now both are exhausted — PTY should be paused.
    expect(transport.paused).toBe(true);

    // Ack for A restores A — PTY should resume even though B is still paused.
    session.addCredit("A", 10);
    expect(transport.paused).toBe(false);
  });
});
