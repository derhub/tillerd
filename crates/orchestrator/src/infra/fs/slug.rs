//! Filesystem slug derivation and collision suffixing (D5).

use std::path::Path;

/// Derive a filesystem slug from a display name.
///
/// Rules (D5): lowercase, non-alphanumeric → `-`, collapse/trim. If the result is
/// empty, use the short form of `fallback_id` (first 8 chars).
pub(crate) fn slugify(name: &str, fallback_id: &str) -> String {
    let raw: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    // Collapse consecutive `-` and trim leading/trailing `-`.
    let mut slug = String::new();
    let mut prev_dash = true; // treat start as dash to trim leading
    for c in raw.chars() {
        if c == '-' {
            if !prev_dash {
                slug.push('-');
                prev_dash = true;
            }
        } else {
            slug.push(c);
            prev_dash = false;
        }
    }
    // Trim trailing `-`
    let slug = slug.trim_end_matches('-').to_owned();

    if slug.is_empty() {
        fallback_id.chars().take(8).collect()
    } else {
        slug
    }
}

/// Pick a slug for a new entity in `parent_dir`, avoiding collisions.
///
/// If `<slug>` is taken, try `<slug>-2`, `<slug>-3`, …
pub(crate) fn unique_slug(parent_dir: &Path, base_slug: &str) -> String {
    let candidate = parent_dir.join(base_slug);
    if !candidate.exists() {
        return base_slug.to_owned();
    }
    let mut n = 2u32;
    loop {
        let s = format!("{base_slug}-{n}");
        if !parent_dir.join(&s).exists() {
            return s;
        }
        n += 1;
    }
}
