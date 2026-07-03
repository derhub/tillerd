// Typed mirror of the Rust state-model tables . The orchestrator entities are
// the source of truth; stateModel.test.ts proves this mirror matches the committed
// state-model.contract.json fixture, so drift on either side fails the build. Guard
// evaluation here is advisory only -- the server enforces every rule.

export const WELL_KNOWN_IDS = {
  defaultWorkspace: "00000000-0000-0000-0000-000000000001",
  unfiledProject: "00000000-0000-0000-0000-000000000000",
} as const;

export type StateModelEntity = "workspace" | "project" | "session" | "surface";

export type GuardRuleId = "not-default" | "not-unfiled" | "active" | "archived";

export interface StateTransition {
  readonly action: string;
  readonly from: string;
  readonly to: string;
}

export interface GuardSpec {
  readonly action: string;
  readonly rule: GuardRuleId;
  readonly fields: readonly string[];
}

export interface EntityStateModel {
  readonly states: readonly string[];
  readonly transitions: readonly StateTransition[];
  readonly guards: readonly GuardSpec[];
}

const CONTAINER_TRANSITIONS: readonly StateTransition[] = [
  { action: "archive", from: "active", to: "archived" },
  { action: "restore", from: "archived", to: "active" },
];

export const STATE_MODEL: Readonly<Record<StateModelEntity, EntityStateModel>> = {
  workspace: {
    states: ["active", "archived"],
    transitions: CONTAINER_TRANSITIONS,
    guards: [
      { action: "archive", rule: "not-default", fields: ["id"] },
      { action: "archive", rule: "active", fields: ["status"] },
      { action: "discard", rule: "not-default", fields: ["id"] },
      { action: "restore", rule: "archived", fields: ["status"] },
    ],
  },
  project: {
    states: ["active", "archived"],
    transitions: CONTAINER_TRANSITIONS,
    guards: [
      { action: "archive", rule: "not-unfiled", fields: ["id"] },
      { action: "archive", rule: "active", fields: ["status"] },
      { action: "discard", rule: "not-unfiled", fields: ["id"] },
      { action: "move", rule: "not-unfiled", fields: ["id"] },
      { action: "restore", rule: "archived", fields: ["status"] },
    ],
  },
  session: {
    states: ["active", "archived"],
    transitions: CONTAINER_TRANSITIONS,
    guards: [],
  },
  surface: {
    states: ["pending", "live", "failed", "idle"],
    transitions: [
      { action: "spawn", from: "pending", to: "live" },
      { action: "spawn", from: "pending", to: "failed" },
      { action: "resume", from: "idle", to: "live" },
      { action: "resume", from: "failed", to: "live" },
      { action: "stop", from: "live", to: "idle" },
      { action: "reconcile", from: "live", to: "failed" },
      { action: "reconcile", from: "pending", to: "failed" },
    ],
    guards: [],
  },
};

export interface GuardRow {
  readonly id?: string;
  readonly status?: string;
}

function rulePasses(rule: GuardRuleId, row: GuardRow): boolean {
  switch (rule) {
    case "not-default":
      return row.id !== WELL_KNOWN_IDS.defaultWorkspace;
    case "not-unfiled":
      return row.id !== WELL_KNOWN_IDS.unfiledProject;
    case "active":
      return row.status !== "archived";
    case "archived":
      return row.status === "archived";
  }
}

// Advisory enablement: every guard the action declares must pass. Actions with no
// declared guards are always enabled -- the server remains the enforcer either way.
export function can(entity: StateModelEntity, action: string, row: GuardRow): boolean {
  return STATE_MODEL[entity].guards
    .filter((g) => g.action === action)
    .every((g) => rulePasses(g.rule, row));
}
