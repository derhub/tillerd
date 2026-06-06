//! Deterministic retrieval metrics: Recall@K, MRR, NDCG@10. No model calls.

use std::collections::HashSet;

/// Fraction of a query's gold chunks that appear in the top `k` ranked ids.
pub fn recall_at_k(ranked: &[i64], gold: &HashSet<i64>, k: usize) -> f64 {
    if gold.is_empty() {
        return 0.0;
    }
    let found = ranked.iter().take(k).filter(|id| gold.contains(id)).count();
    found as f64 / gold.len() as f64
}

/// Reciprocal rank of the first gold hit (0 if none in the list).
pub fn reciprocal_rank(ranked: &[i64], gold: &HashSet<i64>) -> f64 {
    for (i, id) in ranked.iter().enumerate() {
        if gold.contains(id) {
            return 1.0 / (i as f64 + 1.0);
        }
    }
    0.0
}

/// NDCG@k with binary relevance (gold = 1, else 0).
pub fn ndcg_at_k(ranked: &[i64], gold: &HashSet<i64>, k: usize) -> f64 {
    let dcg: f64 = ranked
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, id)| {
            if gold.contains(id) {
                1.0 / ((i as f64 + 2.0).log2())
            } else {
                0.0
            }
        })
        .sum();
    let ideal_hits = gold.len().min(k);
    let idcg: f64 = (0..ideal_hits)
        .map(|i| 1.0 / ((i as f64 + 2.0).log2()))
        .sum();
    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

/// Running accumulator of metrics over a set of queries.
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub n: usize,
    pub recall1: f64,
    pub recall5: f64,
    pub recall10: f64,
    pub mrr: f64,
    pub ndcg10: f64,
    pub latency_ms: f64,
    pub result_size: f64,
}

impl Metrics {
    /// Fold one query's ranked result into the accumulator.
    pub fn add(&mut self, ranked: &[i64], gold: &HashSet<i64>, latency_ms: f64, result_size: usize) {
        self.n += 1;
        self.recall1 += recall_at_k(ranked, gold, 1);
        self.recall5 += recall_at_k(ranked, gold, 5);
        self.recall10 += recall_at_k(ranked, gold, 10);
        self.mrr += reciprocal_rank(ranked, gold);
        self.ndcg10 += ndcg_at_k(ranked, gold, 10);
        self.latency_ms += latency_ms;
        self.result_size += result_size as f64;
    }

    /// Means over all folded queries.
    pub fn averaged(&self) -> Averaged {
        let d = self.n.max(1) as f64;
        Averaged {
            n: self.n,
            recall1: self.recall1 / d,
            recall5: self.recall5 / d,
            recall10: self.recall10 / d,
            mrr: self.mrr / d,
            ndcg10: self.ndcg10 / d,
            latency_ms: self.latency_ms / d,
            result_size: self.result_size / d,
        }
    }
}

/// Averaged metrics for reporting.
#[derive(Debug, Clone)]
pub struct Averaged {
    pub n: usize,
    pub recall1: f64,
    pub recall5: f64,
    pub recall10: f64,
    pub mrr: f64,
    pub ndcg10: f64,
    pub latency_ms: f64,
    pub result_size: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gold(ids: &[i64]) -> HashSet<i64> {
        ids.iter().copied().collect()
    }

    #[test]
    fn recall_at_k_counts_gold_within_cutoff() {
        let ranked = [9, 3, 7, 1];
        assert_eq!(recall_at_k(&ranked, &gold(&[3]), 1), 0.0);
        assert_eq!(recall_at_k(&ranked, &gold(&[3]), 5), 1.0);
        assert_eq!(recall_at_k(&ranked, &gold(&[3, 1]), 5), 1.0);
        assert_eq!(recall_at_k(&ranked, &gold(&[3, 1]), 2), 0.5);
    }

    #[test]
    fn reciprocal_rank_uses_first_gold_position() {
        assert_eq!(reciprocal_rank(&[9, 3, 7], &gold(&[3])), 0.5);
        assert_eq!(reciprocal_rank(&[3, 9], &gold(&[3])), 1.0);
        assert_eq!(reciprocal_rank(&[9, 7], &gold(&[3])), 0.0);
    }

    #[test]
    fn ndcg_is_one_when_gold_is_ranked_first() {
        assert!((ndcg_at_k(&[3, 9, 7], &gold(&[3]), 10) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ndcg_is_lower_when_gold_is_ranked_later() {
        let first = ndcg_at_k(&[3, 9, 7], &gold(&[3]), 10);
        let later = ndcg_at_k(&[9, 7, 3], &gold(&[3]), 10);
        assert!(later < first);
    }

    #[test]
    fn empty_or_missing_gold_scores_zero() {
        assert_eq!(recall_at_k(&[1, 2], &gold(&[]), 5), 0.0);
        assert_eq!(reciprocal_rank(&[1, 2], &gold(&[9])), 0.0);
        assert_eq!(ndcg_at_k(&[1, 2], &gold(&[9]), 10), 0.0);
    }
}
