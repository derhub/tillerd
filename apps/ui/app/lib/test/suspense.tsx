import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { render } from "@testing-library/react";
/// <reference lib="dom" />
import React, { type ReactElement, type ReactNode } from "react";

import { makeQueryClient } from "~/lib/queryClient";

export function renderWithSuspense(
  ui: ReactElement,
  opts?: { client?: QueryClient; fallback?: ReactNode },
): ReturnType<typeof render> & { client: QueryClient } {
  const client = opts?.client ?? makeQueryClient();
  const fallback = opts?.fallback ?? <div data-testid="suspense-fallback" />;
  const result = render(
    <QueryClientProvider client={client}>
      <React.Suspense fallback={fallback}>{ui}</React.Suspense>
    </QueryClientProvider>,
  );
  return { client, ...result };
}
