//! engram CLI. Jobs are manual subcommands; scheduling lands with the daemon.

use engram::jobs::Scope;
use engram::{Engram, RecallResult};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn default_db_path() -> PathBuf {
    let base = std::env::var_os("ATHING_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".athing")
        });
    base.join("engram.db")
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn usage() -> ! {
    eprintln!(
        "usage: engram <command>\n\
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
         \x20 mcp                         run the MCP stdio server\n\
         \x20 serve [port]                run the loopback HTTP viewer/ingress"
    );
    std::process::exit(2);
}

fn main() -> anyhow::Result<()> {
    let mut argv = std::env::args().skip(1);
    let cmd = argv.next().unwrap_or_else(|| "status".to_string());
    let rest: Vec<String> = argv.collect();
    let cmd = cmd.as_str();

    let engram = Engram::open(default_db_path())?;

    match cmd {
        "status" => {
            println!("engram: {} active chunks", engram.active_chunk_count()?);
        }
        "index" => {
            let dir = rest.first().cloned().unwrap_or_else(|| ".".to_string());
            let n = engram.index_project(&dir, now())?;
            let embedded = engram.embed_pending(10_000)?;
            println!("indexed {n} doc chunks under {dir}; embedded {embedded}");
        }
        "embed" => {
            let n = engram.embed_pending(10_000)?;
            println!("embedded {n} chunks");
        }
        "recall" => {
            let q = rest.join(" ");
            engram.embed_pending(10_000)?;
            match engram.recall(&q, now())? {
                RecallResult::Found { hits } => {
                    for h in hits {
                        println!("#{}  {:.3}  {}", h.id, h.score, h.title);
                    }
                }
                RecallResult::Uncertain { .. } => {
                    println!("Not sure. Check the archive? (engram archive-recall \"{q}\")");
                }
            }
        }
        "search" => {
            let q = rest.join(" ");
            engram.embed_pending(10_000)?;
            let results = engram.search(&q, now(), 5)?;
            if results.is_empty() {
                println!("Not sure. Check the archive? (engram archive-recall \"{q}\")");
            }
            for r in results {
                println!("=== #{}  {:.3}  {} ===\n{}\n", r.id, r.score, r.title, r.content);
            }
        }
        "archive-recall" => {
            let q = rest.join(" ");
            for h in engram.archive_recall(&q, 10)? {
                println!("#{}  {:.3}  {}", h.id, h.score, h.title);
            }
        }
        "expand" => {
            let ids: Vec<i64> = rest.iter().filter_map(|s| s.parse().ok()).collect();
            if ids.is_empty() {
                usage();
            }
            let found = engram.expand_many(&ids)?;
            for (id, content) in &found {
                println!("=== #{id} ===\n{content}");
            }
            if found.is_empty() {
                eprintln!("no chunks found for {ids:?}");
            }
        }
        "entity" => {
            let name = rest.first().cloned().unwrap_or_else(|| usage());
            for f in engram.entity(&name)? {
                println!("{} = {}", f.predicate, f.value);
            }
        }
        "consolidate" => {
            let scope = rest.first().map(String::as_str).unwrap_or_else(|| usage());
            let id = match scope {
                "session" => {
                    let sid = rest.get(1).cloned().unwrap_or_else(|| usage());
                    engram.consolidate_session(&sid, now())?
                }
                "daily" => engram.consolidate(Scope::Daily, now())?,
                "weekly" => engram.consolidate(Scope::Weekly, now())?,
                "monthly" => engram.consolidate(Scope::Monthly, now())?,
                _ => usage(),
            };
            match id {
                Some(id) => println!("created {scope} digest #{id}"),
                None => println!("nothing to consolidate at {scope}"),
            }
        }
        "evict" => {
            let threshold: f32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(1.0);
            let n = engram.run_eviction(now(), threshold)?;
            println!("archived {n} chunks");
        }
        "prune" => match rest.first().map(String::as_str).unwrap_or("docs") {
            "docs" => println!("pruned {} doc chunks", engram.prune_docs()?),
            "all" => {
                engram.prune_all()?;
                println!("pruned all memory (active db)");
            }
            _ => usage(),
        },
        "mcp" => {
            engram::mcp::serve_stdio(&engram, now)?;
        }
        "serve" => {
            let port: u16 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(37777);
            engram::server::serve(&engram, port, now)?;
        }
        _ => usage(),
    }
    Ok(())
}
