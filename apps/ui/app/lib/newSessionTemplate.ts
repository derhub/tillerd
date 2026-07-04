import type { SettingView, TemplateView } from "@tillerd/client-bindings";

import { DEFAULT_TEMPLATE_KEY } from "~/lib/settings/keys";

// What the new-session flow instantiates: nothing, one of the project's own
// launch templates, or a portable library template (materialized into a
// project-scoped launch template before a session can reference it -- see
// ui-template-manager design decisions; session_create only accepts a
// launch-template id).
export type TemplateSelection =
  | { kind: "empty" }
  | { kind: "launch"; id: string }
  | { kind: "library"; id: string };

function isTemplateSelection(
  value: unknown,
): value is Exclude<TemplateSelection, { kind: "empty" }> {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return (v.kind === "launch" || v.kind === "library") && typeof v.id === "string";
}

// The project's configured default template (ui-settings-editor spec: "Project
// settings -> Default template honored"). Absent or malformed values resolve to
// null so callers fall back to an empty session, matching prior behavior.
export function resolveDefaultTemplate(
  settings: SettingView[] | undefined,
): TemplateSelection | null {
  const entry = settings?.find((s) => s.key === DEFAULT_TEMPLATE_KEY);
  if (!entry || !isTemplateSelection(entry.value)) return null;
  return entry.value;
}

// A library template's stored spec, looked up to materialize it into a project
// launch template.
export function librarySpecFor(
  templates: TemplateView[],
  id: string,
): { specVersion: number; specJson: string } | null {
  const t = templates.find((t) => t.id === id);
  return t ? { specVersion: t.specVersion, specJson: t.specJson } : null;
}
