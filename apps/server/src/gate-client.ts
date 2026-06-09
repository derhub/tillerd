import type { HookEvent } from "@tillerd/sdk";
import {
  SubscriptionFrameDecoder,
  encodeSubscribePreamble,
  decodeSubscriptionFrame,
  negotiateReady,
  RawFrame,
} from "@tillerd/sdk";

export interface GateSubscribeOptions {
  socketPath: string;
  sessionId: string;
}

export class GateNegotiationError extends Error {
  constructor(
    message: string,
    readonly detail?: unknown,
  ) {
    super(message);
    this.name = "GateNegotiationError";
  }
}

export async function subscribeToSession(
  opts: GateSubscribeOptions,
): Promise<AsyncIterableIterator<HookEvent>> {
  const { socketPath, sessionId } = opts;

  let resolveReady: () => void;
  let rejectReady: (err: unknown) => void;
  const readyP = new Promise<void>((res, rej) => {
    resolveReady = res;
    rejectReady = rej;
  });

  const decoder = new SubscriptionFrameDecoder();
  const queue: HookEvent[] = [];
  let waiters: Array<(value: IteratorResult<HookEvent>) => void> = [];
  let done = false;
  let negotiated = false;

  function enqueue(event: HookEvent) {
    if (waiters.length > 0) {
      const resolve = waiters.shift()!;
      resolve({ value: event, done: false });
    } else {
      queue.push(event);
    }
  }

  function close() {
    done = true;
    for (const resolve of waiters) {
      resolve({ value: undefined as unknown as HookEvent, done: true });
    }
    waiters = [];
  }

  function handleRawFrames(frames: RawFrame[]) {
    for (const rawFrame of frames) {
      const frame = decodeSubscriptionFrame(rawFrame);
      if (!frame) continue;

      if (!negotiated) {
        const result = negotiateReady(frame);
        if (!result.ok) {
          rejectReady(new GateNegotiationError("gate negotiation failed", result.error));
          return;
        }
        negotiated = true;
        resolveReady();
        continue;
      }

      if (frame.type === "Event") {
        enqueue(frame.event);
      } else if (frame.type === "Error") {
        close();
      }
    }
  }

  const connectPromise = new Promise<void>((resolveConn, rejectConn) => {
    Bun.connect({
      unix: socketPath,
      socket: {
        open(socket) {
          socket.write(encodeSubscribePreamble(sessionId));
          resolveConn();
        },
        data(_socket, data: Buffer) {
          const chunk = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
          const frames = decoder.push(chunk);
          handleRawFrames(frames);
        },
        close() {
          close();
        },
        error(_socket, err) {
          if (!negotiated) {
            rejectReady(err);
          }
          close();
        },
        connectError(_socket, err) {
          rejectConn(err);
        },
      },
    });
  });

  await connectPromise;
  await readyP;

  const iter: AsyncIterableIterator<HookEvent> = {
    [Symbol.asyncIterator](): AsyncIterableIterator<HookEvent> {
      return iter;
    },
    async next(): Promise<IteratorResult<HookEvent>> {
      if (queue.length > 0) {
        return { value: queue.shift()!, done: false };
      }
      if (done) {
        return { value: undefined as unknown as HookEvent, done: true };
      }
      return new Promise<IteratorResult<HookEvent>>((resolve) => {
        waiters.push(resolve);
      });
    },
    async return(): Promise<IteratorResult<HookEvent>> {
      close();
      return { value: undefined as unknown as HookEvent, done: true };
    },
  };
  return iter;
}
