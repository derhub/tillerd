//! Entropy fallback for unknown secrets.

const MIN_LEN: usize = 20;
const MIN_ENTROPY: f64 = 3.5;

/// Shannon entropy in bits per character.
fn shannon(s: &str) -> f64 {
    let n = s.chars().count() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mut counts = std::collections::HashMap::new();
    for c in s.chars() {
        *counts.entry(c).or_insert(0u32) += 1;
    }
    counts
        .values()
        .map(|&c| {
            let p = c as f64 / n;
            -p * p.log2()
        })
        .sum()
}

/// A token looks like an unknown secret: long enough, restricted to a
/// secret-like alphabet, mixes letters and digits, and is high-entropy. The
/// gates exist to keep prose, ordinary identifiers, and numbers from tripping.
pub fn is_secret_like(token: &str) -> bool {
    if token.chars().count() < MIN_LEN {
        return false;
    }
    // Deliberately excludes `=`/`+`/`:` so labeled assignments and base64
    // padding are left to the pattern rules, not swallowed whole here.
    if !token
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_/".contains(c))
    {
        return false;
    }
    let has_digit = token.chars().any(|c| c.is_ascii_digit());
    let has_alpha = token.chars().any(|c| c.is_ascii_alphabetic());
    if !(has_digit && has_alpha) {
        return false;
    }
    shannon(token) >= MIN_ENTROPY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_high_entropy_mixed_token() {
        assert!(is_secret_like("a8Fk2Lm9Qz4Xy7Bp1Rt6Wn3"));
    }

    #[test]
    fn ignores_short_token() {
        assert!(!is_secret_like("abc123"));
    }

    #[test]
    fn ignores_prose_word() {
        assert!(!is_secret_like("internationalization"));
    }

    #[test]
    fn ignores_token_with_disallowed_chars() {
        assert!(!is_secret_like("this has spaces and stuff!!"));
    }
}
