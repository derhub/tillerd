//! mtime-revalidated read-through cache of parsed entity files.

use serde::de::DeserializeOwned;
use std::time::SystemTime;

use super::*;

/// Cache of parsed entity files keyed by path. Each entry holds the file's
/// modification time at parse and the parsed JSON value: a read reuses the value
/// when the on-disk mtime is unchanged and re-reads otherwise.
///
/// Single-writer model — the backend invalidates an entry on its own writes, and
/// the mtime compare is the backstop for any out-of-band edit.
#[derive(Default)]
pub(crate) struct FileCache {
    entries: RwLock<HashMap<PathBuf, (SystemTime, serde_json::Value)>>,
}

impl FileCache {
    /// Read and deserialize `path`, reusing the cached parse when its mtime is unchanged.
    pub(crate) fn read<T: DeserializeOwned>(&self, path: &Path) -> Result<T> {
        let mtime = file_mtime(path)?;
        {
            let entries = self.entries.read().unwrap();
            if let Some((cached_mtime, value)) = entries.get(path) {
                if *cached_mtime == mtime {
                    return from_value_at(path, value.clone());
                }
            }
        }
        let content = fs::read_to_string(path).map_err(|e| at(path, e))?;
        let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| at(path, e))?;
        let parsed = from_value_at(path, value.clone())?;
        self.entries
            .write()
            .unwrap()
            .insert(path.to_path_buf(), (mtime, value));
        Ok(parsed)
    }

    /// Drop the cached parse for `path`; the next read re-reads from disk.
    pub(crate) fn invalidate(&self, path: &Path) {
        self.entries.write().unwrap().remove(path);
    }
}

fn at<E: std::fmt::Display>(path: &Path, e: E) -> OrchestratorError {
    OrchestratorError::Persistence(format!("{path:?}: {e}"))
}

fn file_mtime(path: &Path) -> Result<SystemTime> {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map_err(|e| at(path, e))
}

fn from_value_at<T: DeserializeOwned>(path: &Path, value: serde_json::Value) -> Result<T> {
    serde_json::from_value(value).map_err(|e| at(path, e))
}
