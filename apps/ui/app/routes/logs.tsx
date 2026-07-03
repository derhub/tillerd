import { createFileRoute, useSearch } from "@tanstack/react-router";

import { LogViewer } from "~/components/logs/LogViewer";

export const Route = createFileRoute("/logs")({
  component: LogsRoute,
});

function LogsRoute() {
  // The `?service=` filter rides the passthrough root search.
  const service = useSearch({ from: "/logs", select: (s) => s.service });
  return <LogViewer initialService={service} />;
}
