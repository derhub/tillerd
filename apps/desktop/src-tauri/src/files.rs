use std::io::SeekFrom;

use tauri::ipc::Response;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// Byte length, or `None` (JS `null`) when the file is absent — distinct from an empty file (0).
#[tauri::command]
pub async fn file_size(path: String) -> Option<u64> {
    tokio::fs::metadata(&path).await.ok().map(|m| m.len())
}

/// Read `length` bytes from `offset`; returns raw bytes (JS `ArrayBuffer`), short at EOF.
#[tauri::command]
pub async fn file_read(path: String, offset: u64, length: u64) -> Result<Response, String> {
    Ok(Response::new(read_bytes(&path, offset, length).await?))
}

pub(crate) async fn read_bytes(path: &str, offset: u64, length: u64) -> Result<Vec<u8>, String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| e.to_string())?;
    file.seek(SeekFrom::Start(offset))
        .await
        .map_err(|e| e.to_string())?;

    let mut buf = vec![0u8; length as usize];
    let mut filled = 0usize;
    while filled < buf.len() {
        let n = file
            .read(&mut buf[filled..])
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    buf.truncate(filled);
    Ok(buf)
}

/// One structured log file under the runtime logs directory.
#[derive(serde::Serialize)]
pub struct LogFileEntry {
    /// File name, e.g. `tillerd-daemon.2026-06-13.log`.
    pub name: String,
    /// Absolute path, passed back to `file_read` / `file_size`.
    pub path: String,
    /// Current byte size.
    pub size: u64,
}

/// List the structured `.log` files under the runtime logs directory, sorted by
/// name. Each entry carries the absolute path the renderer reads through
/// `file_read` / `file_size`. Empty when the logs directory is absent.
#[tauri::command]
pub async fn list_log_files() -> Vec<LogFileEntry> {
    let dir = tillerd_paths::logging::logs_dir_in(&tillerd_paths::runtime_dir());
    list_log_files_in(&dir).await
}

pub(crate) async fn list_log_files_in(dir: &std::path::Path) -> Vec<LogFileEntry> {
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
        entries.push(LogFileEntry {
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
    use std::io::Write;

    use super::*;

    #[tokio::test]
    async fn file_size_absent_returns_none() {
        assert_eq!(
            file_size("/nonexistent/path/zzz.dat".to_string()).await,
            None
        );
    }

    #[tokio::test]
    async fn file_size_returns_byte_count() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();
        let size = file_size(tmp.path().to_str().unwrap().to_string()).await;
        assert_eq!(size, Some(11));
    }

    #[tokio::test]
    async fn file_size_empty_file_is_zero_not_none() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let size = file_size(tmp.path().to_str().unwrap().to_string()).await;
        assert_eq!(size, Some(0));
    }

    #[tokio::test]
    async fn read_bytes_absent_file_returns_err() {
        assert!(read_bytes("/nonexistent/zzz", 0, 100).await.is_err());
    }

    #[tokio::test]
    async fn read_bytes_full_read_from_zero() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();
        let bytes = read_bytes(tmp.path().to_str().unwrap(), 0, 11)
            .await
            .unwrap();
        assert_eq!(bytes, b"hello world");
    }

    #[tokio::test]
    async fn read_bytes_reads_from_offset() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();
        let bytes = read_bytes(tmp.path().to_str().unwrap(), 6, 5)
            .await
            .unwrap();
        assert_eq!(bytes, b"world");
    }

    #[tokio::test]
    async fn read_bytes_short_at_eof() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"hi").unwrap();
        let bytes = read_bytes(tmp.path().to_str().unwrap(), 0, 100)
            .await
            .unwrap();
        assert_eq!(bytes, b"hi");
    }

    #[tokio::test]
    async fn list_log_files_returns_log_files_with_sizes() {
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
    async fn list_log_files_absent_dir_is_empty() {
        let got = list_log_files_in(std::path::Path::new("/nonexistent/zzz/logs")).await;
        assert!(got.is_empty());
    }
}
