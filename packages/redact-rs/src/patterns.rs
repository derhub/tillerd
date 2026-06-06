//! Pattern catalog (credentials + structured PII), vendored from a
//! Presidio-compatible set, plus the checksum/context gates and the allowlist.

use regex::Regex;
use std::sync::OnceLock;

/// A redaction rule: a compiled pattern and which capture group to redact.
/// `group == 0` redacts the whole match; a higher group redacts only that
/// submatch (used for labeled key/value pairs, so the key is preserved).
pub struct Rule {
    pub re: Regex,
    pub group: usize,
}

/// Catalog compiled once. Labeled-pair rules come first and redact the value
/// group; bare credential/PII shapes redact the whole match.
pub fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        let mk = |p: &str, g: usize| Rule {
            re: Regex::new(p).expect("valid pattern"),
            group: g,
        };
        vec![
            // --- labeled key/value: redact the value, keep the key ---
            mk(
                r#"(?i)([a-z0-9_]*(?:api[_-]?key|secret|token|password|passwd|pwd|auth)[a-z0-9_]*\s*[=:]\s*)([^\s,;"']+)"#,
                2,
            ),
            mk(
                r#"(?i)("[a-z0-9_]*(?:key|token|secret|password|auth)[a-z0-9_]*"\s*:\s*")([^"]+)"#,
                2,
            ),
            mk(
                r#"(?i)(authorization\s*:\s*(?:bearer\s+|basic\s+)?)([A-Za-z0-9._~+/=-]+)"#,
                2,
            ),
            // --- bare credential shapes ---
            mk(r"\b(?:ghp|gho|ghs|ghu|github_pat)_[A-Za-z0-9_]{20,}\b", 0),
            mk(r"\bAKIA[0-9A-Z]{16}\b", 0),
            mk(r"\bsk-(?:ant|proj)?-?[A-Za-z0-9_-]{20,}\b", 0),
            mk(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b", 0),
            mk(r"\bxox[bpoas]-[A-Za-z0-9-]{10,}\b", 0),
            mk(r"\bglpat-[A-Za-z0-9_-]{20,}\b", 0),
            mk(
                r"(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?-----END [A-Z ]*PRIVATE KEY-----",
                0,
            ),
            // --- structured PII ---
            mk(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b", 0), // email
            mk(
                r"\b(?:25[0-5]|2[0-4]\d|[01]?\d?\d)(?:\.(?:25[0-5]|2[0-4]\d|[01]?\d?\d)){3}\b",
                0,
            ), // ipv4
            mk(r"\b(?:[0-9A-Fa-f]{1,4}:){7}[0-9A-Fa-f]{1,4}\b", 0), // ipv6
            mk(r"\b(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b", 0),  // mac
            mk(r"\b\d{3}-\d{2}-\d{4}\b", 0),                        // us-ssn
            mk(r"\b[A-Z]{2}\d{2}[A-Za-z0-9]{11,30}\b", 0),          // iban (compact)
        ]
    })
}

/// Credit-card candidates that pass the Luhn checksum. Returned as byte ranges.
pub fn credit_card_spans(input: &str) -> Vec<(usize, usize)> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"\b\d(?:[ -]?\d){12,18}\b").unwrap());
    re.find_iter(input)
        .filter(|m| {
            let digits: String = m.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
            luhn(&digits)
        })
        .map(|m| (m.start(), m.end()))
        .collect()
}

/// Phone candidates that have a nearby context keyword. Returned as byte ranges.
pub fn phone_spans(input: &str) -> Vec<(usize, usize)> {
    static RE: OnceLock<Regex> = OnceLock::new();
    static CTX: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?:\+?\d{1,3}[\-\s.]?)?\(?\d{3}\)?[\-\s.]?\d{3}[\-\s.]?\d{4}\b").unwrap()
    });
    let ctx = CTX.get_or_init(|| Regex::new(r"(?i)phone|tel|mobile|cell|fax|call").unwrap());
    re.find_iter(input)
        .filter(|m| {
            let lo = m.start().saturating_sub(24);
            ctx.is_match(&input[lo..m.start()])
        })
        .map(|m| (m.start(), m.end()))
        .collect()
}

/// Luhn checksum for payment-card numbers.
pub fn luhn(digits: &str) -> bool {
    let n = digits.len();
    if !(13..=19).contains(&n) {
        return false;
    }
    let mut sum = 0u32;
    for (i, b) in digits.bytes().rev().enumerate() {
        let mut d = (b - b'0') as u32;
        if !i.is_multiple_of(2) {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }
    sum.is_multiple_of(10)
}

/// Structurally-noisy values that must not be redacted by the entropy layer:
/// version-control object hashes, UUIDs, and version-number-like sequences.
pub fn is_allowlisted(token: &str) -> bool {
    static UUID: OnceLock<Regex> = OnceLock::new();
    static HEX: OnceLock<Regex> = OnceLock::new();
    static VER: OnceLock<Regex> = OnceLock::new();
    let uuid = UUID.get_or_init(|| {
        Regex::new(r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap()
    });
    let hex = HEX.get_or_init(|| Regex::new(r"(?i)^[0-9a-f]{7}$|^[0-9a-f]{40}$|^[0-9a-f]{64}$").unwrap());
    let ver = VER.get_or_init(|| Regex::new(r"^v?\d+(?:\.\d+){1,}$").unwrap());
    uuid.is_match(token) || hex.is_match(token) || ver.is_match(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luhn_accepts_valid_card() {
        assert!(luhn("4242424242424242"));
    }

    #[test]
    fn luhn_rejects_invalid() {
        assert!(!luhn("4242424242424241"));
    }

    #[test]
    fn allowlists_uuid_and_sha() {
        assert!(is_allowlisted("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_allowlisted("e43760e0d1678ae6a075ddbea4bdb51622ea0a95"));
        assert!(is_allowlisted("1.2.3"));
        assert!(!is_allowlisted("ghp_abcdef0123456789abcdef0123"));
    }
}
