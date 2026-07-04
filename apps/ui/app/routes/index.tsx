import { createFileRoute } from "@tanstack/react-router";

import { PanelZeroState } from "~/components/PanelZeroState";
import { setActiveProject } from "~/lib/store";

export const Route = createFileRoute("/")({
  loader: () => {
    setActiveProject(null);
  },
  component: PanelZeroState,
});
