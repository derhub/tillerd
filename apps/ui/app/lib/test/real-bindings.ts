// Real client-bindings wrappers, snapshotted as plain values the first time this module evaluates.
// The bunfig test preload imports this before any test file runs, so the snapshot is taken before a
// sibling suite registers a process-global mock.module("@tillerd/client-bindings"). bun never resets
// module mocks between files and mock.restore does not undo mock.module, so a re-export here would go
// live-mocked; capturing the function value freezes the real implementation. A suite that stubs
// query() for its own keys delegates every other key to realQuery, keeping sibling suites that rely
// on the real query()/whenReady() path working regardless of filesystem order (macOS vs Linux CI).

import { query, setReady as setReadySource } from "@tillerd/client-bindings";

export const realQuery = query;
export const setReady = setReadySource;

// Build a query() stub that intercepts the given keys and delegates every other key to the real
// query(), preserving query.infinite. Lets a suite stub its own keys without clobbering the real
// query()/whenReady() path for sibling suites under a process-global mock.module.
type QueryStub = (key: string, args?: unknown) => unknown;
export function delegatingQuery(overrides: Record<string, (args?: unknown) => unknown>): QueryStub {
  const fn: QueryStub = (key, args) => {
    const override = overrides[key];
    return override ? override(args) : (realQuery as never as QueryStub)(key, args);
  };
  return Object.assign(fn, { infinite: (realQuery as { infinite: unknown }).infinite });
}
