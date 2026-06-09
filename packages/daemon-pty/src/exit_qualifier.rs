//! Exit-qualifier: primary exit field. Code/signal/category are diagnostic.

use crate::signals::{
    resolve_signal, signal_category_to_qualifier, ResolvedSignal, SignalInput, SignalPlatform,
};
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct ExitRaw {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
    #[serde(rename = "signalName", skip_serializing_if = "Option::is_none")]
    pub signal_name: Option<String>,
    #[serde(rename = "signalMeaning", skip_serializing_if = "Option::is_none")]
    pub signal_meaning: Option<String>,
    #[serde(rename = "signalCategory", skip_serializing_if = "Option::is_none")]
    pub signal_category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitTranslation {
    pub qualifier: &'static str,
    pub raw: ExitRaw,
}

pub fn translate_exit(
    killed_by_user: bool,
    code: Option<i32>,
    signal: Option<String>,
    platform: SignalPlatform,
) -> ExitTranslation {
    let base = ExitRaw {
        code,
        signal: signal.clone(),
        ..Default::default()
    };

    if killed_by_user {
        return ExitTranslation {
            qualifier: "stopped-by-request",
            raw: base,
        };
    }

    let Some(sig) = signal else {
        let qualifier = if code == Some(0) { "ok" } else { "error" };
        return ExitTranslation {
            qualifier,
            raw: base,
        };
    };

    let resolved = resolve_signal(SignalInput::Name(sig), platform);
    match resolved {
        ResolvedSignal::Unknown { .. } => ExitTranslation {
            qualifier: "unknown",
            raw: base,
        },
        ResolvedSignal::Known {
            name,
            meaning,
            category,
        } => {
            let raw = ExitRaw {
                signal_name: Some(name.to_string()),
                signal_meaning: Some(meaning.to_string()),
                signal_category: Some(category.as_str().to_string()),
                ..base
            };
            // SIGHUP maps to `hangup` (its graceful-termination category is shared
            // with SIGINT/SIGQUIT which map to `interrupted`).
            if name == "SIGHUP" {
                return ExitTranslation {
                    qualifier: "hangup",
                    raw,
                };
            }
            let qualifier = signal_category_to_qualifier(category, false);
            ExitTranslation { qualifier, raw }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(killed: bool, code: Option<i32>, sig: Option<&str>) -> &'static str {
        translate_exit(killed, code, sig.map(String::from), SignalPlatform::Linux).qualifier
    }

    #[test]
    fn self_exit_code_zero_ok() {
        assert_eq!(t(false, Some(0), None), "ok");
    }

    #[test]
    fn self_exit_nonzero_error() {
        assert_eq!(t(false, Some(1), None), "error");
    }

    #[test]
    fn killed_by_user_wins_over_everything() {
        assert_eq!(t(true, Some(0), Some("SIGKILL")), "stopped-by-request");
        assert_eq!(t(true, None, Some("SIGSEGV")), "stopped-by-request");
    }

    #[test]
    fn signal_categories() {
        assert_eq!(t(false, None, Some("SIGSEGV")), "faulted");
        assert_eq!(t(false, None, Some("SIGKILL")), "killed");
        assert_eq!(t(false, None, Some("SIGINT")), "interrupted");
        assert_eq!(t(false, None, Some("SIGPIPE")), "resource-exceeded");
    }

    #[test]
    fn sighup_is_hangup() {
        assert_eq!(t(false, None, Some("SIGHUP")), "hangup");
    }

    #[test]
    fn unknown_signal_unknown() {
        assert_eq!(t(false, None, Some("SIGFAKE")), "unknown");
    }

    #[test]
    fn signal_diagnostic_attached() {
        let r = translate_exit(false, None, Some("SIGSEGV".into()), SignalPlatform::Linux).raw;
        assert_eq!(r.signal_name.as_deref(), Some("SIGSEGV"));
        assert_eq!(r.signal_category.as_deref(), Some("fault"));
    }
}
