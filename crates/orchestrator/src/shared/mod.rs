//! Reusable building blocks shared across layers -- no entity knowledge. Holds the
//! error registry (and, as the refactor lands, `fs`/`kv`/`pagination`/`message`/`bus`/
//! `datetime`). Not a storage abstraction.

pub mod bus;
pub mod datetime;
pub mod errors;
pub mod fs;
pub mod kv;
pub mod message;
pub mod pagination;

pub use bus::Bus;
pub use errors::{Error, Result};
pub use message::{Command, Query};
pub use pagination::{Listing, Page};
