import { query } from "@tillerd/client-bindings";

// Read factories for the template surfaces. The portable library is rooted at
// ["templates"]; a project's launch templates at ["launchTemplates"] (client.query
// ENTITY map). Mutations invalidate the matching root via meta.invalidates.
export function templateListQuery() {
  return query("templateList");
}

export function templateGetQuery(id: string) {
  return query("templateGet", { id });
}

export function launchTemplateListQuery(projectId: string) {
  return query("launchTemplateList", { projectId, limit: null, offset: null, after: null });
}

export function launchTemplateGetQuery(id: string) {
  return query("launchTemplateGet", { id });
}
