#![deny(unsafe_code)]

mod cell;
mod codec;
mod exit_qualifier;
mod hook_ingress;
mod manifest;
mod messages;
mod pty_session;
mod replay;
mod resolve;
mod server;
mod shell_env;
mod signals;
mod snapshot;
mod stopped_sessions;
mod vt;

use manifest::{athing_dir, daemon_sock, hooks_sock, Manifest};
use server::{Daemon, DAEMON_VERSION};
use tokio::sync::mpsc::unbounded_channel;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    rt.block_on(async_main());
}

async fn async_main() {
    let args: Vec<String> = std::env::args().collect();
    let is_handoff = args.iter().any(|a| a == "--handoff");

    if !is_handoff {
        shell_env::install_login_shell_env();
    }

    let dir = athing_dir();
    let _ = std::fs::create_dir_all(&dir);

    let sock = arg_value(&args, "--socket")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| daemon_sock(&dir));
    let sock_for_poll = sock.clone();
    let hooks = hooks_sock(&dir);

    let (events_tx, events_rx) = unbounded_channel();
    let daemon = Daemon::new(&dir, events_tx).with_paths(sock, hooks);

    if is_handoff {
        match arg_value(&args, "--snapshot") {
            Some(snap) => match snapshot::read_snapshot(std::path::Path::new(&snap)) {
                Ok(records) => {
                    let n = daemon.adopt_records(&records);
                    eprintln!("athing-daemon (rust): handoff adopted {n} session(s)");
                }
                Err(e) => eprintln!("handoff: snapshot read failed: {e}; starting empty"),
            },
            None => eprintln!("handoff: --snapshot missing; starting empty"),
        }
    }

    let manifest = Manifest::new(dir.clone());

    // Graceful shutdown on SIGTERM/SIGINT: cascade-terminate sessions, remove
    // the manifest, exit with no orphaned children.
    {
        let daemon = daemon.clone();
        let manifest_dir = dir.clone();
        tokio::spawn(async move {
            let mut term =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            let mut int =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
            tokio::select! {
                _ = term.recv() => {}
                _ = int.recv() => {}
            }
            daemon.shutdown();
            Manifest::new(manifest_dir).remove();
            std::process::exit(0);
        });
    }

    // Run the accept loop; write the manifest only AFTER the control socket is
    // bound, so a predecessor handing off to us sees our pid only once we are
    // actually serving (its handoff ack polls the manifest pid).
    let serve_daemon = daemon.clone();
    let serve_handle = tokio::spawn(async move {
        if let Err(e) = serve_daemon.serve(events_rx).await {
            eprintln!("daemon serve error: {e}");
            std::process::exit(1);
        }
    });

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !sock_for_poll.exists() && std::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    manifest.write(DAEMON_VERSION).ok();

    let _ = serve_handle.await;
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let prefix = format!("{flag}=");
    args.iter()
        .find_map(|a| a.strip_prefix(&prefix).map(|s| s.to_string()))
}
