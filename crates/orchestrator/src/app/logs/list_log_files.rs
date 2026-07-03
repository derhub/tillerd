use std::path::Path;

use serde::Deserialize;

use crate::app::logs::view::LogFileView;
use crate::context::Ctx;
use crate::shared::message::Query;
use crate::shared::Result;

/// All `.log` files under the runtime logs directory, sorted by name.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListLogFiles;

impl Query<Ctx> for ListLogFiles {
    type Out = Vec<LogFileView>;
    async fn handle(&self, _cx: &Ctx) -> Result<Self::Out> {
        let dir = tillerd_paths::logging::logs_dir_in(&tillerd_paths::runtime_dir());
        Ok(list_log_files_in(&dir).await)
    }
}

pub(crate) async fn list_log_files_in(dir: &Path) -> Vec<LogFileView> {
    let mut entries = Vec::new();
    let Ok(mut rd) = tokio::fs::read_dir(dir).await else {
        return entries;
    };
    while let Ok(Some(e)) = rd.next_entry().await {
        let path = e.path();
        if path.extension().and_then(|x| x.to_str()) != Some("log") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(String::from) else {
            continue;
        };
        let size = e.metadata().await.map(|m| m.len()).unwrap_or(0);
        entries.push(LogFileView {
            name,
            path: path.to_string_lossy().into_owned(),
            size,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_log_files_with_sizes_sorted_by_name() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("tillerd-daemon.2026-06-13.log"), b"abc").unwrap();
        std::fs::write(tmp.path().join("tillerd-gate.2026-06-13.log"), b"hello").unwrap();
        std::fs::write(tmp.path().join("notes.txt"), b"ignore me").unwrap();

        let got = list_log_files_in(tmp.path()).await;

        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "tillerd-daemon.2026-06-13.log");
        assert_eq!(got[0].size, 3);
        assert_eq!(got[1].name, "tillerd-gate.2026-06-13.log");
        assert_eq!(got[1].size, 5);
    }

    #[tokio::test]
    async fn absent_dir_is_empty() {
        let got = list_log_files_in(Path::new("/nonexistent/zzz/logs")).await;
        assert!(got.is_empty());
    }
}
