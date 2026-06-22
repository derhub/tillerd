use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::shared::Result;

/// Options for a `put` operation.
#[derive(Clone, Debug, Default)]
pub struct PutOptions {
    /// Optional TTL from now. `None` means the entry never expires.
    pub ttl: Option<Duration>,
}

impl PutOptions {
    /// No TTL — entry lives until explicitly deleted or overwritten.
    pub fn none() -> Self {
        PutOptions { ttl: None }
    }

    /// Entry expires after `ttl` from now.
    pub fn with_ttl(ttl: Duration) -> Self {
        PutOptions { ttl: Some(ttl) }
    }
}

/// Schemaless async key-value store. Keys are strings; values are raw bytes.
/// `get` returns `None` for an absent key or one whose TTL has elapsed.
pub trait Kv: Send + Sync + 'static {
    fn put(
        &self,
        key: &str,
        value: &[u8],
        opts: PutOptions,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    fn get(&self, key: &str) -> impl std::future::Future<Output = Result<Option<Vec<u8>>>> + Send;
}

// ── SqliteKv ─────────────────────────────────────────────────────────────────

/// Sqlite-backed key-value store. Requires a table created by the migration:
///
/// ```sql
/// CREATE TABLE IF NOT EXISTS kv (
///     key        TEXT    PRIMARY KEY NOT NULL,
///     value      BLOB    NOT NULL,
///     expires_at INTEGER             -- unix millis; NULL = no expiry
/// );
/// ```
pub struct SqliteKv {
    pool: sqlx::SqlitePool,
}

impl SqliteKv {
    /// Wrap an existing pool. The `kv` table must already exist (run migrations first).
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        SqliteKv { pool }
    }

    /// Create a `SqliteKv` backed by an in-memory pool with the schema applied.
    /// Useful in tests and bootstrapping without a migration runner.
    pub async fn in_memory() -> Result<Self> {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;

        // shared-cache in-memory so all connections see the same data
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.shared_cache(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS kv (
                key        TEXT    PRIMARY KEY NOT NULL,
                value      BLOB    NOT NULL,
                expires_at INTEGER
            )",
        )
        .execute(&pool)
        .await?;
        Ok(SqliteKv { pool })
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

impl Kv for SqliteKv {
    async fn put(&self, key: &str, value: &[u8], opts: PutOptions) -> Result<()> {
        let expires_at: Option<i64> = opts.ttl.map(|d| now_millis() + d.as_millis() as i64);
        sqlx::query(
            "INSERT INTO kv (key, value, expires_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, expires_at = excluded.expires_at",
        )
        .bind(key)
        .bind(value)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let now = now_millis();
        let row: Option<(Vec<u8>, Option<i64>)> =
            sqlx::query_as("SELECT value, expires_at FROM kv WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.and_then(|(value, expires_at)| {
            if let Some(exp) = expires_at {
                if now >= exp {
                    return None;
                }
            }
            Some(value)
        }))
    }
}

// ── MemoryKv ──────────────────────────────────────────────────────────────────

struct Entry {
    value: Vec<u8>,
    // unix millis; None = no expiry
    expires_at: Option<i64>,
}

/// In-memory key-value store. Suitable for tests and cases where durability is not required.
pub struct MemoryKv {
    store: Mutex<HashMap<String, Entry>>,
}

impl Default for MemoryKv {
    fn default() -> Self {
        MemoryKv {
            store: Mutex::new(HashMap::new()),
        }
    }
}

impl MemoryKv {
    pub fn new() -> Self {
        MemoryKv::default()
    }
}

impl Kv for MemoryKv {
    async fn put(&self, key: &str, value: &[u8], opts: PutOptions) -> Result<()> {
        let expires_at = opts.ttl.map(|d| now_millis() + d.as_millis() as i64);
        let mut store = self.store.lock().expect("MemoryKv lock poisoned");
        store.insert(
            key.to_owned(),
            Entry {
                value: value.to_vec(),
                expires_at,
            },
        );
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let now = now_millis();
        let store = self.store.lock().expect("MemoryKv lock poisoned");
        Ok(store.get(key).and_then(|entry| {
            if let Some(exp) = entry.expires_at {
                if now >= exp {
                    return None;
                }
            }
            Some(entry.value.clone())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── MemoryKv contract tests ───────────────────────────────────────────────

    // Scenario: round-trip by key
    #[tokio::test]
    async fn memory_kv_get_returns_stored_value() {
        let kv = MemoryKv::new();
        kv.put("hello", b"world", PutOptions::none()).await.unwrap();
        let got = kv.get("hello").await.unwrap();
        assert_eq!(got, Some(b"world".to_vec()));
    }

    #[tokio::test]
    async fn memory_kv_get_returns_none_for_absent_key() {
        let kv = MemoryKv::new();
        let got = kv.get("missing").await.unwrap();
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn memory_kv_overwrite_returns_latest_value() {
        let kv = MemoryKv::new();
        kv.put("k", b"v1", PutOptions::none()).await.unwrap();
        kv.put("k", b"v2", PutOptions::none()).await.unwrap();
        let got = kv.get("k").await.unwrap();
        assert_eq!(got, Some(b"v2".to_vec()));
    }

    // Scenario: TTL — expired entry returns None
    #[tokio::test]
    async fn memory_kv_expired_entry_returns_none() {
        let kv = MemoryKv::new();
        kv.put("k", b"v", PutOptions::with_ttl(Duration::from_millis(1)))
            .await
            .unwrap();
        // sleep long enough for the TTL to elapse
        tokio::time::sleep(Duration::from_millis(10)).await;
        let got = kv.get("k").await.unwrap();
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn memory_kv_live_ttl_entry_is_returned() {
        let kv = MemoryKv::new();
        kv.put("k", b"v", PutOptions::with_ttl(Duration::from_secs(60)))
            .await
            .unwrap();
        let got = kv.get("k").await.unwrap();
        assert_eq!(got, Some(b"v".to_vec()));
    }

    // ── SqliteKv contract tests ───────────────────────────────────────────────

    // Scenario: round-trip by key
    #[tokio::test]
    async fn sqlite_kv_get_returns_stored_value() {
        let kv = SqliteKv::in_memory().await.unwrap();
        kv.put("hello", b"world", PutOptions::none()).await.unwrap();
        let got = kv.get("hello").await.unwrap();
        assert_eq!(got, Some(b"world".to_vec()));
    }

    #[tokio::test]
    async fn sqlite_kv_get_returns_none_for_absent_key() {
        let kv = SqliteKv::in_memory().await.unwrap();
        let got = kv.get("missing").await.unwrap();
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn sqlite_kv_overwrite_returns_latest_value() {
        let kv = SqliteKv::in_memory().await.unwrap();
        kv.put("k", b"v1", PutOptions::none()).await.unwrap();
        kv.put("k", b"v2", PutOptions::none()).await.unwrap();
        let got = kv.get("k").await.unwrap();
        assert_eq!(got, Some(b"v2".to_vec()));
    }

    // Scenario: TTL — expired entry returns None
    #[tokio::test]
    async fn sqlite_kv_expired_entry_returns_none() {
        let kv = SqliteKv::in_memory().await.unwrap();
        kv.put("k", b"v", PutOptions::with_ttl(Duration::from_millis(1)))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let got = kv.get("k").await.unwrap();
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn sqlite_kv_live_ttl_entry_is_returned() {
        let kv = SqliteKv::in_memory().await.unwrap();
        kv.put("k", b"v", PutOptions::with_ttl(Duration::from_secs(60)))
            .await
            .unwrap();
        let got = kv.get("k").await.unwrap();
        assert_eq!(got, Some(b"v".to_vec()));
    }
}
