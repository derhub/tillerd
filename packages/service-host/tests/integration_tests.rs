//! End-to-end lifecycle over the crate's public surface: a service runs through
//! [`service_host::run`], the manifest and probe come up during serve, and a
//! clean stop removes the manifest and the probe socket.

use std::time::Duration;

use service_host::host::{run, ServeContext, Service, ServiceConfig};
use service_host::manifest::Manifest;
use service_host::paths::Paths;
use service_host::probe::probe_once;

struct ProbeOnceService {
    config: ServiceConfig,
}

impl Service for ProbeOnceService {
    fn config(&self) -> ServiceConfig {
        self.config.clone()
    }

    async fn serve(&mut self, ctx: ServeContext) -> std::io::Result<()> {
        // During serve the manifest is in place and the probe is reachable
        // without a credential, reporting the configured version.
        let manifest =
            Manifest::read(&ctx.paths.manifest_path()).expect("manifest present during serve");
        assert_eq!(manifest.version, "3.1.4");

        let (status, body) = probe_once(
            &ctx.paths.health_socket_path(),
            "GET /health HTTP/1.1\r\n\r\n",
        )
        .await?;
        assert_eq!(status, "200 OK");
        assert!(body.contains("\"version\":\"3.1.4\""));
        assert!(body.contains("\"reachable\":true"));
        Ok(())
    }
}

fn temp_base(tag: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("/tmp/sh-it-{tag}-{}-{nanos}", std::process::id())
}

#[tokio::test]
async fn host_run_serves_then_cleans_up_on_clean_stop() {
    let base = temp_base("clean-stop");
    let service = ProbeOnceService {
        config: ServiceConfig::new("widget", "3.1.4").with_base_override(Some(base.clone())),
    };

    run(service).await.unwrap();

    let paths = Paths::resolve("widget", Some(&base));
    assert!(
        Manifest::read(&paths.manifest_path()).is_none(),
        "manifest removed on clean stop"
    );
    assert!(
        !paths.health_socket_path().exists(),
        "probe (health) socket removed on clean stop"
    );

    // Give the runtime a beat, then clean up the temp base.
    tokio::time::sleep(Duration::from_millis(10)).await;
    let _ = std::fs::remove_dir_all(&base);
}
