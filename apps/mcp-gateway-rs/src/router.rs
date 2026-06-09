//! Namespace codec: pure, no state/I/O.

pub const SEP: &str = "__";

pub fn namespaced(backend: &str, name: &str) -> String {
    format!("{backend}{SEP}{name}")
}

// Split on the FIRST separator so the original name may contain it.
pub fn split(public: &str) -> Option<(&str, &str)> {
    public.split_once(SEP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_simple_name() {
        let n = namespaced("github", "create_issue");
        assert_eq!(n, "github__create_issue");
        assert_eq!(split(&n), Some(("github", "create_issue")));
    }

    #[test]
    fn split_keeps_separators_inside_the_tool_name() {
        assert_eq!(split("db__weird__tool"), Some(("db", "weird__tool")));
    }

    #[test]
    fn unnamespaced_name_has_no_backend() {
        assert_eq!(split("bare"), None);
    }
}
