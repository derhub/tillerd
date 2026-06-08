//! Bounded exponential restart backoff.
//!
//! A managed child that exits is restarted with a delay that doubles each
//! attempt but is clamped to a fixed cap, so a child that fails persistently
//! does not spin.

use std::time::Duration;

/// Restart policy: base delay, growth cap, and an attempt ceiling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackoffPolicy {
    /// Delay before the first restart; doubled each subsequent attempt.
    pub base: Duration,
    /// Upper bound on any single restart delay.
    pub cap: Duration,
    /// Maximum number of restart attempts before giving up.
    pub max_attempts: u32,
}

impl Default for BackoffPolicy {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(250),
            cap: Duration::from_secs(30),
            max_attempts: 5,
        }
    }
}

impl BackoffPolicy {
    /// Delay before restart `attempt` (1-based): `base * 2^(attempt-1)`, clamped to `cap`.
    ///
    /// Attempt `0` is treated as attempt `1`. The shift is computed on saturating
    /// arithmetic so a large attempt count cannot overflow.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let step = attempt.saturating_sub(1).min(63);
        let factor = 1u128.checked_shl(step).unwrap_or(u128::MAX);
        let scaled = (self.base.as_millis()).saturating_mul(factor);
        let capped = scaled.min(self.cap.as_millis());
        Duration::from_millis(capped as u64)
    }

    /// Whether another restart is permitted after `attempts` have been made.
    pub fn should_retry(&self, attempts: u32) -> bool {
        attempts < self.max_attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> BackoffPolicy {
        BackoffPolicy {
            base: Duration::from_millis(100),
            cap: Duration::from_millis(800),
            max_attempts: 4,
        }
    }

    #[test]
    fn restart_backoff_uses_exponential_strategy() {
        let p = policy();
        assert_eq!(p.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(p.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(p.delay_for_attempt(3), Duration::from_millis(400));
    }

    #[test]
    fn restart_backoff_does_not_exceed_cap() {
        let p = policy();
        for attempt in 4..=64 {
            assert!(p.delay_for_attempt(attempt) <= p.cap);
        }
        assert_eq!(p.delay_for_attempt(64), p.cap);
    }

    #[test]
    fn restart_attempts_respect_max_attempts() {
        let p = policy();
        assert!(p.should_retry(0));
        assert!(p.should_retry(3));
        assert!(!p.should_retry(4));
        assert!(!p.should_retry(5));
    }
}
