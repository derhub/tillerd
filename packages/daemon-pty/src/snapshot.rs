//! Snapshot: one record per live session. Upgrade handoff mechanism.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotRecord {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub token: String,
    pub pid: u32,
    pub cols: u16,
    pub rows: u16,
    pub cwd: String,
    /// Position in the successor's inherited-fd table (the PTY master fd lands here).
    #[serde(rename = "fdIndex")]
    pub fd_index: i32,
    #[serde(rename = "replayBuffer")]
    pub replay_buffer: String,
}

impl SnapshotRecord {
    pub fn encode_replay(bytes: &[u8]) -> String {
        STANDARD.encode(bytes)
    }

    pub fn decode_replay(&self) -> Vec<u8> {
        STANDARD.decode(&self.replay_buffer).unwrap_or_default()
    }
}

pub fn write_snapshot(path: &Path, records: &[SnapshotRecord]) -> std::io::Result<()> {
    let mut body = String::new();
    for r in records {
        body.push_str(&serde_json::to_string(r).expect("snapshot record serialize"));
        body.push('\n');
    }
    let tmp = path.with_extension("ndjson.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn read_snapshot(path: &Path) -> std::io::Result<Vec<SnapshotRecord>> {
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ndjson_and_base64() {
        let dir = std::env::temp_dir().join(format!("athing-snap-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("snap.ndjson");
        let recs = vec![
            SnapshotRecord {
                session_id: "s1".into(),
                token: "t1".into(),
                pid: 111,
                cols: 80,
                rows: 24,
                cwd: "/tmp".into(),
                fd_index: 4,
                replay_buffer: SnapshotRecord::encode_replay(b"hello\x00\xffworld"),
            },
            SnapshotRecord {
                session_id: "s2".into(),
                token: "t2".into(),
                pid: 222,
                cols: 100,
                rows: 30,
                cwd: "/".into(),
                fd_index: 5,
                replay_buffer: SnapshotRecord::encode_replay(b""),
            },
        ];
        write_snapshot(&path, &recs).unwrap();
        let read = read_snapshot(&path).unwrap();
        assert_eq!(read, recs);
        assert_eq!(read[0].decode_replay(), b"hello\x00\xffworld");
        assert_eq!(read[1].decode_replay(), b"");
        let _ = fs::remove_dir_all(&dir);
    }
}
