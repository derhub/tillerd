//! Retrieval evaluation harness. Indexes a committed corpus into a scratch
//! store, runs each labeled query through `rank`, and reports deterministic IR
//! metrics per category plus mean latency and result size.
//!
//! Run from the crate dir: cargo run --bin memorya-eval

use memorya::eval::{Averaged, Metrics};
use memorya::{ChunkKind, Engram, NewChunk};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const RETRIEVAL_DEPTH: usize = 10;

#[derive(Deserialize)]
struct CorpusItem {
    id: i64,
    title: String,
    content: String,
}

#[derive(Deserialize)]
struct Query {
    query: String,
    gold: Vec<i64>,
    category: String,
}

fn load<T: for<'de> Deserialize<'de>>(path: &str) -> anyhow::Result<Vec<T>> {
    let full = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path);
    let text = std::fs::read_to_string(&full)?;
    let mut out = Vec::new();
    for line in text.lines() {
        if !line.trim().is_empty() {
            out.push(serde_json::from_str(line)?);
        }
    }
    Ok(out)
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn main() -> anyhow::Result<()> {
    let corpus: Vec<CorpusItem> = load("eval/corpus.jsonl")?;
    let queries: Vec<Query> = load("eval/queries.jsonl")?;

    let base = std::env::temp_dir().join(format!("memorya-eval-{}", std::process::id()));
    std::fs::create_dir_all(&base)?;
    let memorya = Engram::open(base.join("memorya.db"))?;

    let mut to_corpus: HashMap<i64, i64> = HashMap::new();
    for item in &corpus {
        let chunk_id = memorya
            .ingest(NewChunk {
                session_id: None,
                kind: ChunkKind::Doc,
                content: item.content.clone(),
                title: Some(item.title.clone()),
                file_path: Some(format!("/eval/{}.md", item.id)),
                turn_index: None,
                ts: 0,
            })?
            .expect("corpus chunk inserted");
        to_corpus.insert(chunk_id, item.id);
    }
    memorya.embed_pending(1_000_000)?;

    let mut overall = Metrics::default();
    let mut by_category: BTreeMap<String, Metrics> = BTreeMap::new();

    for q in &queries {
        let gold: HashSet<i64> = q.gold.iter().copied().collect();
        let started = Instant::now();
        let ranked_memorya = memorya.rank(&q.query, now(), RETRIEVAL_DEPTH)?;
        let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
        let ranked: Vec<i64> = ranked_memorya
            .iter()
            .filter_map(|id| to_corpus.get(id).copied())
            .collect();
        overall.add(&ranked, &gold, latency_ms, ranked.len());
        by_category.entry(q.category.clone()).or_default().add(
            &ranked,
            &gold,
            latency_ms,
            ranked.len(),
        );
    }

    let avg = overall.averaged();
    println!(
        "\nmemorya retrieval eval — {} chunks, {} queries, depth {}\n",
        corpus.len(),
        queries.len(),
        RETRIEVAL_DEPTH
    );
    println!(
        "  {:<11}{:>4}  {:>7} {:>7} {:>7}  {:>6} {:>8}  {:>9} {:>8}",
        "group", "n", "R@1", "R@5", "R@10", "MRR", "NDCG@10", "latency", "results"
    );
    println!("  {}", "─".repeat(74));
    report("overall", &avg);
    for (cat, m) in &by_category {
        report(&format!("· {cat}"), &m.averaged());
    }

    let verdict = if avg.recall5 >= 0.9 {
        "strong"
    } else if avg.recall5 >= 0.75 {
        "ok"
    } else {
        "weak"
    };
    println!(
        "\n  primary: Recall@5 = {:.1}%  ({verdict})\n",
        avg.recall5 * 100.0
    );
    Ok(())
}

fn report(label: &str, a: &Averaged) {
    println!(
        "  {label:<11}{n:>4}  {r1:>6.1}% {r5:>6.1}% {r10:>6.1}%  {mrr:>6.3} {ndcg:>8.3}  {lat:>7.2}ms {size:>8.1}",
        n = a.n,
        r1 = a.recall1 * 100.0,
        r5 = a.recall5 * 100.0,
        r10 = a.recall10 * 100.0,
        mrr = a.mrr,
        ndcg = a.ndcg10,
        lat = a.latency_ms,
        size = a.result_size,
    );
}
