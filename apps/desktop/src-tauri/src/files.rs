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
    let mut file = tokio::fs::File::open(path).await.map_err(|e| e.to_string())?;
    file.seek(SeekFrom::Start(offset)).await.map_err(|e| e.to_string())?;

    let mut buf = vec![0u8; length as usize];
    let mut filled = 0usize;
    while filled < buf.len() {
        let n = file.read(&mut buf[filled..]).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        filled += n;
    }
    buf.truncate(filled);
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[tokio::test]
    async fn file_size_absent_returns_none() {
        assert_eq!(file_size("/nonexistent/path/zzz.dat".to_string()).await, None);
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
        let bytes = read_bytes(tmp.path().to_str().unwrap(), 0, 11).await.unwrap();
        assert_eq!(bytes, b"hello world");
    }

    #[tokio::test]
    async fn read_bytes_reads_from_offset() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"hello world").unwrap();
        let bytes = read_bytes(tmp.path().to_str().unwrap(), 6, 5).await.unwrap();
        assert_eq!(bytes, b"world");
    }

    #[tokio::test]
    async fn read_bytes_short_at_eof() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"hi").unwrap();
        let bytes = read_bytes(tmp.path().to_str().unwrap(), 0, 100).await.unwrap();
        assert_eq!(bytes, b"hi");
    }
}
