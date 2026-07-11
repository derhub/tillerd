import { useMutation, useQuery } from "@tanstack/react-query";
import { command } from "@tillerd/client-bindings";
import React from "react";

import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";
import { commandListQuery } from "~/lib/data/commands";
import { projectSettingsQuery } from "~/lib/data/settings";
import { launchTemplateListQuery, templateListQuery } from "~/lib/data/templates";
import { describeLaunchSpec } from "~/lib/launchSpec";
import { resolveDefaultTemplate, type TemplateSelection } from "~/lib/newSessionTemplate";
import { DEFAULT_TEMPLATE_KEY } from "~/lib/settings/keys";
import { useActiveProject } from "~/lib/store";

const NONE_VALUE = "none";
const LAUNCH_PREFIX = "launch:";
const LIBRARY_PREFIX = "library:";

function encode(selection: TemplateSelection): string {
  if (selection.kind === "empty") return NONE_VALUE;
  const prefix = selection.kind === "launch" ? LAUNCH_PREFIX : LIBRARY_PREFIX;
  return `${prefix}${selection.id}`;
}

function decode(value: string): TemplateSelection {
  if (value.startsWith(LAUNCH_PREFIX)) {
    return { kind: "launch", id: value.slice(LAUNCH_PREFIX.length) };
  }
  if (value.startsWith(LIBRARY_PREFIX)) {
    return { kind: "library", id: value.slice(LIBRARY_PREFIX.length) };
  }
  return { kind: "empty" };
}

// Project-scoped settings (ui-settings-editor "Project settings"): only rendered with an
// active project. Default template is read back by the new-session flow via
// resolveDefaultTemplate/projectSettingsQuery (lib/newSessionTemplate.ts, lib/data/settings.ts)
// -- this component is that value's only write path.
export function ProjectSection() {
  const projectId = useActiveProject();
  const enabled = Boolean(projectId);

  const { data: settings } = useQuery({ ...projectSettingsQuery(projectId ?? ""), enabled });
  const { data: launchTemplates = [] } = useQuery({
    ...launchTemplateListQuery(projectId ?? ""),
    enabled,
  });
  const { data: libraryTemplates = [] } = useQuery({ ...templateListQuery(), enabled });
  const { data: commands = [] } = useQuery({ ...commandListQuery(), enabled });
  const commandsById = React.useMemo(() => new Map(commands.map((c) => [c.id, c])), [commands]);

  const setDefaultTemplate = useMutation(command("settingSet"));
  const clearDefaultTemplate = useMutation(command("settingReset"));

  if (!projectId) return null;

  const current = resolveDefaultTemplate(settings) ?? { kind: "empty" as const };
  const selected = encode(current);

  const labelFor = (value: string): string => {
    if (value === NONE_VALUE) return "None";
    if (value.startsWith(LAUNCH_PREFIX)) {
      const id = value.slice(LAUNCH_PREFIX.length);
      const t = launchTemplates.find((lt) => lt.id === id);
      return t ? describeLaunchSpec(t.specJson, (cid) => commandsById.get(cid)?.name) : value;
    }
    const id = value.slice(LIBRARY_PREFIX.length);
    return libraryTemplates.find((t) => t.id === id)?.name ?? value;
  };

  const handleChange = (value: string | null) => {
    if (!value) return;
    if (value === NONE_VALUE) {
      clearDefaultTemplate.mutate({ scope: "project", projectId, key: DEFAULT_TEMPLATE_KEY });
      return;
    }
    setDefaultTemplate.mutate({
      scope: "project",
      projectId,
      key: DEFAULT_TEMPLATE_KEY,
      valueJson: JSON.stringify(decode(value)),
    });
  };

  return (
    <section aria-labelledby="settings-project-heading" className="flex flex-col gap-3 max-w-sm">
      <h2
        id="settings-project-heading"
        className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground"
      >
        Project
      </h2>

      <div className="flex items-center justify-between gap-3">
        <span className="text-foreground">Default template</span>
        <Select value={selected} onValueChange={handleChange}>
          <SelectTrigger aria-label="Default template" className="w-48">
            {/* Encoded values (e.g. "launch:lt-1") aren't display labels -- resolve the
                selected item's own label instead of SelectValue's default (the raw value). */}
            <SelectValue>{(value: string) => labelFor(value)}</SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={NONE_VALUE}>None</SelectItem>
            <SelectGroup>
              <SelectLabel>This project</SelectLabel>
              {launchTemplates.map((t) => (
                <SelectItem key={t.id} value={`${LAUNCH_PREFIX}${t.id}`}>
                  {describeLaunchSpec(t.specJson, (id) => commandsById.get(id)?.name)}
                </SelectItem>
              ))}
            </SelectGroup>
            <SelectGroup>
              <SelectLabel>Library</SelectLabel>
              {libraryTemplates.map((t) => (
                <SelectItem key={t.id} value={`${LIBRARY_PREFIX}${t.id}`}>
                  {t.name}
                </SelectItem>
              ))}
            </SelectGroup>
          </SelectContent>
        </Select>
      </div>
    </section>
  );
}
