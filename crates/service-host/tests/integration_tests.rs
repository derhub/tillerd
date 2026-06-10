//! End-to-end lifecycle: clean stop must remove the manifest.

use std::time::Duration;

use service_host::host::{run, ServeContext, Service, ServiceConfig};
use service_host::manifest::Manifest;
use service_host::paths::Paths;

struct ManifestCheckService {
    config: ServiceConfig,
}

impl Service for ManifestCheckService {
    fn config(&self) -> ServiceConfig {
        self.config.clone()
    }

    async fn serve(&mut self, ctx: ServeContext) -> std::io::Result<()> {
        let manifest =
            Manifest::read(&ctx.paths.manifest_path()).expect("manifest present during serve");
        assert_eq!(manifest.version, "3.1.4");
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
    let service = ManifestCheckService {
        config: ServiceConfig::new("widget", "3.1.4").with_base_override(Some(base.clone())),
    };

    run(service).await.unwrap();

    let paths = Paths::resolve("widget", Some(&base));
    assert!(
        Manifest::read(&paths.manifest_path()).is_none(),
        "manifest removed on clean stop"
    );

    tokio::time::sleep(Duration::from_millis(10)).await;
    let _ = std::fs::remove_dir_all(&base);
}
