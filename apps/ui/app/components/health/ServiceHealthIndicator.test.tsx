import { QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, render, waitFor } from "@testing-library/react";
import { setQueryClient } from "@tillerd/client-bindings";
import { query } from "@tillerd/client-bindings";
/// <reference lib="dom" />
import { afterEach, expect, mock, test } from "bun:test";
import React from "react";

import { makeQueryClient } from "~/lib/queryClient";

// mock.module is process-global; spread the real module so `desktopHostStore` (imported by
// other suites, e.g. NotificationIndicator) survives once this mock is installed.
const actualDesktopHost = await import("~/lib/useDesktopHost");
void mock.module("~/lib/useDesktopHost", () => ({
  ...actualDesktopHost,
  useDesktopHost: () => ({ status: "ready" }),
}));

import { ServiceHealthIndicator } from "./ServiceHealthIndicator";

afterEach(cleanup);

test("the indicator renders live from the service-health query cache", async () => {
  const qc = makeQueryClient();
  setQueryClient(qc);
  const ready = [
    { name: "gate", version: "1", state: "ready" },
    { name: "daemon", version: "1", state: "ready" },
  ];
  qc.setQueryData(query("serviceHealth").queryKey, ready);

  const view = render(
    <QueryClientProvider client={qc}>
      <ServiceHealthIndicator />
    </QueryClientProvider>,
  );

  await waitFor(() => {
    expect(view.getByLabelText("Service health: ready")).toBeTruthy();
  });

  // A cache update (what an invalidation refetch produces) must reflect without a remount --
  // the indicator subscribes to the cache, it does not mirror a one-shot fetch into state.
  act(() => {
    qc.setQueryData(query("serviceHealth").queryKey, [
      { name: "gate", version: "1", state: "ready" },
      { name: "daemon", version: "1", state: "unavailable" },
    ]);
  });

  await waitFor(() => {
    expect(view.getByLabelText("Service health: failed")).toBeTruthy();
  });
});
