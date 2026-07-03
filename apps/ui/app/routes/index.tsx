import { createFileRoute } from "@tanstack/react-router";

import { PanelContent } from "~/components/shell/PanelContent";
import { setActiveProject } from "~/lib/store";

export const Route = createFileRoute("/")({
  loader: () => {
    setActiveProject(null);
  },
  component: PanelContent,
});
