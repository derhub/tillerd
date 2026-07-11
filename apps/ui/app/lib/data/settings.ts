import { query } from "@tillerd/client-bindings";

// Effective settings for a project: global defaults merged with project-scoped
// overrides, project wins on collision (rooted at ["settings"], client.query's
// ENTITY map maps `settings*` -> "settings"). Read-only: there is no per-project
// settings write path yet (lands with the Project settings section, task 13.9).
export function projectSettingsQuery(projectId: string) {
  return query("settingsResolve", { projectId });
}
