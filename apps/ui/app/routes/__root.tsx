import type { QueryClient } from "@tanstack/react-query";

import { createRootRouteWithContext } from "@tanstack/react-router";
import { query } from "@tillerd/client-bindings";

import { RootLayout } from "~/components/shell/RootLayout";

export interface RouterContext {
  queryClient: QueryClient;
}

// validateSearch is a passthrough: narrowing would drop unrelated query params (e.g. ?service=).
export type RootSearch = Record<string, string>;

export const Route = createRootRouteWithContext<RouterContext>()({
  validateSearch: (raw: Record<string, unknown>): RootSearch => {
    const search: RootSearch = {};
    for (const [k, v] of Object.entries(raw)) {
      if (typeof v === "string") search[k] = v;
    }
    return search;
  },
  // Kick off sidebar reads without awaiting -- suspense boundary picks them up in-flight.
  loader: ({ context }) => {
    const qc = context.queryClient;
    void qc.ensureQueryData(query("workspaceList"));
    void qc.ensureQueryData(query("projectList", { workspaceId: null }));
    void qc.ensureQueryData(query("sessionList", { projectId: null, limit: null, offset: null }));
  },
  component: RootLayout,
});
