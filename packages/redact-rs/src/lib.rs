//! Sensitive-data redaction: detect credentials and structured PII, then
//! replace each detected span with a fixed `[REDACTED]` marker.
//!
//! Detection layers: a regex catalog (credentials + structured PII), a
//! Shannon-entropy fallback for unknown secrets, and an allowlist that
//! suppresses structural false positives. For a labeled key/value pair only
//! the value is redacted; the key is preserved. Pure and deterministic.

mod entropy;
mod patterns;

const MARKER: &str = "[REDACTED]";

/// Redact credentials and structured PII in `input`. Returns the input
/// unchanged when nothing sensitive is detected.
pub fn redact(input: &str) -> String {
    let mut spans = detect(input);
    if spans.is_empty() {
        return input.to_string();
    }
    spans.sort_by_key(|s| s.0);

    // Merge overlapping/adjacent spans so one marker replaces a contiguous run.
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(spans.len());
    for (start, end) in spans {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }

    let mut out = String::with_capacity(input.len());
    let mut cursor = 0;
    for (start, end) in merged {
        if start > cursor {
            out.push_str(&input[cursor..start]);
        }
        out.push_str(MARKER);
        cursor = end;
    }
    out.push_str(&input[cursor..]);
    out
}

/// Collect byte ranges to redact from all detection layers.
fn detect(input: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();

    for rule in patterns::rules() {
        for caps in rule.re.captures_iter(input) {
            if let Some(m) = caps.get(rule.group) {
                spans.push((m.start(), m.end()));
            }
        }
    }
    spans.extend(patterns::credit_card_spans(input));
    spans.extend(patterns::phone_spans(input));

    // Entropy fallback over whitespace-delimited tokens, with byte offsets.
    let mut byte = 0;
    for token in input.split_inclusive(char::is_whitespace) {
        let trimmed = token.trim_end();
        let len = trimmed.len();
        if len > 0 && entropy::is_secret_like(trimmed) && !patterns::is_allowlisted(trimmed) {
            spans.push((byte, byte + len));
        }
        byte += token.len();
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_clean_text_unchanged() {
        let s = "the quick brown fox jumps over the lazy dog";
        assert_eq!(redact(s), s);
    }

    #[test]
    fn redacts_github_token() {
        let r = redact("token ghp_abcdef0123456789abcdefABCDEF0123 here");
        assert_eq!(r, "token [REDACTED] here");
    }

    #[test]
    fn redacts_email() {
        assert_eq!(
            redact("mail me at user@example.com ok"),
            "mail me at [REDACTED] ok"
        );
    }

    #[test]
    fn keeps_key_redacts_value() {
        assert_eq!(
            redact("API_KEY=hunter2supersecretvalue"),
            "API_KEY=[REDACTED]"
        );
    }

    #[test]
    fn redacts_jwt() {
        let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.dozjgNryP4J3jVmNHl0w5N";
        assert_eq!(redact(&format!("auth {jwt} end")), "auth [REDACTED] end");
    }

    #[test]
    fn redacts_luhn_valid_card_only() {
        assert_eq!(redact("card 4242424242424242 x"), "card [REDACTED] x");
        // Non-Luhn 16-digit run is left alone.
        assert_eq!(redact("id 1234567890123456 x"), "id 1234567890123456 x");
    }

    #[test]
    fn redacts_entropy_secret() {
        let r = redact("blob a8Fk2Lm9Qz4Xy7Bp1Rt6Wn3Vc5 done");
        assert_eq!(r, "blob [REDACTED] done");
    }

    #[test]
    fn preserves_uuid() {
        let s = "id 550e8400-e29b-41d4-a716-446655440000 ok";
        assert_eq!(redact(s), s);
    }

    #[test]
    fn redacts_phone_with_context() {
        assert_eq!(redact("phone: 415-555-2671"), "phone: [REDACTED]");
        // No context keyword -> not treated as a phone number.
        let bare = "build 415 555 2671 done";
        assert_eq!(redact(bare), bare);
    }
}
