// Bridge an async event-listener registration to a synchronous effect cleanup. Components subscribe
// inside useEffect without async/await/.then (the promise is awaited here, a plain helper): pass the
// listen() promise, get back a cleanup that detaches once the handle resolves -- and detaches eagerly
// if the effect tears down before it does.
export function subscribe(listening: Promise<() => void>): () => void {
  let unlisten: (() => void) | undefined;
  let cancelled = false;
  void listening.then((off) => {
    if (cancelled) off();
    else unlisten = off;
  });
  return () => {
    cancelled = true;
    unlisten?.();
  };
}

// Fire an async action from a synchronous handler without surfacing the promise in the component.
// `run(doThing())` replaces `await doThing()` / `void doThing().catch(...)` at a call site that must
// stay non-async; errors are swallowed (callers that need the result use a queryFn/mutationFn).
export function run(action: Promise<unknown>): void {
  void action.then(undefined, () => {});
}
