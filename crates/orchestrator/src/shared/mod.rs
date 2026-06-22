//! Reusable building blocks shared across layers — no entity knowledge. Holds the
//! error registry (and, as the refactor lands, `fs`/`kv`/`pagination`/`cqs`/`bus`/
//! `datetime`). Not a storage abstraction.

pub mod bus;
pub mod cqs;
pub mod datetime;
pub mod errors;
pub mod fs;
pub mod kv;
pub mod pagination;

pub use bus::Bus;
pub use cqs::{Command, Query};
pub use errors::{Error, Result};
pub use pagination::{Listing, Page};
