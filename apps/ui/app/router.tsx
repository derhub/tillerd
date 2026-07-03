import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { PersistQueryClientProvider } from "@tanstack/react-query-persist-client";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import { setQueryClient } from "@tillerd/client-bindings";
import React from "react";

import { DefaultErrorBoundary } from "~/components/shell/DefaultErrorBoundary";
import { DefaultNotFound } from "~/components/shell/DefaultNotFound";
import { mountCrossWindowInvalidate } from "~/lib/crossWindowSync";
import {
  makeQueryClient,
  onPersistRestored,
  PERSIST_BUSTER,
  PERSIST_MAX_AGE,
  queryPersister,
  shouldDehydrateQuery,
} from "~/lib/queryClient";
import { subscribe } from "~/lib/subscribe";
import { mountSurfaceStatusSync } from "~/lib/surfaceStatusSync";

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
  React.useEffect(() => mountSurfaceStatusSync(queryClient), []);

  const content = <RouterProvider router={router} />;

  if (!queryPersister) {
    return <QueryClientProvider client={queryClient}>{content}</QueryClientProvider>;
  }

  return (
    <PersistQueryClientProvider
      client={queryClient}
      persistOptions={{
        persister: queryPersister,
        maxAge: PERSIST_MAX_AGE,
        buster: PERSIST_BUSTER,
        dehydrateOptions: {
          shouldDehydrateQuery,
        },
      }}
      onSuccess={() => onPersistRestored(queryClient)}
    >
      {content}
    </PersistQueryClientProvider>
  );
}
