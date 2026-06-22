//! The generic CQS contracts (Meyer's command-query separation). A `Command`
//! mutates and returns nothing; a `Query` reads and returns its `Out`. Both get
//! only the context — the transaction, if any, is the command's concern, not the
//! bus's. Dispatch is static (no `Box<dyn _>`); a command is called with its
//! concrete type in hand.

use crate::shared::Result;

/// A mutation. Loads, applies an entity rule, persists — returning no data.
pub trait Command<Cx>: Send + 'static {
    fn handle(&self, cx: &Cx) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// A read. Returns its `Out` and performs no write.
pub trait Query<Cx>: Send + 'static {
    type Out: Send;
    fn handle(&self, cx: &Cx) -> impl std::future::Future<Output = Result<Self::Out>> + Send;
}
