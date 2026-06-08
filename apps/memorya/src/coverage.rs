//! Eviction scoring: age + staleness, scaled by coverage, minus access frequency.

const DAY_SECS: f32 = 86_400.0;

/// Fields needed to score a chunk for eviction.
#[derive(Debug, Clone, Copy)]
pub struct ChunkStat {
    pub id: i64,
    pub ts: i64,
    pub last_accessed: Option<i64>,
    pub access_count: i64,
    pub covered_by_digest: bool,
    pub covered_by_fact: bool,
}

fn coverage_weight(by_digest: bool, by_fact: bool) -> f32 {
    match (by_digest, by_fact) {
        (true, true) => 2.0,
        (true, false) => 1.5,
        (false, true) => 1.2,
        (false, false) => 0.5,
    }
}

/// Eviction score for a chunk at time `now` (unix seconds).
pub fn eviction_score(c: &ChunkStat, now: i64) -> f32 {
    let age = (now - c.ts).max(0) as f32 / DAY_SECS;
    let recency = (now - c.last_accessed.unwrap_or(c.ts)).max(0) as f32 / DAY_SECS;
    let freq = c.access_count as f32;
    let coverage = coverage_weight(c.covered_by_digest, c.covered_by_fact);
    ((age * 0.3) + (recency * 0.3)) * coverage - (freq * 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 0;

    /// A chunk `age_days` old at `NOW`, accessed `freq` times, with the given
    /// coverage. `last_accessed` is left unset (so recency tracks age).
    fn stat(age_days: i64, freq: i64, dig: bool, fact: bool) -> ChunkStat {
        ChunkStat {
            id: 1,
            ts: NOW - age_days * DAY_SECS as i64,
            last_accessed: None,
            access_count: freq,
            covered_by_digest: dig,
            covered_by_fact: fact,
        }
    }

    #[test]
    fn covered_chunks_outrank_uncovered_chunks_for_eviction() {
        let covered = stat(40, 0, true, true);
        let uncovered = stat(40, 0, false, false);
        assert!(eviction_score(&covered, NOW) > eviction_score(&uncovered, NOW));
    }

    #[test]
    fn frequent_access_lowers_the_eviction_score() {
        let rare = stat(40, 0, true, false);
        let frequent = stat(40, 20, true, false);
        assert!(eviction_score(&frequent, NOW) < eviction_score(&rare, NOW));
    }

    #[test]
    fn fresh_content_is_not_evicted() {
        assert!(eviction_score(&stat(0, 0, false, false), NOW) <= 0.0);
    }
}
