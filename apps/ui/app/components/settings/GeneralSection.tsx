import { useQuery } from "@tanstack/react-query";
import { query } from "@tillerd/client-bindings";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";
import { Switch } from "~/components/ui/switch";
import { useBoolGlobalSetting, useGlobalSetting } from "~/lib/settings/context";
import { GENERAL_STARTUP_WORKSPACE_KEY, PANEL_CLOSE_CONFIRM_SKIP_KEY } from "~/lib/settings/keys";

const LAST_USED_WORKSPACE = "__last-used__";

// UI zoom (ui-settings-editor "General settings"; tasks.md 13.7) is a separate, not-yet-built
// setting -- deliberately absent here rather than a placeholder control.
export function GeneralSection() {
  const { data: workspaces = [] } = useQuery(query("workspaceList"));

  // Mirrors PanelContent's own consumer of the same key: the close-surface confirmation
  // dialog's "don't ask again" checkbox. This toggle re-enables asking by clearing the skip
  // flag (checked = confirmation on = skip flag off).
  const { value: skipCloseConfirm, setValue: setSkipCloseConfirm } = useBoolGlobalSetting(
    PANEL_CLOSE_CONFIRM_SKIP_KEY,
    false,
  );

  // Applied once at launch (see context.tsx's hydrateSettings), not a live pointer -- switching
  // workspaces during the session is unaffected. Unset ("Last used") keeps restoring the
  // last-active workspace, matching current behavior.
  const { value: startupWorkspaceId, setValue: setStartupWorkspaceId } = useGlobalSetting(
    GENERAL_STARTUP_WORKSPACE_KEY,
    "",
  );
  const selectedStartupWorkspace = startupWorkspaceId || LAST_USED_WORKSPACE;

  return (
    <section aria-labelledby="settings-general-heading" className="flex flex-col gap-3 max-w-sm">
      <h2
        id="settings-general-heading"
        className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground"
      >
        General
      </h2>

      <label className="flex items-center justify-between gap-3 text-foreground select-none">
        Confirm before closing a running terminal
        <Switch
          checked={!skipCloseConfirm}
          onCheckedChange={(checked) => setSkipCloseConfirm(!checked)}
        />
      </label>

      <div className="flex items-center justify-between gap-3">
        <span className="text-foreground">Startup workspace</span>
        <Select
          value={selectedStartupWorkspace}
          onValueChange={(value) =>
            value && setStartupWorkspaceId(value === LAST_USED_WORKSPACE ? "" : value)
          }
        >
          <SelectTrigger aria-label="Startup workspace" className="w-40">
            {/* The sentinel value isn't a real label -- resolve the displayed text explicitly
                instead of relying on SelectValue's default (which renders the raw value). */}
            <SelectValue>
              {(value: string) =>
                value === LAST_USED_WORKSPACE
                  ? "Last used"
                  : (workspaces.find((w) => w.id === value)?.name ?? value)
              }
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={LAST_USED_WORKSPACE}>Last used</SelectItem>
            {workspaces.map((w) => (
              <SelectItem key={w.id} value={w.id}>
                {w.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </section>
  );
}
