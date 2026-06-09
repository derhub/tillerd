//! Gateway: service-host child. Gateway owns supervisor teardown on shutdown.

use std::net::Ipv4Addr;
use std::path::Path;

use service_host::host::{ServeContext, Service, ServiceConfig};
use uuid::Uuid;

use crate::{build, transport, Gateway, McpConfig, GATEWAY_VERSION};

/// The hosted tool name; the manifest derives from it.
const SERVICE_NAME: &str = "gateway";

/// The gateway service: the resolved base override and, once `serve` runs, the
/// built gateway it tears down on shutdown.
pub struct GatewayService {
    base_override: Option<String>,
    gateway: Option<Gateway>,
}

impl GatewayService {
    /// Build the service from the environment (the `ATHING_DIR` base override).
    pub fn from_env() -> Self {
        Self {
            base_override: std::env::var("ATHING_DIR").ok(),
            gateway: None,
        }
    }

    /// Bind the loopback front and publish `gateway.url` under `base` so clients can
    /// reach the MCP face. Returns the listener and the control-plane bearer token.
    async fn bind_and_publish(base: &Path) -> std::io::Result<(tokio::net::TcpListener, String)> {
        std::fs::create_dir_all(base)?;
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let port = listener.local_addr()?.port();
        std::fs::write(base.join("gateway.url"), format!("http://127.0.0.1:{port}"))?;
        Ok((listener, gen_token()))
    }
}

/// Mint a 32-byte (64 hex) random token for the loopback control-plane guard.
fn gen_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

impl Service for GatewayService {
    fn config(&self) -> ServiceConfig {
        ServiceConfig::new(SERVICE_NAME, GATEWAY_VERSION)
            .with_base_override(self.base_override.clone())
    }

    async fn serve(&mut self, ctx: ServeContext) -> std::io::Result<()> {
        let config = McpConfig::load()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        let gateway = build(config)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        self.gateway = Some(gateway.clone());

        let (listener, token) = Self::bind_and_publish(ctx.paths.base_dir()).await?;
        tracing::info!(
            port = listener.local_addr()?.port(),
            "gateway listening on the loopback MCP face"
        );

        let router = transport::http::router(gateway, token);
        axum::serve(listener, router).await
    }

    async fn shutdown(&mut self) {
        if let Some(gateway) = &self.gateway {
            gateway.supervisor().shutdown().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // A process-wide counter guarantees distinct base dirs even when parallel
    // tests read the same wall-clock nanosecond.
    static BASE_SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_base(tag: &str) -> std::path::PathBuf {
        let seq = BASE_SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("gw-service-{tag}-{}-{seq}", std::process::id()))
    }

    #[test]
    fn config_identifies_the_tool_as_gateway() {
        let service = GatewayService {
            base_override: None,
            gateway: None,
        };

        let config = service.config();

        assert_eq!(config.name, "gateway");
        assert_eq!(config.version, GATEWAY_VERSION);
    }

    #[tokio::test]
    async fn binds_the_front_on_loopback_only() {
        let base = temp_base("loopback");

        let (listener, _token) = GatewayService::bind_and_publish(&base).await.unwrap();

        assert!(
            listener.local_addr().unwrap().ip().is_loopback(),
            "the MCP front binds 127.0.0.1 only"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn publishes_the_gateway_url_for_clients() {
        let base = temp_base("url");

        let _ = GatewayService::bind_and_publish(&base).await.unwrap();

        let url = std::fs::read_to_string(base.join("gateway.url")).unwrap();
        assert!(url.starts_with("http://127.0.0.1:"), "got: {url}");
        let _ = std::fs::remove_dir_all(&base);
    }
}
