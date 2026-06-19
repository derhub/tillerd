//! Concrete storage backends. Each is a self-contained async-capable struct over
//! `entities`; the `store` layer composes them behind the `Backend` enum.

pub mod fs;
pub mod memory;
pub mod schema;
pub mod sqlite;

pub use fs::{DomainStore, FsBackend};
pub use memory::InMemoryStore;
pub use sqlite::SqliteBackend;
