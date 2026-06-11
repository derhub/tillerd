# Roadmap

Status: `[WIP]` in progress · unlabeled = planned · `[HELP]` wants input.

0.1.0 items grouped as **Strategic/Foundational**, **Important**, or **Nice to have**.

> Status: 0.0.x, pre-release. Core components are scaffolded — PTY daemon, gate,
> desktop shell — but the app does not yet work end-to-end. 0.1.0 is the first
> complete release. See [CHANGELOG](../CHANGELOG.md).

---

## 0.0.1 — App opens, services run

The narrowest vertical slice: desktop opens, gate and daemon start, UI connects.
Nothing renders yet — this milestone proves the supervised startup chain works
end-to-end.

- Supervised startup — desktop adopt-or-spawns gate, then daemon; no service
  assumes another is already running.
- Bridge live — after ensure, `daemon_connect` succeeds and the byte bridge is
  open.
- UI loads — `useDesktopHost` reaches `ready`; app renders without errors (blank
  terminal is acceptable).

---

## 0.0.2 — Terminal works

A real terminal session opens and streams inside the desktop app.

- `DesktopTerminalPane` opens a PTY session, streams input and output through
  xterm.
- Session sidebar shows the active session.
- Works reproducibly on a fresh launch.

---

## 0.0.3 — Session survives restart

Sessions are durable across desktop restarts.

- Session model basics (ADR-0020) — create, list, resume, close; terminal surface
  reconnects to an existing session after restart.

---

## 0.1.0 — Solid foundation

The first complete release: agent surface, unified architecture, and distribution.
Terminal basics land in 0.0.x; 0.1.0 closes out the remaining strategic items.

### Strategic/Foundational

- Agent surface — agent pane renders, streams, and shows lifecycle and failure
  states (gate / daemon / agent down).
- Session model full (ADR-0020) — agent surface added; full create / list / resume
  / close with both terminal and agent surfaces.
- Hook-event path — desktop subscribes to the gate's hook fan-out and routes
  events to the right session / surface.
- Single host — retire the server path; the desktop is the only thing that starts
  and owns sessions, and the orchestration domain moves into a crate the shell
  calls.
- One service contract — every long-lived service born the same: lifecycle,
  discovery, health.
- Persistence model — formalize storage tiers; standardize durable state on SQLite
  (discovery files and ephemeral state excluded).
- Desktop distribution — signed, notarized bundles (dmg / AppImage / deb) plus
  auto-update.

### Important

- Desktop end-to-end test — a visual test driving the real app: spawn a session,
  assert the terminal renders and streams.
- One observability model — a single agent action traceable end-to-end across
  process hops.
- Drain-and-restart daemon upgrade — replace fd-handoff with a simpler
  planned-upgrade path. `[WIP]`
- Release pipeline — versioned releases and generated changelogs via changesets.
- Docs reconciliation — README and guides match the consolidated desktop-only
  architecture.

### Nice to have

- Validate the extension point — pressure-test the adapter contract with a second
  agent before relying on it.
