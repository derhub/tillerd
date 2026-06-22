//! RFC 3339 / ISO-8601 timestamp utilities for persistence and config.
//! Produces stable, sortable timestamps without external time dependencies.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current UTC timestamp in RFC 3339 / ISO-8601 format.
/// Produces a string suitable for database timestamps and user-config metadata.
/// Format: `YYYY-MM-DDTHH:MM:SSZ`.
pub fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let total_secs = secs;
    let min = secs / 60;
    let hour = min / 60;
    let day_total = hour / 24;

    let sec = total_secs % 60;
    let min = min % 60;
    let hour = hour % 24;

    let (year, month, day) = days_to_ymd(day_total as u32);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
/// Month is 1-based (1 = January), day is 1-based.
fn days_to_ymd(mut d: u32) -> (u32, u32, u32) {
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

/// Check if a year is a leap year in the Gregorian calendar.
fn is_leap(y: u32) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_iso8601_produces_valid_format() {
        let ts = now_iso8601();
        // Format: YYYY-MM-DDTHH:MM:SSZ (20 characters)
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..20], "Z");
    }

    #[test]
    fn now_iso8601_is_sortable() {
        let ts1 = now_iso8601();
        let ts2 = now_iso8601();
        // Timestamps should be lexicographically sortable (RFC 3339 with Z suffix)
        assert!(ts1 <= ts2);
    }

    #[test]
    fn days_to_ymd_epoch() {
        let (year, month, day) = days_to_ymd(0);
        assert_eq!((year, month, day), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_epoch_plus_one() {
        let (year, month, day) = days_to_ymd(1);
        assert_eq!((year, month, day), (1970, 1, 2));
    }

    #[test]
    fn days_to_ymd_leap_year() {
        // 2000 is a leap year (divisible by 400)
        // Days from 1970-01-01 to 2000-02-29
        // 30 years * 365 = 10950
        // Plus leap years: 1972, 1976, 1980, 1984, 1988, 1992, 1996 = 7 extra days
        // Plus 59 days for Jan 1 - Feb 29 in 2000 (31 + 29 - 1)
        let days = 30 * 365 + 7 + 59;
        let (year, month, day) = days_to_ymd(days as u32);
        assert_eq!((year, month, day), (2000, 2, 29));
    }

    #[test]
    fn days_to_ymd_non_leap_year() {
        // 1971 is NOT a leap year. Jan 1, 1971 + 31 = Feb 1, 1971
        // Days from epoch to Feb 1, 1971: 365 (1970) + 31 (Jan 1971) = 396
        let days = 365 + 31;
        let (year, month, day) = days_to_ymd(days as u32);
        assert_eq!((year, month, day), (1971, 2, 1));
    }

    #[test]
    fn is_leap_century_rule() {
        assert!(!is_leap(1900)); // divisible by 100 but not 400
        assert!(is_leap(2000)); // divisible by 400
        assert!(!is_leap(1970)); // not divisible by 4
        assert!(is_leap(1972)); // divisible by 4, not by 100
    }

    #[test]
    fn days_to_ymd_year_boundary() {
        // Last day of 1970
        let days = 365 - 1;
        let (year, month, day) = days_to_ymd(days as u32);
        assert_eq!((year, month, day), (1970, 12, 31));

        // First day of 1971
        let days = 365;
        let (year, month, day) = days_to_ymd(days as u32);
        assert_eq!((year, month, day), (1971, 1, 1));
    }
}
