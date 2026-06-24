import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { setQueryClient } from "@tillerd/client-bindings";
import React from "react";

import { DefaultErrorBoundary } from "~/components/shell/DefaultErrorBoundary";
import { DefaultNotFound } from "~/components/shell/DefaultNotFound";
import { mountCrossWindowInvalidate } from "~/lib/crossWindowSync";
import { makeQueryClient } from "~/lib/queryClient";
import { subscribe } from "~/lib/subscribe";

import { routeTree } from "./routeTree.gen";

const queryClient: QueryClient = makeQueryClient();
setQueryClient(queryClient);

// File-based routing: route tree generated from app/routes/ by the Vite plugin.
// defaultErrorComponent: last-resort catch boundary; defaultNotFoundComponent: stray-path fallback.
const router = createRouter({
  routeTree,
  context: { queryClient },
  // Query owns caching; disable router stale-while-revalidate so Query manages freshness.
  // No-op until preload/loaders are enabled.
  defaultPreloadStaleTime: 0,
  defaultErrorComponent: DefaultErrorBoundary,
  defaultNotFoundComponent: DefaultNotFound,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

export function AppRouter() {
  React.useEffect(() => subscribe(mountCrossWindowInvalidate(queryClient)), []);
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
}
