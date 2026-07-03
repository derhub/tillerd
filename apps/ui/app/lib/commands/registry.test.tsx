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

  test("an unregistered definition gets an inert no-op handler", () => {
    const defs: CommandDef[] = [{ id: "a", title: "A" }];
    const [cmd] = composeCommands(defs, new Map(), {});
    expect(() => cmd.run()).not.toThrow();
    expect(cmd.checked).toBeUndefined();
  });

  test("a toggle definition resolves checked from context", () => {
    const defs: CommandDef[] = [{ id: "t", title: "T", toggle: (ctx) => Boolean(ctx.on) }];
    expect(composeCommands(defs, new Map(), { on: true })[0].checked).toBe(true);
    expect(composeCommands(defs, new Map(), { on: false })[0].checked).toBe(false);
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

  test("invoking a defined command with no handler does not throw", () => {
    let run: (() => void) | undefined;
    function Runner() {
      run = useCommands().find((c) => c.id === ACTION.projectNew)?.run;
      return null;
    }
    render(
      <CommandRegistryProvider>
        <Runner />
      </CommandRegistryProvider>,
    );
    expect(run).toBeDefined();
    expect(() => run?.()).not.toThrow();
  });
});
