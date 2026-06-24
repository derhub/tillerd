import type { QueryKey } from "@tanstack/react-query";

declare module "@tanstack/react-query" {
  interface Register {
    mutationMeta: { invalidates?: QueryKey[] };
  }
}

let ready = false;

// Resets to a new pending promise on setReady(false) so re-boot cycles get a fresh signal.
let readyResolve: ((ok: boolean) => void) | null = null;
let readyPromise: Promise<boolean> = new Promise((resolve) => {
  readyResolve = resolve;
});

export function setReady(ok: boolean): void {
  ready = ok;
  if (readyResolve) {
    const resolve = readyResolve;
    readyResolve = null;
    resolve(ok);
  }
  if (!ok) {
    readyPromise = new Promise((resolve) => {
      readyResolve = resolve;
    });
  }
}

export function isReady(): boolean {
  return ready;
}

export function whenReady(): Promise<boolean> {
  return readyPromise;
}

/** Unwrap a typedError result, throwing on error status. */
export function ensureResult<T>(
  result: { status: "ok"; data: T } | { status: "error"; error: unknown },
): T {
  if (result.status === "error") throw new Error(String(result.error));
  return result.data;
}
