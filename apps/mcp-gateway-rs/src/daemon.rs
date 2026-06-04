//! Daemon runtime: bind the loopback front, publish the manifest, serve.

use std::io::Read;

use crate::manifest::{Manifest, DAEMON_VERSION};
use crate::{build, transport};

fn gen_token() -> anyhow::Result<String> {
    let mut buf = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut buf)?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = term => {}
    }
    tracing::info!("shutdown signal received");
}

// Best-effort detach; no-op when already a session leader.
#[cfg(unix)]
fn detach() {
    match nix::unistd::setsid() {
        Ok(_) => tracing::debug!("detached into new session"),
        Err(e) => tracing::debug!(?e, "setsid skipped (already session leader)"),
    }
}
#[cfg(not(unix))]
fn detach() {}


pub async fn run() -> anyhow::Result<()> {
    if let Some(existing) = Manifest::read() {
        if existing.is_reusable() {
            tracing::info!(
                port = existing.port,
                "gateway already running; deferring to it"
            );
            return Ok(());
        }
        // Stale manifest: overwrite by continuing to spawn.
    }
    detach();

    let config = crate::config::McpConfig::load()?;
    let gateway = build(config).await?;

    let token = gen_token()?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();

    let manifest = Manifest {
        pid: std::process::id(),
        port,
        token: token.clone(),
        version: DAEMON_VERSION.to_string(),
    };
    manifest.write()?;
    tracing::info!(port, "gateway daemon listening on http://127.0.0.1:{port}/mcp");

    let router = transport::http::router(gateway.clone(), token);
    let serve = axum::serve(listener, router).with_graceful_shutdown(shutdown_signal());

    let result = serve.await;

    gateway.supervisor().shutdown().await;
    Manifest::remove();
    result?;
    Ok(())
}
