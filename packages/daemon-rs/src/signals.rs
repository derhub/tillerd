//! Signal table and category mapping.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalCategory {
    GracefulTermination,
    ForcedTermination,
    Fault,
    JobControl,
    Resource,
    Timer,
    UserDefined,
    Child,
    Window,
    Info,
}

impl SignalCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            SignalCategory::GracefulTermination => "graceful-termination",
            SignalCategory::ForcedTermination => "forced-termination",
            SignalCategory::Fault => "fault",
            SignalCategory::JobControl => "job-control",
            SignalCategory::Resource => "resource",
            SignalCategory::Timer => "timer",
            SignalCategory::UserDefined => "user-defined",
            SignalCategory::Child => "child",
            SignalCategory::Window => "window",
            SignalCategory::Info => "info",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedSignal {
    Known {
        name: &'static str,
        meaning: &'static str,
        category: SignalCategory,
    },
    Unknown {
        raw: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalPlatform {
    Linux,
    Darwin,
}

impl SignalPlatform {
    pub fn host() -> Self {
        if cfg!(target_os = "macos") {
            SignalPlatform::Darwin
        } else {
            SignalPlatform::Linux
        }
    }
}

use SignalCategory::*;

fn signal_table(name: &str) -> Option<(&'static str, &'static str, SignalCategory)> {
    let row = match name {
        "SIGHUP" => (
            "SIGHUP",
            "Controlling terminal closed or process leader exited",
            GracefulTermination,
        ),
        "SIGINT" => (
            "SIGINT",
            "Interrupt from keyboard (Ctrl+C)",
            GracefulTermination,
        ),
        "SIGQUIT" => (
            "SIGQUIT",
            "Quit from keyboard (Ctrl+\\) with possible core dump",
            GracefulTermination,
        ),
        "SIGTERM" => ("SIGTERM", "Polite termination request", GracefulTermination),
        "SIGKILL" => (
            "SIGKILL",
            "Forced kill — cannot be caught or ignored",
            ForcedTermination,
        ),
        "SIGSEGV" => (
            "SIGSEGV",
            "Invalid memory reference (segmentation fault)",
            Fault,
        ),
        "SIGABRT" => (
            "SIGABRT",
            "Abort — typically from failed assertion or abort(3)",
            Fault,
        ),
        "SIGFPE" => ("SIGFPE", "Floating-point or arithmetic exception", Fault),
        "SIGBUS" => (
            "SIGBUS",
            "Bus error — misaligned or non-existent memory address",
            Fault,
        ),
        "SIGILL" => ("SIGILL", "Illegal CPU instruction", Fault),
        "SIGSYS" => ("SIGSYS", "Bad system call argument", Fault),
        "SIGTRAP" => ("SIGTRAP", "Trace or breakpoint trap", Fault),
        "SIGSTOP" => (
            "SIGSTOP",
            "Pause process — cannot be caught or ignored",
            JobControl,
        ),
        "SIGTSTP" => ("SIGTSTP", "Stop signal from terminal (Ctrl+Z)", JobControl),
        "SIGCONT" => ("SIGCONT", "Continue if stopped", JobControl),
        "SIGTTIN" => (
            "SIGTTIN",
            "Background process attempted terminal read",
            JobControl,
        ),
        "SIGTTOU" => (
            "SIGTTOU",
            "Background process attempted terminal write",
            JobControl,
        ),
        "SIGPIPE" => (
            "SIGPIPE",
            "Broken pipe — write to pipe with no readers",
            Resource,
        ),
        "SIGXCPU" => ("SIGXCPU", "CPU time limit exceeded", Resource),
        "SIGXFSZ" => ("SIGXFSZ", "File size limit exceeded", Resource),
        "SIGALRM" => ("SIGALRM", "Timer signal from alarm(2)", Timer),
        "SIGVTALRM" => ("SIGVTALRM", "Virtual alarm clock", Timer),
        "SIGPROF" => ("SIGPROF", "Profiling timer expired", Timer),
        "SIGUSR1" => ("SIGUSR1", "User-defined signal 1", UserDefined),
        "SIGUSR2" => ("SIGUSR2", "User-defined signal 2", UserDefined),
        "SIGCHLD" => ("SIGCHLD", "Child process stopped or terminated", Child),
        "SIGWINCH" => ("SIGWINCH", "Terminal window size changed", Window),
        "SIGURG" => ("SIGURG", "Urgent condition on socket", Info),
        "SIGINFO" => ("SIGINFO", "Status request from keyboard", Info),
        "SIGPWR" => ("SIGPWR", "Power failure or restart", Info),
        "SIGSTKFLT" => ("SIGSTKFLT", "Stack fault on coprocessor (Linux)", Info),
        _ => return None,
    };
    Some(row)
}

fn linux_number_to_name(n: i32) -> Option<&'static str> {
    Some(match n {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        4 => "SIGILL",
        5 => "SIGTRAP",
        6 => "SIGABRT",
        7 => "SIGBUS",
        8 => "SIGFPE",
        9 => "SIGKILL",
        10 => "SIGUSR1",
        11 => "SIGSEGV",
        12 => "SIGUSR2",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        16 => "SIGSTKFLT",
        17 => "SIGCHLD",
        18 => "SIGCONT",
        19 => "SIGSTOP",
        20 => "SIGTSTP",
        21 => "SIGTTIN",
        22 => "SIGTTOU",
        23 => "SIGURG",
        24 => "SIGXCPU",
        25 => "SIGXFSZ",
        26 => "SIGVTALRM",
        27 => "SIGPROF",
        28 => "SIGWINCH",
        30 => "SIGPWR",
        31 => "SIGSYS",
        _ => return None,
    })
}

fn macos_number_to_name(n: i32) -> Option<&'static str> {
    Some(match n {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        4 => "SIGILL",
        5 => "SIGTRAP",
        6 => "SIGABRT",
        8 => "SIGFPE",
        9 => "SIGKILL",
        10 => "SIGBUS",
        11 => "SIGSEGV",
        12 => "SIGSYS",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        16 => "SIGURG",
        17 => "SIGSTOP",
        18 => "SIGTSTP",
        19 => "SIGCONT",
        20 => "SIGCHLD",
        21 => "SIGTTIN",
        22 => "SIGTTOU",
        23 => "SIGIO",
        24 => "SIGXCPU",
        25 => "SIGXFSZ",
        26 => "SIGVTALRM",
        27 => "SIGPROF",
        28 => "SIGWINCH",
        29 => "SIGINFO",
        30 => "SIGUSR1",
        31 => "SIGUSR2",
        _ => return None,
    })
}

pub enum SignalInput {
    Name(String),
    Number(i32),
}

pub fn resolve_signal(signal: SignalInput, platform: SignalPlatform) -> ResolvedSignal {
    let (name, raw): (String, String) = match signal {
        SignalInput::Name(s) => (s.clone(), s),
        SignalInput::Number(n) => {
            let mapped = match platform {
                SignalPlatform::Linux => linux_number_to_name(n),
                SignalPlatform::Darwin => macos_number_to_name(n),
            };
            (
                mapped
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| n.to_string()),
                n.to_string(),
            )
        }
    };
    match signal_table(&name) {
        Some((name, meaning, category)) => ResolvedSignal::Known {
            name,
            meaning,
            category,
        },
        None => ResolvedSignal::Unknown { raw },
    }
}

pub fn signal_category_to_qualifier(
    category: SignalCategory,
    killed_by_user: bool,
) -> &'static str {
    if killed_by_user {
        return "stopped-by-request";
    }
    match category {
        SignalCategory::Fault => "faulted",
        SignalCategory::ForcedTermination => "killed",
        SignalCategory::GracefulTermination => "interrupted",
        SignalCategory::Resource => "resource-exceeded",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat_of(name: &str) -> SignalCategory {
        match resolve_signal(SignalInput::Name(name.into()), SignalPlatform::Linux) {
            ResolvedSignal::Known { category, .. } => category,
            _ => panic!("expected known signal {name}"),
        }
    }

    #[test]
    fn sigsegv_and_sigabrt_fault() {
        assert_eq!(
            signal_category_to_qualifier(cat_of("SIGSEGV"), false),
            "faulted"
        );
        assert_eq!(
            signal_category_to_qualifier(cat_of("SIGABRT"), false),
            "faulted"
        );
    }

    #[test]
    fn sigkill_killed_unless_user() {
        assert_eq!(
            signal_category_to_qualifier(cat_of("SIGKILL"), false),
            "killed"
        );
        assert_eq!(
            signal_category_to_qualifier(cat_of("SIGKILL"), true),
            "stopped-by-request"
        );
    }

    #[test]
    fn sigpipe_resource() {
        assert_eq!(
            signal_category_to_qualifier(cat_of("SIGPIPE"), false),
            "resource-exceeded"
        );
    }

    #[test]
    fn sighup_graceful_category() {
        assert_eq!(cat_of("SIGHUP"), SignalCategory::GracefulTermination);
    }

    #[test]
    fn unmapped_is_unknown() {
        match resolve_signal(SignalInput::Name("SIGFAKE".into()), SignalPlatform::Linux) {
            ResolvedSignal::Unknown { raw } => assert_eq!(raw, "SIGFAKE"),
            _ => panic!("expected unknown"),
        }
    }

    #[test]
    fn numeric_sigsegv_linux() {
        match resolve_signal(SignalInput::Number(11), SignalPlatform::Linux) {
            ResolvedSignal::Known { name, .. } => assert_eq!(name, "SIGSEGV"),
            _ => panic!("expected SIGSEGV"),
        }
    }
}
