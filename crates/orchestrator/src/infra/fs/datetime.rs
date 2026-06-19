//! RFC 3339 / ISO-8601 timestamp formatting without an external time dependency.

/// Current UTC timestamp in RFC 3339 / ISO-8601 format.
pub(crate) fn now_iso8601() -> String {
    // Use std time only — no external time dep needed.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Format as a rough ISO-8601 UTC string.
    // Full RFC-3339 formatting without chrono:
    let s = secs;
    let min = s / 60;
    let hour = min / 60;
    let day_total = hour / 24;
    let sec = s % 60;
    let min = min % 60;
    let hour = hour % 24;
    // Days since epoch → rough year/month/day (good enough for storage; not shown to users)
    let (year, month, day) = days_to_ymd(day_total as u32);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn days_to_ymd(mut d: u32) -> (u32, u32, u32) {
    // Rata Die-style computation from Unix epoch (1970-01-01).
    let mut year = 1970u32;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u32;
    for &md in &month_days {
        if d < md {
            break;
        }
        d -= md;
        month += 1;
    }
    (year, month, d + 1)
}

fn is_leap(y: u32) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}
