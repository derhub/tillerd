//! memorya CLI. Jobs are manual subcommands; scheduling lands with the daemon.

use contracts::SessionId;
use memorya::capture::HookCapturer;
use memorya::dual_mode::{self, CaptureMode, Face};
use memorya::hook_source::{GateSubscriptionSource, HookSource};
use memorya::jobs::Scope;
use memorya::worker::{self, EmbeddingWorker};
use memorya::{Engram, RecallResult};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

fn default_db_path() -> PathBuf {
    let base = std::env::var_os("ATHING_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".athing")
        });
    base.join("memorya.db")
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn usage() -> ! {
    eprintln!(
        "usage: memorya <command>\n\
         \n\
         commands:\n\
         \x20 status                      show active chunk count\n\
         \x20 index <dir>                 (re)index project markdown\n\
         \x20 embed                       embed any pending chunks\n\
         \x20 recall <query...>           hybrid recall (ids + titles)\n\
         \x20 search <query...>           hybrid recall with full content inline\n\
         \x20 archive-recall <query...>   search the archive\n\
         \x20 expand <id...>              print full content of one or more chunks\n\
         \x20 entity <name>               list currently-valid facts\n\
         \x20 consolidate <scope> [sid]   session|daily|weekly|monthly\n\
         \x20 evict [threshold]           move cold chunks to archive\n\
         \x20 prune [docs|all]            delete indexed docs (default) or wipe all memory\n\
         \x20 mcp                         run the MCP stdio server + loopback viewer\n\
         \x20 serve [port]                run the loopback HTTP viewer"
    );
    std::process::exit(2);
}

fn main() -> anyhow::Result<()> {
    let mut argv = std::env::args().skip(1);
    let cmd = argv.next().unwrap_or_else(|| "status".to_string());
    let rest: Vec<String> = argv.collect();

    // The long-running faces share one serialized memorya with the capture and
    // embedding-worker threads, so they take a separate path.
    if let Some(face) = dual_mode::face_for_subcommand(&cmd) {
        return run_face(face, &rest);
    }

    let memorya = Engram::open(default_db_path())?;

    match cmd.as_str() {
        "status" => {
            println!("memorya: {} active chunks", memorya.active_chunk_count()?);
        }
        "index" => {
            let dir = rest.first().cloned().unwrap_or_else(|| ".".to_string());
            let n = memorya.index_project(&dir, now())?;
            let embedded = memorya.embed_pending(10_000)?;
            println!("indexed {n} doc chunks under {dir}; embedded {embedded}");
        }
        "embed" => {
            let n = memorya.embed_pending(10_000)?;
            println!("embedded {n} chunks");
        }
        "recall" => {
            let q = rest.join(" ");
            memorya.embed_pending(10_000)?;
            match memorya.recall(&q, now())? {
                RecallResult::Found { hits } => {
                    for h in hits {
                        println!("#{}  {:.3}  {}", h.id, h.score, h.title);
                    }
                }
                RecallResult::Uncertain { .. } => {
                    println!("Not sure. Check the archive? (memorya archive-recall \"{q}\")");
                }
            }
        }
        "search" => {
            let q = rest.join(" ");
            memorya.embed_pending(10_000)?;
            let results = memorya.search(&q, now(), 5)?;
            if results.is_empty() {
                println!("Not sure. Check the archive? (memorya archive-recall \"{q}\")");
            }
            for r in results {
                println!(
                    "=== #{}  {:.3}  {} ===\n{}\n",
                    r.id, r.score, r.title, r.content
                );
            }
        }
        "archive-recall" => {
            let q = rest.join(" ");
            for h in memorya.archive_recall(&q, 10)? {
                println!("#{}  {:.3}  {}", h.id, h.score, h.title);
            }
        }
        "expand" => {
            let ids: Vec<i64> = rest.iter().filter_map(|s| s.parse().ok()).collect();
            if ids.is_empty() {
                usage();
            }
            let found = memorya.expand_many(&ids)?;
            for (id, content) in &found {
                println!("=== #{id} ===\n{content}");
            }
            if found.is_empty() {
                eprintln!("no chunks found for {ids:?}");
            }
        }
        "entity" => {
            let name = rest.first().cloned().unwrap_or_else(|| usage());
            for f in memorya.entity(&name)? {
                println!("{} = {}", f.predicate, f.value);
            }
        }
        "consolidate" => {
            let scope = rest.first().map(String::as_str).unwrap_or_else(|| usage());
            let id = match scope {
                "session" => {
                    let sid = rest.get(1).cloned().unwrap_or_else(|| usage());
                    memorya.consolidate_session(&sid, now())?
                }
                "daily" => memorya.consolidate(Scope::Daily, now())?,
                "weekly" => memorya.consolidate(Scope::Weekly, now())?,
                "monthly" => memorya.consolidate(Scope::Monthly, now())?,
                _ => usage(),
            };
            match id {
                Some(id) => println!("created {scope} digest #{id}"),
                None => println!("nothing to consolidate at {scope}"),
            }
        }
        "evict" => {
            let threshold: f32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(1.0);
            let n = memorya.run_eviction(now(), threshold)?;
            println!("archived {n} chunks");
        }
        "prune" => match rest.first().map(String::as_str).unwrap_or("docs") {
            "docs" => println!("pruned {} doc chunks", memorya.prune_docs()?),
            "all" => {
                memorya.prune_all()?;
                println!("pruned all memory (active db)");
            }
            _ => usage(),
        },
        _ => usage(),
    }
    Ok(())
}

/// Serve a long-running face (`mcp` or `serve`). The face shares one serialized
/// memorya with the capture and embedding-worker threads. In composed mode it
/// subscribes to the gate and embeds in the background; in standalone mode it is
/// memory-only.
fn run_face(face: Face, rest: &[String]) -> anyhow::Result<()> {
    let memorya = Arc::new(Mutex::new(Engram::open(default_db_path())?));
    let port: u16 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(37777);

    let capture = match dual_mode::capture_mode_from_env() {
        CaptureMode::Composed {
            gate_url,
            session_id,
        } => Some(start_composed_capture(
            memorya.clone(),
            &gate_url,
            session_id,
        )?),
        CaptureMode::Standalone => None,
    };

    match face {
        Face::McpWithViewer => {
            let viewer = memorya.clone();
            thread::spawn(move || {
                if let Err(e) = memorya::server::serve(viewer, port) {
                    eprintln!("memorya viewer failed: {e}");
                }
            });
            memorya::mcp::serve_stdio(memorya, now)?;
            // The MCP loop returned (stdin closed): drain and stop the workers.
            if let Some(capture) = capture {
                capture.shutdown();
            }
        }
        Face::ViewerOnly => {
            // The viewer loops until the process exits; the background threads
            // run alongside it and are torn down on exit.
            let _capture = capture;
            memorya::server::serve(memorya, port)?;
        }
    }
    Ok(())
}

/// The background threads of composed capture: a gate subscription feeding the
/// capture dispatcher, plus the embedding worker.
struct ComposedCapture {
    stop: Arc<AtomicBool>,
    worker: JoinHandle<()>,
}

impl ComposedCapture {
    /// Signal the worker to stop and wait for it. The capture thread blocks on
    /// the gate socket, so it is left to be torn down on process exit.
    fn shutdown(self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.worker.join();
    }
}

fn start_composed_capture(
    memorya: Arc<Mutex<Engram>>,
    gate_url: &str,
    session_id: String,
) -> anyhow::Result<ComposedCapture> {
    let source = GateSubscriptionSource::connect(gate_url, SessionId(session_id))?;
    let stop = Arc::new(AtomicBool::new(false));
    let worker = EmbeddingWorker::spawn(memorya.clone(), worker::drain_interval(), stop.clone());

    let capture_stop = stop.clone();
    thread::spawn(move || drive_capture(source, memorya, &capture_stop));

    Ok(ComposedCapture { stop, worker })
}

fn drive_capture(
    mut source: GateSubscriptionSource,
    memorya: Arc<Mutex<Engram>>,
    stop: &AtomicBool,
) {
    let capturer = HookCapturer::new(memorya);
    while !stop.load(Ordering::Relaxed) {
        match source.next() {
            Some(event) => {
                if let Err(e) = capturer.dispatch(&event) {
                    eprintln!("memorya capture: dispatch failed: {e}");
                }
            }
            // The gate closed the stream.
            None => break,
        }
    }
}
