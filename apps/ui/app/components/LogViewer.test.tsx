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

test("renders a record's OpenTelemetry fields from the source", async () => {
  const line = `${JSON.stringify({
    timestamp: "2026-06-13T10:00:00Z",
    level: "INFO",
    fields: { message: "spawning pty" },
    spans: [{ "service.name": "tillerd-daemon", "session.id": "s1", name: "service" }],
  })}\n`;
  const source = sourceWith(line);

  render(<LogViewer resolveSource={() => Promise.resolve(source)} pollMs={1_000_000} />);

  expect(await screen.findByText("spawning pty")).toBeTruthy();
  expect(screen.getByText("tillerd-daemon")).toBeTruthy();
  // "s1" appears both in the row and the session facet option, so match >= 1.
  expect(screen.getAllByText("s1").length).toBeGreaterThan(0);
});
