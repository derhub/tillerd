import type { ReactNode } from "react";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, expect, mock, test } from "bun:test";

let desktop = true;
let files: { name: string; path: string; size: number }[] = [];
let recordsByPath: Record<string, unknown[]> = {};

void mock.module("~/lib/transport", () => ({
  isDesktopHost: () => desktop,
}));

// Spread the real module so non-overridden exports stay intact: mock.module is process-global
// and persists across files, so a partial replacement would clobber sibling suites that use the
// real query/command wrappers. afterAll restores so this override does not leak past this file.
const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  query: (key: string, args?: { path?: string }) => ({
    queryKey: [key, args ?? null],
    queryFn: () => {
      if (key === "logList") return Promise.resolve(files);
      const path = args?.path ?? "";
      return Promise.resolve({ records: recordsByPath[path] ?? [], start: 0, end: 0 });
    },
  }),
  subscribe: () => ({ listen: () => Promise.resolve(() => {}) }),
  subscribeLogs: () => Promise.resolve({ teardown: () => Promise.resolve() }),
  useEventSub: () => {},
}));

const { LogViewer } = await import("./LogViewer");

afterEach(() => {
  cleanup();
  desktop = true;
  files = [];
  recordsByPath = {};
});

afterAll(() => mock.restore());

function wrapper({ children }: { children: ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

function recordView(message: string, service: string): unknown {
  return {
    timestamp: "2026-06-14T10:00:00Z",
    level: "INFO",
    body: message,
    attributes: {},
    resource: { "service.name": service },
    raw: message,
  };
}

test("shows a desktop-only notice when not running on the desktop host", () => {
  desktop = false;
  render(<LogViewer />, { wrapper });
  expect(screen.getByText(/desktop app/i)).toBeTruthy();
});

// Row virtualization needs real layout (happy-dom lacks it); covered by desktop e2e.
test("mounts the viewer chrome on the desktop host", async () => {
  files = [{ name: "tillerd-daemon.log", path: "/logs/tillerd-daemon.log", size: 42 }];
  recordsByPath["/logs/tillerd-daemon.log"] = [recordView("spawning pty", "tillerd-daemon")];

  render(<LogViewer />, { wrapper });

  expect(await screen.findByRole("button", { name: /load older/i })).toBeTruthy();
  expect(screen.getByLabelText("Level")).toBeTruthy();
  expect(screen.queryByText(/desktop app/i)).toBeNull();
});
