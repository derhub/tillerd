//! The generic message contracts (Meyer's command-query separation). A message is
//! a `Command` or a `Query`: a `Command` mutates domain state and returns nothing;
//! a `Query` reads domain state and returns its `Out`. Each gets only the context
//! -- the transaction, if any, is the message's concern, not the bus's. Dispatch is
//! static (no `Box<dyn _>`); a message is called with its concrete type in hand.
//!
//! External I/O (process, fs, network streams) is not a message kind -- it lives off
//! the bus as the runtime port plus its sink, addressed by primitive id.
//!
//! # Borrowed-event sink convention
//!
//! Per-domain event streams follow a parallel, complementary contract: each domain
//! in `events/` defines a `borrowed enum Event<'a>` (plain built-in payload types
//! only) and a `trait Sink { fn emit(&self, id: &str, event: &Event<'_>) }`.
//! `Broadcast<dyn Sink>` (in `shared::bus`) fans the call out to every subscriber
//! synchronously, forwarding the borrowed payload zero-copy. Subscribers may
//! borrow, copy, or clone -- the choice is theirs alone.

use crate::shared::Result;

/// A mutation. Loads, applies an entity rule, persists -- returning no data.
pub trait Command<Cx>: Send + 'static {
    fn handle(&self, cx: &Cx) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// A read. Returns its `Out` and performs no write.
pub trait Query<Cx>: Send + 'static {
    type Out: Send;
    fn handle(&self, cx: &Cx) -> impl std::future::Future<Output = Result<Self::Out>> + Send;
}
