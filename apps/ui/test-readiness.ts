/// <reference types="bun" />
import { beforeEach } from "bun:test";
// bun runs every test file in one process and module mocks / module-global state are process-global,
// never reset between files (mock.restore does NOT undo mock.module in bun 1.3.x), so a file's stubs
// leak into whatever file runs next. Under a different filesystem order (macOS vs Linux CI) that
// leakage stalls or breaks sibling suites. Importing real-bindings here snapshots the real query()
// and setReady() before any test file registers a mock.

import { setReady } from "./app/lib/test/real-bindings";

// Readiness is module-global mutable state: a file that sets it false (a not-ready assertion or
// teardown) leaks false to the next file, stalling real query()s on the whenReady() gate (~1s
// timeouts). Default every test to ready; a suite asserting the not-ready -> ready transition opts
// out by calling setReady(false) at its start.
setReady(true);
beforeEach(() => setReady(true));
