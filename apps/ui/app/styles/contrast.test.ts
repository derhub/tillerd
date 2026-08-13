import { expect, test } from "bun:test";

type Tokens = Record<string, string>;

const themes = await Promise.all(
  ["light-2026.css", "dark-2026.css"].map(async (name) => {
    const css = await Bun.file(new URL(`./themes/${name}`, import.meta.url)).text();
    return Object.fromEntries(
      [...css.matchAll(/--([\w-]+):\s*(#[\da-f]{6})/gi)].map(([, token, value]) => [token, value]),
    ) as Tokens;
  }),
);
const terminalCss = await Bun.file(new URL("../app.css", import.meta.url)).text();
const terminalTokens = Object.fromEntries(
  [...terminalCss.matchAll(/--(terminal-[\w-]+):\s*(#[\da-f]{6})/gi)].map(([, token, value]) => [
    token,
    value,
  ]),
) as Tokens;

function luminance(hex: string): number {
  const channels = hex
    .slice(1)
    .match(/.{2}/g)!
    .map((channel) => Number.parseInt(channel, 16) / 255)
    .map((channel) => (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4));
  return 0.2126 * channels[0]! + 0.7152 * channels[1]! + 0.0722 * channels[2]!;
}

function contrast(foreground: string, background: string): number {
  const [lighter, darker] = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
  return (lighter! + 0.05) / (darker! + 0.05);
}

test("muted text on background passes", () => {
  for (const tokens of themes) {
    expect(contrast(tokens["muted-foreground"]!, tokens.background!)).toBeGreaterThanOrEqual(4.5);
  }
});

test("rendered chrome contrast audit passes", () => {
  const requiredPairs = [
    ["foreground", "background", 4.5],
    ["muted-foreground", "background", 4.5],
    ["foreground", "card", 4.5],
    ["muted-foreground", "card", 4.5],
    ["popover-foreground", "popover", 4.5],
    ["muted-foreground", "popover", 4.5],
    ["foreground", "muted", 4.5],
    ["foreground", "secondary", 4.5],
    ["secondary-foreground", "secondary", 4.5],
    ["accent-foreground", "accent", 4.5],
    ["primary-foreground", "primary", 4.5],
    ["destructive", "background", 4.5],
    ["destructive", "card", 4.5],
    ["destructive", "muted", 4.5],
    ["ring", "background", 3],
    ["ring", "card", 3],
  ] as const;

  for (const tokens of themes) {
    for (const [foreground, background, threshold] of requiredPairs) {
      expect(contrast(tokens[foreground]!, tokens[background]!)).toBeGreaterThanOrEqual(threshold);
    }
  }
});

test("terminal search chrome contrast passes", () => {
  expect(
    contrast(terminalTokens["terminal-fg"]!, terminalTokens["terminal-surface"]!),
  ).toBeGreaterThanOrEqual(4.5);
  expect(
    contrast(terminalTokens["terminal-muted"]!, terminalTokens["terminal-surface"]!),
  ).toBeGreaterThanOrEqual(4.5);
  expect(
    contrast(terminalTokens["terminal-muted"]!, terminalTokens["terminal-surface"]!),
  ).toBeGreaterThanOrEqual(3);
});
