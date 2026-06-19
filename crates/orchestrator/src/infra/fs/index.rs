//! Boot-scan of the file tree into an id->path index, archive-aware listing.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::error::Result;

use super::atomic_io::{persist, read_json};
use super::{ProjectFile, SessionFile, WorkspaceFile};

/// Scan a directory, returning immediate child entries (non-recursive).
/// Skips entries starting with `.` (e.g., `.archive`).
pub(crate) fn list_live_dirs(parent: &Path) -> Result<Vec<PathBuf>> {
    if !parent.exists() {
        return Ok(vec![]);
    }
    let mut dirs = Vec::new();
    for entry in fs::read_dir(parent).map_err(persist)? {
        let entry = entry.map_err(persist)?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    Ok(dirs)
}

/// Boot-scan the tree rooted at `root`, building the id→path index.
///
/// Scans live workspace dirs → project dirs → session dirs.
/// Archived subtrees (under `.archive/`) are also indexed so get-by-id works
/// for archived entities (needed for hard_delete).
pub(crate) fn build_index(root: &Path) -> Result<HashMap<String, PathBuf>> {
    let mut index = HashMap::new();
    let ws_root = root.join("workspaces");
    if !ws_root.exists() {
        return Ok(index);
    }

    // Walk all workspace dirs (live + archived)
    let ws_dirs = all_dirs_including_archive(&ws_root)?;
    for ws_dir in ws_dirs {
        let ws_file = ws_dir.join("workspace.json");
        if ws_file.exists() {
            if let Ok(wf) = read_json::<WorkspaceFile>(&ws_file) {
                index.insert(wf.id.clone(), ws_dir.clone());

                // Walk project dirs (live + archived) inside this workspace
                let proj_root = ws_dir.join("projects");
                let proj_dirs = all_dirs_including_archive(&proj_root)?;
                for proj_dir in proj_dirs {
                    let pf_path = proj_dir.join("project.json");
                    if pf_path.exists() {
                        match read_json::<ProjectFile>(&pf_path) {
                            Ok(pf) => {
                                index.insert(pf.id.clone(), proj_dir.clone());

                                // Walk session dirs (live + archived)
                                let sess_root = proj_dir.join("sessions");
                                let sess_dirs = all_dirs_including_archive(&sess_root)?;
                                for sess_dir in sess_dirs {
                                    let sf_path = sess_dir.join("session.json");
                                    if sf_path.exists() {
                                        if let Ok(sf) = read_json::<SessionFile>(&sf_path) {
                                            index.insert(sf.id.clone(), sess_dir.clone());
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("warn: skipping malformed {}: {}", pf_path.display(), e);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(index)
}

/// Return all child dirs under `parent`, including those under `.archive/`.
pub(crate) fn all_dirs_including_archive(parent: &Path) -> Result<Vec<PathBuf>> {
    if !parent.exists() {
        return Ok(vec![]);
    }
    let mut result = Vec::new();
    for entry in fs::read_dir(parent).map_err(persist)? {
        let entry = entry.map_err(persist)?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == ".archive" {
            // Also recurse into .archive children
            for arch_entry in fs::read_dir(&path).map_err(persist)? {
                let arch_entry = arch_entry.map_err(persist)?;
                let ap = arch_entry.path();
                if ap.is_dir() {
                    result.push(ap);
                }
            }
        } else {
            result.push(path);
        }
    }
    Ok(result)
}

/// Determine whether a path is inside a `.archive` directory.
pub(crate) fn is_archived(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".archive")
}
