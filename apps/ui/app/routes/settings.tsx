import { createFileRoute } from "@tanstack/react-router";
import { query } from "@tillerd/client-bindings";

import { SettingsEditor } from "~/components/settings/SettingsEditor";

export const Route = createFileRoute("/settings")({
  // Non-blocking prewarm (render-as-you-fetch): the editor renders immediately and each
  // section degrades independently while its own list settles.
  loader: ({ context }) => {
    void context.queryClient.ensureQueryData(query("profileList"));
    void context.queryClient.ensureQueryData(query("themeList"));
  },
  component: SettingsEditor,
});
