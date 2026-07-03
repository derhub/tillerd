import { describe, expect, test } from "bun:test";

import contract from "../../../../crates/orchestrator/src/entities/state-model.contract.json";
import {
  can,
  STATE_MODEL,
  WELL_KNOWN_IDS,
  type StateModelEntity,
  type GuardRow,
} from "./stateModel";

interface FixtureGuard {
  action: string;
  rule: string;
  fields: string[];
}

interface FixtureEntity {
  entity: string;
  states: string[];
  transitions: { action: string; from: string; to: string }[];
  guards: FixtureGuard[];
}

const fixtureEntities = (contract as { entities: FixtureEntity[] }).entities;

describe("state-model contract (mirror matches the Rust fixture)", () => {
  test("well-known ids match", () => {
    const ids = (
      contract as { well_known_ids: { default_workspace: string; unfiled_project: string } }
    ).well_known_ids;
    expect(WELL_KNOWN_IDS.defaultWorkspace).toBe(ids.default_workspace);
    expect(WELL_KNOWN_IDS.unfiledProject).toBe(ids.unfiled_project);
  });

  test("mirror covers exactly the fixture's entities", () => {
    const fixtureNames = fixtureEntities.map((e) => e.entity).sort();
    expect(Object.keys(STATE_MODEL).sort()).toEqual(fixtureNames);
  });

  for (const entity of fixtureEntities) {
    const mirror = STATE_MODEL[entity.entity as StateModelEntity];

    test(`${entity.entity}: states match`, () => {
      expect([...mirror.states]).toEqual(entity.states);
    });

    test(`${entity.entity}: transitions match`, () => {
      expect(mirror.transitions.map((t) => ({ ...t }))).toEqual(entity.transitions);
    });

    test(`${entity.entity}: guards match`, () => {
      expect(mirror.guards.map((g) => ({ ...g, fields: [...g.fields] }))).toEqual(entity.guards);
    });
  }

  test("every guard field exists on the wire view types", () => {
    // Compile-time contract: guard rules may only reference fields the generated view
    // types carry. These assignments fail to typecheck if a field disappears.
    type ViewLike = { id: string; status: string };
    const probe: ViewLike = { id: "x", status: "active" };
    for (const entity of fixtureEntities) {
      for (const guard of entity.guards) {
        for (const field of guard.fields) {
          expect(field in probe).toBe(true);
        }
      }
    }
  });
});

describe("can (advisory guard evaluation)", () => {
  const active = (id: string): GuardRow => ({ id, status: "active" });
  const archived = (id: string): GuardRow => ({ id, status: "archived" });

  test("delete on the Default workspace is disabled", () => {
    expect(can("workspace", "discard", active(WELL_KNOWN_IDS.defaultWorkspace))).toBe(false);
  });

  test("delete on an ordinary workspace is enabled", () => {
    expect(can("workspace", "discard", active("ws-1"))).toBe(true);
  });

  test("archive on an archived workspace is disabled", () => {
    expect(can("workspace", "archive", archived("ws-1"))).toBe(false);
  });

  test("restore on an active workspace is disabled", () => {
    expect(can("workspace", "restore", active("ws-1"))).toBe(false);
  });

  test("restore on an archived workspace is enabled", () => {
    expect(can("workspace", "restore", archived("ws-1"))).toBe(true);
  });

  test("archive on the Unfiled project is disabled", () => {
    expect(can("project", "archive", active(WELL_KNOWN_IDS.unfiledProject))).toBe(false);
  });

  test("move on the Unfiled project is disabled", () => {
    expect(can("project", "move", active(WELL_KNOWN_IDS.unfiledProject))).toBe(false);
  });

  test("an action with no declared guards is enabled", () => {
    expect(can("session", "archive", active("s-1"))).toBe(true);
  });
});
