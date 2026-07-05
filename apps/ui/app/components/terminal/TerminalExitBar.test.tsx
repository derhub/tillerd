import { cleanup, fireEvent, render, screen } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, expect, test } from "bun:test";

import { TerminalExitBar } from "./TerminalExitBar";

afterEach(cleanup);

test("shows the exited message on a clean exit", () => {
  render(<TerminalExitBar qualifier="ok" onRestart={() => {}} onNewSurface={() => {}} />);
  expect(screen.getByTestId("terminal-exit-bar").textContent).toContain("Process exited");
});

test("shows a stopped message when stopped by request", () => {
  render(
    <TerminalExitBar qualifier="stopped-by-request" onRestart={() => {}} onNewSurface={() => {}} />,
  );
  expect(screen.getByTestId("terminal-exit-bar").textContent).toContain("Process stopped");
});

test("clicking Restart fires onRestart", () => {
  let restarted = false;
  render(
    <TerminalExitBar qualifier="ok" onRestart={() => (restarted = true)} onNewSurface={() => {}} />,
  );
  fireEvent.click(screen.getByTestId("terminal-exit-restart"));
  expect(restarted).toBe(true);
});

test("clicking New surface fires onNewSurface", () => {
  let requestedNewSurface = false;
  render(
    <TerminalExitBar
      qualifier="ok"
      onRestart={() => {}}
      onNewSurface={() => (requestedNewSurface = true)}
    />,
  );
  fireEvent.click(screen.getByTestId("terminal-exit-new"));
  expect(requestedNewSurface).toBe(true);
});
