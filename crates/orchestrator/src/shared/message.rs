//! The generic message contracts (Meyer's command-query separation, plus I/O). A
//! message is a `Command`, a `Query`, or an `Io`: a `Command` mutates domain state
//! and returns nothing; a `Query` reads domain state and returns its `Out`; an `Io`
//! performs external I/O (filesystem, process, network) and returns its `Out`. Each
//! gets only the context -- the transaction, if any, is the message's concern, not
//! the bus's. Dispatch is static (no `Box<dyn _>`); a message is called with its
//! concrete type in hand.

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

/// An external I/O effect. Touches the world outside the domain store -- filesystem,
/// process, network -- and returns its `Out`. Kept distinct from `Query` (pure
/// domain read) so I/O-bearing messages are explicit at the call site.
pub trait Io<Cx>: Send + 'static {
    type Out: Send;
    fn handle(&self, cx: &Cx) -> impl std::future::Future<Output = Result<Self::Out>> + Send;
}
