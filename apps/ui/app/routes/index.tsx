import { createFileRoute } from "@tanstack/react-router";

import { PanelContent } from "~/components/shell/PanelContent";

export const Route = createFileRoute("/")({
  component: PanelContent,
});
