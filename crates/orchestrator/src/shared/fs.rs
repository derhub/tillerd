//! Atomic file read/write/list/delete utilities for user-config (settings, profiles,
//! themes, keybindings). Provides common operations with proper error handling.

use std::fs;
use std::path::{Path, PathBuf};

use super::errors::Error;

/// Read the entire contents of a file as a string.
pub async fn read_string(path: impl AsRef<Path>) -> Result<String, Error> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(Error::from)
}

/// Read the entire contents of a file as bytes.
pub async fn read_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>, Error> {
    let path = path.as_ref();
    fs::read(path).map_err(Error::from)
}

/// Write a string to a file, creating or truncating it. Parent directory must exist.
pub async fn write_string(path: impl AsRef<Path>, content: &str) -> Result<(), Error> {
    let path = path.as_ref();
    fs::write(path, content).map_err(Error::from)
}

/// Write bytes to a file, creating or truncating it. Parent directory must exist.
pub async fn write_bytes(path: impl AsRef<Path>, content: &[u8]) -> Result<(), Error> {
    let path = path.as_ref();
    fs::write(path, content).map_err(Error::from)
}

/// Delete a file if it exists. Returns `Ok(())` whether the file was deleted or not found.
pub async fn delete(path: impl AsRef<Path>) -> Result<(), Error> {
    let path = path.as_ref();
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::from(e)),
    }
}

/// List all entries in a directory. Returns `Ok(vec![])` if the directory does not exist.
pub async fn list_entries(path: impl AsRef<Path>) -> Result<Vec<PathBuf>, Error> {
    let path = path.as_ref();
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut result = Vec::new();
            for entry in entries {
                let entry = entry?;
                result.push(entry.path());
            }
            Ok(result)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(Error::from(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        std_fs::write(dir.path().join("a.txt"), "").unwrap();
        std_fs::write(dir.path().join("b.txt"), "").unwrap();
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
}
