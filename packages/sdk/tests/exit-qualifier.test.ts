import { test, expect, describe } from "bun:test";
import { exitToStatus, isRecoverable, qualifierToCoarse } from "../src/exit-qualifier";
import { resolveSignal, signalCategoryToQualifier } from "../src/signals";
import type { ExitQualifier } from "../src/types/events";

describe("exitToStatus", () => {
  test("ok maps to DONE", () => {
    expect(exitToStatus("ok")).toBe("DONE");
  });

  test("stopped-by-request maps to DONE", () => {
    expect(exitToStatus("stopped-by-request")).toBe("DONE");
  });

  test("crash-class qualifiers map to crashed", () => {
    const crashQualifiers: ExitQualifier[] = [
      "error",
      "killed",
      "faulted",
      "hangup",
      "interrupted",
      "resource-exceeded",
      "unknown",
    ];
    for (const q of crashQualifiers) {
      expect(exitToStatus(q)).toBe("crashed");
    }
  });
});

describe("isRecoverable", () => {
  test("ok is not recoverable", () => {
    expect(isRecoverable("ok")).toBe(false);
  });

  test("stopped-by-request is not recoverable", () => {
    expect(isRecoverable("stopped-by-request")).toBe(false);
  });

  test("crash-class qualifiers are recoverable", () => {
    expect(isRecoverable("error")).toBe(true);
    expect(isRecoverable("faulted")).toBe(true);
    expect(isRecoverable("killed")).toBe(true);
  });
});

describe("qualifierToCoarse", () => {
  test("stopped-by-request → user", () => {
    expect(qualifierToCoarse("stopped-by-request")).toBe("user");
  });

  test("ok → clean", () => {
    expect(qualifierToCoarse("ok")).toBe("clean");
  });

  test("crash-class → unexpected", () => {
    expect(qualifierToCoarse("error")).toBe("unexpected");
    expect(qualifierToCoarse("faulted")).toBe("unexpected");
    expect(qualifierToCoarse("killed")).toBe("unexpected");
  });
});

describe("resolveSignal — string names", () => {
  test("SIGSEGV resolves with fault category", () => {
    const r = resolveSignal("SIGSEGV");
    expect(r.name).toBe("SIGSEGV");
    if (r.name !== "unknown") {
      expect(r.category).toBe("fault");
      expect(r.meaning).toBeTruthy();
    }
  });

  test("SIGKILL resolves with forced-termination category", () => {
    const r = resolveSignal("SIGKILL");
    expect(r.name).toBe("SIGKILL");
    if (r.name !== "unknown") {
      expect(r.category).toBe("forced-termination");
    }
  });

  test("unknown signal name returns unknown shape with raw preserved", () => {
    const r = resolveSignal("SIGFAKE");
    expect(r.name).toBe("unknown");
    if (r.name === "unknown") {
      expect(r.raw).toBe("SIGFAKE");
    }
  });
});

describe("resolveSignal — numeric platform numbers", () => {
  test("SIGCHLD resolves consistently: Linux=17 → SIGCHLD, macOS=20 → SIGCHLD", () => {
    const platform = process.platform;
    const linuxChld = 17;
    const macosChld = 20;
    const n = platform === "linux" ? linuxChld : macosChld;
    const r = resolveSignal(n);
    expect(r.name).toBe("SIGCHLD");
  });

  test("SIGSEGV resolves consistently: both platforms use 11", () => {
    const r = resolveSignal(11);
    expect(r.name).toBe("SIGSEGV");
  });

  test("SIGUSR1 resolves to SIGUSR1 on current platform", () => {
    const platform = process.platform;
    const n = platform === "linux" ? 10 : 30;
    const r = resolveSignal(n);
    expect(r.name).toBe("SIGUSR1");
  });

  test("unmapped number returns unknown with raw number preserved", () => {
    const r = resolveSignal(9999);
    expect(r.name).toBe("unknown");
    if (r.name === "unknown") {
      expect(r.raw).toBe(9999);
    }
  });
});

describe("signalCategoryToQualifier", () => {
  test("fault → faulted", () => {
    expect(signalCategoryToQualifier("fault", false)).toBe("faulted");
  });

  test("forced-termination → killed", () => {
    expect(signalCategoryToQualifier("forced-termination", false)).toBe("killed");
  });

  test("graceful-termination → interrupted", () => {
    expect(signalCategoryToQualifier("graceful-termination", false)).toBe("interrupted");
  });

  test("resource → resource-exceeded", () => {
    expect(signalCategoryToQualifier("resource", false)).toBe("resource-exceeded");
  });

  test("killedByUser=true always returns stopped-by-request regardless of category", () => {
    expect(signalCategoryToQualifier("fault", true)).toBe("stopped-by-request");
    expect(signalCategoryToQualifier("forced-termination", true)).toBe("stopped-by-request");
  });

  test("other categories → unknown", () => {
    expect(signalCategoryToQualifier("timer", false)).toBe("unknown");
    expect(signalCategoryToQualifier("job-control", false)).toBe("unknown");
  });
});
