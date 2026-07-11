import { query } from "@tillerd/client-bindings";

// Read factories for the command library, rooted at the ["commands"] query key
// (client.query's ENTITY map maps `command*` -> "commands"). Mutations invalidate
// this root via meta.invalidates, so a create/edit/pin refetches the list.
export function commandListQuery() {
  return query("commandList");
}
