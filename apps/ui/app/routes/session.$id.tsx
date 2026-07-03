import { createFileRoute } from "@tanstack/react-router";
import { query } from "@tillerd/client-bindings";

import { PanelContent } from "~/components/shell/PanelContent";
import { setGlobalSetting } from "~/lib/settings/context";
import { lastSessionKey } from "~/lib/settings/keys";
import { setActiveProject } from "~/lib/store";
import { sessionLayoutQuery } from "~/lib/usePanelTree";

export const Route = createFileRoute("/session/$id")({
  // Warm the layout cache and resolve the active project ID to save in the store. Both
  // non-blocking (render-as-you-fetch): the route renders immediately; the active-project
  // side effect applies when the session read settles, and a failure leaves it unresolved.
  loader: ({ context, params }) => {
    void context.queryClient.ensureQueryData(sessionLayoutQuery(params.id));
    void context.queryClient
      .ensureQueryData(query("sessionGet", { id: params.id }))
      .then((session) => {
        if (session) {
          setActiveProject(session.projectId);
          // Last-visited view pointer (ADR-0044); fire-and-forget like the rest.
          setGlobalSetting(lastSessionKey(session.projectId), session.id);
        }
      })
      .catch(() => {
        // non-fatal; active project remains unresolved
      });
  },
  component: PanelContent,
});
