import { cleanup, render, screen } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, test } from "bun:test";
import React from "react";

import {
  CommandRegistryProvider,
  useCommands,
  useRegisterCommands,
  type Command,
} from "./registry";

afterEach(cleanup);

function Register({ commands }: { commands: Command[] }) {
  const memo = React.useMemo(() => commands, [commands]);
  useRegisterCommands(memo);
  return null;
}

function Probe() {
  const commands = useCommands();
  return (
    <ul>
      {commands.map((c) => (
        <li key={c.id} data-testid="cmd">
          {c.id}:{c.title}
        </li>
      ))}
    </ul>
  );
}

const cmd = (id: string, title = id): Command => ({ id, title, run: () => {} });

describe("command registry", () => {
  test("exposes commands a mounted contributor registers", () => {
    render(
      <CommandRegistryProvider>
        <Register commands={[cmd("a"), cmd("b")]} />
        <Probe />
      </CommandRegistryProvider>,
    );
    expect(screen.getAllByTestId("cmd").map((el) => el.textContent)).toEqual(["a:a", "b:b"]);
  });

  test("merges commands from independent contributors", () => {
    render(
      <CommandRegistryProvider>
        <Register commands={[cmd("a")]} />
        <Register commands={[cmd("b")]} />
        <Probe />
      </CommandRegistryProvider>,
    );
    const ids = screen.getAllByTestId("cmd").map((el) => el.textContent);
    expect(ids).toContain("a:a");
    expect(ids).toContain("b:b");
  });

  test("a later contributor's id wins on collision", () => {
    render(
      <CommandRegistryProvider>
        <Register commands={[cmd("a", "first")]} />
        <Register commands={[cmd("a", "second")]} />
        <Probe />
      </CommandRegistryProvider>,
    );
    expect(screen.getByTestId("cmd").textContent).toBe("a:second");
  });

  test("unmounting a contributor removes its commands", () => {
    const { rerender } = render(
      <CommandRegistryProvider>
        <Register commands={[cmd("a")]} />
        <Probe />
      </CommandRegistryProvider>,
    );
    expect(screen.getAllByTestId("cmd")).toHaveLength(1);

    rerender(
      <CommandRegistryProvider>
        <Probe />
      </CommandRegistryProvider>,
    );
    expect(screen.queryAllByTestId("cmd")).toHaveLength(0);
  });

  test("useCommands is empty without a provider", () => {
    render(<Probe />);
    expect(screen.queryAllByTestId("cmd")).toHaveLength(0);
  });
});
