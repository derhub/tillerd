import { afterEach, expect, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";

import type { LogFileInfo, LogSource } from "~/lib/transport/log-source";
import { LogViewer } from "./LogViewer";

afterEach(cleanup);

function sourceWith(line: string): LogSource {
  const bytes = new TextEncoder().encode(line);
  return {
    list: (): Promise<LogFileInfo[]> =>
      Promise.resolve([
        { name: "tillerd-daemon.log", path: "/logs/tillerd-daemon.log", size: bytes.length },
      ]),
    size: (): Promise<number | null> => Promise.resolve(bytes.length),
    read: (): Promise<Uint8Array> => Promise.resolve(bytes),
  };
}

test("shows a desktop-only notice when no source is available", async () => {
  render(<LogViewer resolveSource={() => Promise.resolve(null)} />);
  expect(await screen.findByText(/desktop app/i)).toBeTruthy();
});

// Row rendering is virtualized (needs real layout, which happy-dom lacks) and is covered by the
// desktop e2e. Here we assert the viewer mounts its chrome when a source resolves.
test("mounts the viewer chrome when a source is available", async () => {
  const line = `${JSON.stringify({
    timestamp: "2026-06-14T10:00:00Z",
    level: "INFO",
    fields: { message: "spawning pty" },
    spans: [{ "service.name": "tillerd-daemon", name: "service" }],
  })}\n`;

  render(<LogViewer resolveSource={() => Promise.resolve(sourceWith(line))} pollMs={1_000_000} />);

  expect(await screen.findByRole("button", { name: /load older/i })).toBeTruthy();
  expect(screen.getByLabelText("Level")).toBeTruthy();
  expect(screen.queryByText(/desktop app/i)).toBeNull();
});
