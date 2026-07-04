import { cleanup, render } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

import type { CommandDef, CommandHandler } from "./types";

import { ACTION } from "./ids";
import { CommandRegistryProvider, composeCommands, useCommand, useCommands } from "./registry";

afterEach(cleanup);

function Register({ id, handler }: { id: string; handler: CommandHandler }) {
  useCommand(id, handler);
  return null;
}

describe("composeCommands", () => {
  const handler = () => {};

  test("wires a registered handler onto its definition", () => {
    const defs: CommandDef[] = [{ id: "a", title: "A" }];
    const [cmd] = composeCommands(defs, new Map([["a", handler]]), {});
    expect(cmd.run).toBe(handler);
  });

  test("excludes a definition with no registered handler", () => {
    const defs: CommandDef[] = [{ id: "a", title: "A" }];
    expect(composeCommands(defs, new Map(), {})).toEqual([]);
  });

  test("a toggle definition resolves checked from context", () => {
    const defs: CommandDef[] = [{ id: "t", title: "T", toggle: (ctx) => Boolean(ctx.on) }];
    const handlers = new Map([["t", handler]]);
    expect(composeCommands(defs, handlers, { on: true })[0].checked).toBe(true);
    expect(composeCommands(defs, handlers, { on: false })[0].checked).toBe(false);
  });
});

describe("command registry", () => {
  test("a registered handler is invoked by its command", () => {
    const spy = mock(() => {});
    let run: (() => void) | undefined;
    function Runner() {
      run = useCommands().find((c) => c.id === ACTION.viewLogs)?.run;
      return null;
    }
    render(
      <CommandRegistryProvider>
        <Register id={ACTION.viewLogs} handler={spy} />
        <Runner />
      </CommandRegistryProvider>,
    );
    run?.();
    expect(spy).toHaveBeenCalledTimes(1);
  });

  test("invoking with an argument payload passes it to the handler", () => {
    const spy = mock((_args?: { entityId?: string; entityKind?: string }) => {});
    let run: CommandHandler | undefined;
    function Runner() {
      run = useCommands().find((c) => c.id === ACTION.viewLogs)?.run;
      return null;
    }
    render(
      <CommandRegistryProvider>
        <Register id={ACTION.viewLogs} handler={spy} />
        <Runner />
      </CommandRegistryProvider>,
    );
    run?.({ entityId: "session-1", entityKind: "session" });
    expect(spy).toHaveBeenCalledWith({ entityId: "session-1", entityKind: "session" });
  });

  test("invoking without arguments still calls the handler with no payload", () => {
    const spy = mock((_args?: { entityId?: string }) => {});
    let run: CommandHandler | undefined;
    function Runner() {
      run = useCommands().find((c) => c.id === ACTION.viewLogs)?.run;
      return null;
    }
    render(
      <CommandRegistryProvider>
        <Register id={ACTION.viewLogs} handler={spy} />
        <Runner />
      </CommandRegistryProvider>,
    );
    run?.();
    expect(spy).toHaveBeenCalledWith();
  });

  test("a command with no registered handler is absent from the registry", () => {
    let found: unknown;
    function Runner() {
      found = useCommands().find((c) => c.id === ACTION.projectNew);
      return null;
    }
    render(
      <CommandRegistryProvider>
        <Runner />
      </CommandRegistryProvider>,
    );
    expect(found).toBeUndefined();
  });
});
