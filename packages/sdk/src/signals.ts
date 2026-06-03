import type { SignalCategory, ExitQualifier } from "./types/events";

export interface SignalInfo {
  name: string;
  meaning: string;
  category: SignalCategory;
}

export type ResolvedSignal = SignalInfo | { name: "unknown"; raw: string | number };

const SIGNAL_TABLE: Record<string, { meaning: string; category: SignalCategory }> = {
  SIGHUP:    { meaning: "Controlling terminal closed or process leader exited", category: "graceful-termination" },
  SIGINT:    { meaning: "Interrupt from keyboard (Ctrl+C)", category: "graceful-termination" },
  SIGQUIT:   { meaning: "Quit from keyboard (Ctrl+\\) with possible core dump", category: "graceful-termination" },
  SIGTERM:   { meaning: "Polite termination request", category: "graceful-termination" },
  SIGKILL:   { meaning: "Forced kill — cannot be caught or ignored", category: "forced-termination" },
  SIGSEGV:   { meaning: "Invalid memory reference (segmentation fault)", category: "fault" },
  SIGABRT:   { meaning: "Abort — typically from failed assertion or abort(3)", category: "fault" },
  SIGFPE:    { meaning: "Floating-point or arithmetic exception", category: "fault" },
  SIGBUS:    { meaning: "Bus error — misaligned or non-existent memory address", category: "fault" },
  SIGILL:    { meaning: "Illegal CPU instruction", category: "fault" },
  SIGSYS:    { meaning: "Bad system call argument", category: "fault" },
  SIGTRAP:   { meaning: "Trace or breakpoint trap", category: "fault" },
  SIGSTOP:   { meaning: "Pause process — cannot be caught or ignored", category: "job-control" },
  SIGTSTP:   { meaning: "Stop signal from terminal (Ctrl+Z)", category: "job-control" },
  SIGCONT:   { meaning: "Continue if stopped", category: "job-control" },
  SIGTTIN:   { meaning: "Background process attempted terminal read", category: "job-control" },
  SIGTTOU:   { meaning: "Background process attempted terminal write", category: "job-control" },
  SIGPIPE:   { meaning: "Broken pipe — write to pipe with no readers", category: "resource" },
  SIGXCPU:   { meaning: "CPU time limit exceeded", category: "resource" },
  SIGXFSZ:   { meaning: "File size limit exceeded", category: "resource" },
  SIGALRM:   { meaning: "Timer signal from alarm(2)", category: "timer" },
  SIGVTALRM: { meaning: "Virtual alarm clock", category: "timer" },
  SIGPROF:   { meaning: "Profiling timer expired", category: "timer" },
  SIGUSR1:   { meaning: "User-defined signal 1", category: "user-defined" },
  SIGUSR2:   { meaning: "User-defined signal 2", category: "user-defined" },
  SIGCHLD:   { meaning: "Child process stopped or terminated", category: "child" },
  SIGWINCH:  { meaning: "Terminal window size changed", category: "window" },
  SIGURG:    { meaning: "Urgent condition on socket", category: "info" },
  SIGINFO:   { meaning: "Status request from keyboard", category: "info" },
  SIGPWR:    { meaning: "Power failure or restart", category: "info" },
  SIGSTKFLT: { meaning: "Stack fault on coprocessor (Linux)", category: "info" },
};

const LINUX_NUMBER_TO_NAME: Record<number, string> = {
  1: "SIGHUP",   2: "SIGINT",   3: "SIGQUIT",  4: "SIGILL",   5: "SIGTRAP",
  6: "SIGABRT",  7: "SIGBUS",   8: "SIGFPE",   9: "SIGKILL",  10: "SIGUSR1",
  11: "SIGSEGV", 12: "SIGUSR2", 13: "SIGPIPE", 14: "SIGALRM", 15: "SIGTERM",
  16: "SIGSTKFLT", 17: "SIGCHLD", 18: "SIGCONT", 19: "SIGSTOP", 20: "SIGTSTP",
  21: "SIGTTIN", 22: "SIGTTOU", 23: "SIGURG",  24: "SIGXCPU", 25: "SIGXFSZ",
  26: "SIGVTALRM", 27: "SIGPROF", 28: "SIGWINCH", 30: "SIGPWR", 31: "SIGSYS",
};

const MACOS_NUMBER_TO_NAME: Record<number, string> = {
  1: "SIGHUP",   2: "SIGINT",   3: "SIGQUIT",  4: "SIGILL",   5: "SIGTRAP",
  6: "SIGABRT",  8: "SIGFPE",   9: "SIGKILL",  10: "SIGBUS",  11: "SIGSEGV",
  12: "SIGSYS",  13: "SIGPIPE", 14: "SIGALRM", 15: "SIGTERM", 16: "SIGURG",
  17: "SIGSTOP", 18: "SIGTSTP", 19: "SIGCONT", 20: "SIGCHLD", 21: "SIGTTIN",
  22: "SIGTTOU", 23: "SIGIO",   24: "SIGXCPU", 25: "SIGXFSZ", 26: "SIGVTALRM",
  27: "SIGPROF", 28: "SIGWINCH", 29: "SIGINFO", 30: "SIGUSR1", 31: "SIGUSR2",
};

function numberToName(n: number): string | undefined {
  const platform = process.platform;
  const map = platform === "linux" ? LINUX_NUMBER_TO_NAME : MACOS_NUMBER_TO_NAME;
  return map[n];
}

export function resolveSignal(signal: string | number): ResolvedSignal {
  const name = typeof signal === "number" ? (numberToName(signal) ?? String(signal)) : signal;
  const entry = SIGNAL_TABLE[name];
  if (!entry) return { name: "unknown", raw: signal };
  return { name, meaning: entry.meaning, category: entry.category };
}

export function signalCategoryToQualifier(category: SignalCategory, killedByUser: boolean): ExitQualifier {
  if (killedByUser) return "stopped-by-request";
  switch (category) {
    case "fault": return "faulted";
    case "forced-termination": return "killed";
    case "graceful-termination": return "interrupted";
    case "resource": return "resource-exceeded";
    default: return "unknown";
  }
}
