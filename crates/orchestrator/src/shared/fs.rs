//! Atomic file read/write/list/delete utilities for user-config (settings, profiles,
//! themes, keybindings). Provides common operations with proper error handling.

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::errors::Error;

pub async fn read_string(path: impl AsRef<Path>) -> Result<String, Error> {
    tokio::fs::read_to_string(path).await.map_err(Error::from)
}

pub async fn read_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>, Error> {
    tokio::fs::read(path).await.map_err(Error::from)
}

/// A window of complete lines from a file, bounded by absolute byte offsets.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tail {
    /// Complete lines in file order, newline stripped.
    pub lines: Vec<String>,
    /// Byte offset of the first returned line.
    pub start: u64,
    /// Byte offset one past the last newline; where the next read begins.
    pub end: u64,
}

/// Read complete lines from `path` within `[from, from + max_bytes)`. With `align` set (and
/// `from > 0`) the partial first line is dropped so the window starts on a line boundary -- for
/// reads that begin mid-line (backfilling from the end or an older chunk). With `align` clear,
/// `from` is taken as a known boundary (continuing a prior read) and the first line is kept. A
/// partial trailing line is always deferred (re-read from `end`). Empty when the file is absent
/// or the range holds no newline. Newlines never fall inside a UTF-8 sequence, so byte-windowed
/// lines decode whole.
pub async fn tail(
    path: impl AsRef<Path>,
    from: u64,
    max_bytes: u64,
    align: bool,
) -> Result<Tail, Error> {
    let path = path.as_ref().to_path_buf();
    tokio::task::spawn_blocking(move || tail_sync(&path, from, max_bytes, align))
        .await
        .map_err(|e| Error::from(std::io::Error::other(e)))?
}

fn tail_sync(path: &Path, from: u64, max_bytes: u64, align: bool) -> Result<Tail, Error> {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Tail { lines: Vec::new(), start: from, end: from });
        }
        Err(e) => return Err(Error::from(e)),
    };
    let len = file.metadata().map_err(Error::from)?.len();
    let from = from.min(len);
    file.seek(SeekFrom::Start(from)).map_err(Error::from)?;
    // Stream complete lines from the seek point, capping the scan at max_bytes. Holds one line
    // at a time plus the small BufReader buffer -- never the file or the whole window.
    let mut reader = BufReader::new(file.take(max_bytes));
    let mut line = Vec::new();
    let mut consumed: u64 = 0;

    if align && from > 0 {
        let n = reader.read_until(b'\n', &mut line).map_err(Error::from)?;
        if line.last() != Some(&b'\n') {
            return Ok(Tail { lines: Vec::new(), start: from, end: from });
        }
        consumed += n as u64;
        line.clear();
    }

    let start = from + consumed;
    let mut end = start;
    let mut lines = Vec::new();
    loop {
        line.clear();
        let n = reader.read_until(b'\n', &mut line).map_err(Error::from)?;
        // A line without a trailing newline is partial (window or EOF cut it) -- defer it.
        if n == 0 || line.last() != Some(&b'\n') {
            break;
        }
        consumed += n as u64;
        end = from + consumed;
        let body = &line[..line.len() - 1];
        if !body.is_empty() {
            lines.push(String::from_utf8_lossy(body).into_owned());
        }
    }
    Ok(Tail { lines, start, end })
}

/// Write a string to a file, creating or truncating it. Parent directory must exist.
pub async fn write_string(path: impl AsRef<Path>, content: &str) -> Result<(), Error> {
    tokio::fs::write(path, content).await.map_err(Error::from)
}

/// Write bytes to a file, creating or truncating it. Parent directory must exist.
pub async fn write_bytes(path: impl AsRef<Path>, content: &[u8]) -> Result<(), Error> {
    tokio::fs::write(path, content).await.map_err(Error::from)
}

/// Delete a file if it exists. Returns `Ok(())` whether the file was deleted or not found.
pub async fn delete(path: impl AsRef<Path>) -> Result<(), Error> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::from(e)),
    }
}

/// List all entries in a directory. Returns `Ok(vec![])` if the directory does not exist.
pub async fn list_entries(path: impl AsRef<Path>) -> Result<Vec<PathBuf>, Error> {
    let mut dir = match tokio::fs::read_dir(path).await {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(Error::from(e)),
    };
    let mut result = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        result.push(entry.path());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::fs as std_fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn write_and_read_string_round_trip() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        let content = "hello world";

        write_string(&file, content).await.unwrap();
        let read = read_string(&file).await.unwrap();

        assert_eq!(read, content);
    }

    #[tokio::test]
    async fn write_and_read_bytes_round_trip() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.bin");
        let content = b"hello \x00 world";

        write_bytes(&file, content).await.unwrap();
        let read = read_bytes(&file).await.unwrap();

        assert_eq!(read, content);
    }

    #[tokio::test]
    async fn write_truncates_existing_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");

        write_string(&file, "original content").await.unwrap();
        write_string(&file, "new").await.unwrap();
        let read = read_string(&file).await.unwrap();

        assert_eq!(read, "new");
    }

    #[tokio::test]
    async fn read_string_fails_on_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nonexistent.txt");

        let result = read_string(&file).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_bytes_fails_on_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nonexistent.bin");

        let result = read_bytes(&file).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn delete_removes_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        write_string(&file, "content").await.unwrap();

        delete(&file).await.unwrap();

        assert!(!file.exists());
    }

    #[tokio::test]
    async fn delete_nonexistent_file_succeeds() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nonexistent.txt");

        let result = delete(&file).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_entries_returns_all_files_and_dirs() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "").unwrap();
        fs::write(dir.path().join("b.txt"), "").unwrap();
        std_fs::create_dir(dir.path().join("subdir")).unwrap();

        let entries = list_entries(dir.path()).await.unwrap();

        assert_eq!(entries.len(), 3);
        let names: Vec<_> = entries
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();
        assert!(names.contains(&"a.txt".to_string()));
        assert!(names.contains(&"b.txt".to_string()));
        assert!(names.contains(&"subdir".to_string()));
    }

    #[tokio::test]
    async fn list_entries_nonexistent_dir_returns_empty() {
        let dir = TempDir::new().unwrap();
        let nonexistent = dir.path().join("nonexistent");

        let entries = list_entries(&nonexistent).await.unwrap();

        assert_eq!(entries.len(), 0);
    }

    #[tokio::test]
    async fn list_entries_empty_dir_returns_empty() {
        let dir = TempDir::new().unwrap();

        let entries = list_entries(dir.path()).await.unwrap();

        assert_eq!(entries.len(), 0);
    }

    #[tokio::test]
    async fn write_fails_when_parent_dir_missing() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nonexistent_parent").join("test.txt");

        let result = write_string(&file, "content").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn io_errors_convert_to_error_type() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");

        let result = read_string(&file).await;
        assert!(matches!(result, Err(Error::Io(_))));
    }

    #[tokio::test]
    async fn tail_from_start_returns_complete_lines_and_defers_partial() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("t.log");
        fs::write(&file, "a\nbb\nccc").unwrap();

        let t = tail(&file, 0, 1024, false).await.unwrap();

        assert_eq!(t.lines, vec!["a", "bb"]);
        assert_eq!(t.start, 0);
        assert_eq!(t.end, 5);
    }

    #[tokio::test]
    async fn tail_mid_file_drops_partial_first_line() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("t.log");
        fs::write(&file, "aaa\nbbb\nccc\n").unwrap();

        // Start inside the first line; the partial "aa" is dropped, window begins at byte 4.
        let t = tail(&file, 1, 1024, true).await.unwrap();

        assert_eq!(t.lines, vec!["bbb", "ccc"]);
        assert_eq!(t.start, 4);
        assert_eq!(t.end, 12);
    }

    #[tokio::test]
    async fn tail_continues_from_prior_end() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("t.log");
        fs::write(&file, "one\ntwo\n").unwrap();
        let first = tail(&file, 0, 1024, false).await.unwrap();

        fs::write(&file, "one\ntwo\nthree\n").unwrap();
        let next = tail(&file, first.end, 1024, false).await.unwrap();

        assert_eq!(next.lines, vec!["three"]);
        assert_eq!(next.start, first.end);
    }

    #[tokio::test]
    async fn tail_no_newline_in_range_is_empty() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("t.log");
        fs::write(&file, "no newline here").unwrap();

        let t = tail(&file, 0, 1024, false).await.unwrap();

        assert!(t.lines.is_empty());
    }

    #[tokio::test]
    async fn tail_absent_file_is_empty() {
        let dir = TempDir::new().unwrap();
        let t = tail(dir.path().join("missing.log"), 0, 1024, false).await.unwrap();

        assert!(t.lines.is_empty());
        assert_eq!(t.end, 0);
    }
}
