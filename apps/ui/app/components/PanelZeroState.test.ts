import { expect, test } from "bun:test";

import { shouldOfferProjectCreation } from "./PanelZeroState";

test("offers project creation when no named projects and Unfiled is empty", () => {
  expect(shouldOfferProjectCreation(0, 0)).toBe(true);
});

test("keeps the picker when a named project exists", () => {
  expect(shouldOfferProjectCreation(1, 0)).toBe(false);
});

test("keeps the picker when Unfiled holds a session", () => {
  expect(shouldOfferProjectCreation(0, 1)).toBe(false);
});
