import { createFileRoute } from "@tanstack/react-router";

import { PanelContent } from "~/components/shell/PanelContent";
import { sessionLayoutQuery } from "~/lib/usePanelTree";

export const Route = createFileRoute("/session/$id")({
  // Warm the layout cache without awaiting; usePanelTree reads the same key on arrival.
  loader: ({ context, params }) => {
    void context.queryClient.ensureQueryData(sessionLayoutQuery(params.id));
  },
  component: PanelContent,
});
