// Tauri event bus over BroadcastChannel: Tauri windows are separate OS webview processes
// (WKWebView / WebView2 / WebKitGTK), so BroadcastChannel does not reliably broker across them.
//
// Coalescing guard: a burst of mutations (e.g. drag-reorder) must not become a storm of emits +
// refetches. Keys accumulate in a Map keyed by serialized form (dedupe) and flush once per trailing
// window. N rapid mutations cost at most one emit per flush; each receiver runs at most one
// invalidation pass per flush. TanStack dedupes concurrent refetches of the same query in-flight.

import type { QueryClient, QueryKey } from "@tanstack/react-query";

import { emitEvent, listenEvent, windowLabel } from "./tauriEvents";
import { isDesktopHost } from "./transport/core";

const INVALIDATE_EVENT = "query:invalidate";

// Long enough to absorb a synchronous mutation burst; short enough to stay perceptually live.
const FLUSH_MS = 80;

// QueryKey is a serializable array (strings + ids), so it crosses postMessage as-is.
type InvalidatePayload = { source: string; keys: QueryKey[] };

type Coalescer = { add: (keys: QueryKey[]) => void; cancel: () => void };

// Both send and receive sides share this shape; they differ only in what `flush` does.
function makeCoalescer(flush: (keys: QueryKey[]) => void): Coalescer {
  const pending = new Map<string, QueryKey>();
  let timer: ReturnType<typeof setTimeout> | null = null;
  return {
    add(keys) {
      for (const key of keys) pending.set(JSON.stringify(key), key);
      if (timer === null) {
        timer = setTimeout(() => {
          timer = null;
          const batch = [...pending.values()];
          pending.clear();
          flush(batch);
        }, FLUSH_MS);
      }
    },
    cancel() {
      if (timer !== null) clearTimeout(timer);
    },
  };
}

async function emitInvalidate(keys: QueryKey[]): Promise<void> {
  if (keys.length === 0) return;
  const source = (await windowLabel()) ?? "";
  await emitEvent(INVALIDATE_EVENT, { source, keys } satisfies InvalidatePayload);
}

const outbound = makeCoalescer((keys) => void emitInvalidate(keys));

// Fire-and-forget: if the emit fails, each window self-heals on its next read.
export function broadcastInvalidate(keys: QueryKey[]): void {
  if (!isDesktopHost() || keys.length === 0) return;
  outbound.add(keys);
}

// Side channel for stores that mirror server state outside the Query cache (the
// settings store): they re-read their source only when a SIBLING window's write
// lands -- never on this window's own writes, so a local optimistic value can
// never be clobbered by its own feedback.
type RemoteInvalidateListener = (keys: QueryKey[]) => void;
const remoteListeners = new Set<RemoteInvalidateListener>();

export function onRemoteInvalidate(listener: RemoteInvalidateListener): () => void {
  remoteListeners.add(listener);
  return () => remoteListeners.delete(listener);
}

// Mount once per window. The sender receives its own global emit; skip events tagged with this
// window's label (already invalidated locally). Listeners never re-emit, so there is no echo loop.
export async function mountCrossWindowInvalidate(client: QueryClient): Promise<() => void> {
  if (!isDesktopHost()) return () => {};
  const self = await windowLabel();
  const inbound = makeCoalescer((keys) => {
    keys.forEach((queryKey) => void client.invalidateQueries({ queryKey }));
    remoteListeners.forEach((listener) => listener(keys));
  });

  const unlisten = await listenEvent<InvalidatePayload>(INVALIDATE_EVENT, (payload) => {
    if (payload.source === self) return;
    inbound.add(payload.keys);
  });

  return () => {
    inbound.cancel();
    unlisten();
  };
}
