import { createFileRoute } from "@tanstack/react-router";
import { query } from "@tillerd/client-bindings";

import { PanelContent } from "~/components/shell/PanelContent";
import { setActiveProject } from "~/lib/store";
import { sessionLayoutQuery } from "~/lib/usePanelTree";

export const Route = createFileRoute("/session/$id")({
  // Warm the layout cache and resolve the active project ID to save in the store.
  loader: async ({ context, params }) => {
    void context.queryClient.ensureQueryData(sessionLayoutQuery(params.id));
    try {
      const session = await context.queryClient.ensureQueryData(
        query("sessionGet", { id: params.id }),
      );
      if (session) {
        setActiveProject(session.projectId);
      }
    } catch {
      // non-fatal; active project remains unresolved
    }
  },
  component: PanelContent,
});
