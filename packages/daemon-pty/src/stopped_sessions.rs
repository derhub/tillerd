//! Durable stopped-session store: an authoritative, never-evicted set of
//! intentionally-stopped session ids, persisted atomically (tmp + fsync + rename)
//! so a stopped session stays stopped across daemon restarts.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub struct StoppedSessionsStore {
    file_path: PathBuf,
    set: BTreeSet<String>,
}

impl StoppedSessionsStore {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            set: BTreeSet::new(),
        }
    }

    pub fn load(&mut self) {
        self.set = match fs::read_to_string(&self.file_path) {
            Ok(content) => content
                .split('\n')
                .filter(|l| !l.trim().is_empty())
                .map(|s| s.to_string())
                .collect(),
            Err(_) => BTreeSet::new(),
        };
    }

    pub fn add(&mut self, session_id: &str) {
        if self.set.contains(session_id) {
            return;
        }
        self.set.insert(session_id.to_string());
        self.persist();
    }

    pub fn has(&self, session_id: &str) -> bool {
        self.set.contains(session_id)
    }

    fn persist(&self) {
        if let Some(parent) = self.file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let tmp = self.file_path.with_extension("txt.tmp");
        let body = self.set.iter().cloned().collect::<Vec<_>>().join("\n") + "\n";
        if let Ok(mut f) = fs::File::create(&tmp) {
            if f.write_all(body.as_bytes()).is_ok() {
                let _ = f.sync_all();
                let _ = fs::rename(&tmp, &self.file_path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_persists_and_survives_reload() {
        let path = std::env::temp_dir().join(format!("athing-stopped-{}.txt", std::process::id()));
        let _ = fs::remove_file(&path);
        let mut store = StoppedSessionsStore::new(path.clone());
        store.load();
        assert!(!store.has("s1"));
        store.add("s1");
        store.add("s2");

        let mut reloaded = StoppedSessionsStore::new(path.clone());
        reloaded.load();
        assert!(reloaded.has("s1"));
        assert!(reloaded.has("s2"));
        assert!(!reloaded.has("s3"));
        let _ = fs::remove_file(&path);
    }
}
