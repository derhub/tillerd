//! Atomic write-temp-rename file IO and JSON (de)serialization for the fs backend.

use std::{fs, io::Write, path::Path};

use serde::{Deserialize, Serialize};

use crate::error::{OrchestratorError, Result};

/// Map any displayable error (io, serde) to a persistence error.
pub(crate) fn persist<E: std::fmt::Display>(e: E) -> OrchestratorError {
    OrchestratorError::Persistence(e.to_string())
}

/// Recursively create `path`, owner-only (0700) on unix so the domain tree
/// under the user's data root is not readable by other local accounts.
pub(crate) fn create_dir_secure(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(path)
    }
}

/// Atomically write `content` to `path` via a `.tmp` sibling and rename.
/// The file is owner-only (0600) on unix — domain state may carry spec env.
pub(crate) fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    {
        #[cfg(unix)]
        let mut f = {
            use std::os::unix::fs::OpenOptionsExt as _;
            fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)
                .map_err(persist)?
        };
        #[cfg(not(unix))]
        let mut f = fs::File::create(&tmp).map_err(persist)?;
        f.write_all(content.as_bytes()).map_err(persist)?;
        f.flush().map_err(persist)?;
    }
    fs::rename(&tmp, path).map_err(persist)
}

/// Serialize `value` to pretty JSON with a trailing newline.
pub(crate) fn to_json<T: Serialize>(value: &T) -> Result<String> {
    let mut s = serde_json::to_string_pretty(value).map_err(persist)?;
    s.push('\n');
    Ok(s)
}

/// Read and deserialize a JSON file.
pub(crate) fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let content = fs::read_to_string(path)
        .map_err(|e| OrchestratorError::Persistence(format!("{path:?}: {e}")))?;
    serde_json::from_str(&content)
        .map_err(|e| OrchestratorError::Persistence(format!("{path:?}: {e}")))
}
