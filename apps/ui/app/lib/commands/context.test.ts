import { afterEach, describe, expect, test } from "bun:test";

import { contextStore, readContext, resetContext, setContextKey } from "./context";

afterEach(resetContext);

describe("context store", () => {
  test("setContextKey stores a value readable via readContext", () => {
    setContextKey("terminalFocus", true);
    expect(readContext().terminalFocus).toBe(true);
  });

  test("setContextKey overwrites a prior value", () => {
    setContextKey("mode", "edit");
    setContextKey("mode", "view");
    expect(readContext().mode).toBe("view");
  });

  test("setting undefined clears a key", () => {
    setContextKey("hasSession", true);
    setContextKey("hasSession", undefined);
    expect(readContext().hasSession).toBeUndefined();
  });

  test("resetContext clears all keys", () => {
    setContextKey("a", true);
    setContextKey("b", "x");
    resetContext();
    expect(readContext()).toEqual({});
  });

  test("the store updates its state reference on change", () => {
    const before = contextStore.state;
    setContextKey("k", true);
    expect(contextStore.state).not.toBe(before);
    expect(contextStore.state.k).toBe(true);
  });
});
