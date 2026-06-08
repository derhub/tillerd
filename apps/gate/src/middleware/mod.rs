//! The middleware trait, the single-use continuation [`Next`], and the
//! [`seq`]/[`par`] combinators.

pub mod auth;
pub mod fanout;
pub mod normalize;
pub mod observe;
pub mod passthrough;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::{Ctx, Flow, Outbound};

/// One layer of the gate onion.
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Process one inbound, optionally advancing the chain via `next.run`.
    async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow;
}

enum Terminal<'a> {
    Accept,
    Spy(Arc<Mutex<bool>>),
    Chain(Box<Next<'a>>),
}

/// The single-use continuation handed to a middleware: the remaining chain plus
/// a terminal action. Running it advances to the next layer or, once the chain
/// is exhausted, performs the terminal action.
pub struct Next<'a> {
    chain: &'a [Arc<dyn Middleware>],
    terminal: Terminal<'a>,
}

impl<'a> Next<'a> {
    /// A continuation that accepts immediately.
    pub fn noop() -> Self {
        Next {
            chain: &[],
            terminal: Terminal::Accept,
        }
    }

    /// A continuation that records whether it was run; for tests asserting that a
    /// middleware did (or did not) continue.
    pub fn spy() -> (Self, Arc<Mutex<bool>>) {
        let called = Arc::new(Mutex::new(false));
        (
            Next {
                chain: &[],
                terminal: Terminal::Spy(called.clone()),
            },
            called,
        )
    }

    /// Advance the chain by one layer, or run the terminal once exhausted.
    pub async fn run(self, ctx: Ctx) -> Flow {
        match self.chain.split_first() {
            Some((head, rest)) => {
                let next = Next {
                    chain: rest,
                    terminal: self.terminal,
                };
                head.handle(ctx, next).await
            }
            None => match self.terminal {
                Terminal::Accept => Ok(Outbound::Accepted),
                Terminal::Spy(called) => {
                    *called.lock().expect("spy flag mutex poisoned") = true;
                    Ok(Outbound::Accepted)
                }
                // Boxed to break the otherwise-infinite future size of the
                // recursive delegation into the wrapped continuation.
                Terminal::Chain(inner) => Box::pin(inner.run(ctx)).await,
            },
        }
    }
}

struct Seq {
    items: Vec<Arc<dyn Middleware>>,
}

#[async_trait]
impl Middleware for Seq {
    async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
        run_seq(&self.items, ctx, next).await
    }
}

// A single lifetime unifies the borrowed item chain and the wrapped outer
// continuation so the inner onion can fall through to the outer one.
async fn run_seq<'a>(items: &'a [Arc<dyn Middleware>], ctx: Ctx, outer: Next<'a>) -> Flow {
    Next {
        chain: items,
        terminal: Terminal::Chain(Box::new(outer)),
    }
    .run(ctx)
    .await
}

/// Compose middlewares into a sequential onion. The first to return `Err` or a
/// terminal `Outbound` without calling its `next` short-circuits the rest.
pub fn seq(items: Vec<Arc<dyn Middleware>>) -> Arc<dyn Middleware> {
    Arc::new(Seq { items })
}

struct Par {
    items: Vec<Arc<dyn Middleware>>,
}

#[async_trait]
impl Middleware for Par {
    async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
        let mut handles = Vec::with_capacity(self.items.len());
        for item in &self.items {
            let item = item.clone();
            let branch_ctx = ctx.clone();
            handles.push(tokio::spawn(async move {
                let _ = item.handle(branch_ctx, Next::noop()).await;
            }));
        }
        for handle in handles {
            // A panicking branch surfaces as a JoinError, swallowed here so the
            // other branches and the continuation are unaffected.
            let _ = handle.await;
        }
        next.run(ctx).await
    }
}

/// Run middlewares as concurrent, isolated branches, join them all, then
/// continue. A panic in one branch never aborts the others or the continuation.
pub fn par(items: Vec<Arc<dyn Middleware>>) -> Arc<dyn Middleware> {
    Arc::new(Par { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kind, Reject, Token};
    use bytes::Bytes;
    use contracts::{CorrelationId, SessionId};
    use std::time::Duration;

    fn ctx() -> Ctx {
        Ctx {
            kind: Kind::Hook,
            session: SessionId("s".into()),
            correlation: CorrelationId("c".into()),
            token: Token::new("t"),
            body: Bytes::new(),
            event: None,
            record: Default::default(),
        }
    }

    struct Recorder {
        label: &'static str,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    #[async_trait]
    impl Middleware for Recorder {
        async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
            self.log.lock().unwrap().push(self.label);
            next.run(ctx).await
        }
    }

    struct Terminate;

    #[async_trait]
    impl Middleware for Terminate {
        async fn handle(&self, _ctx: Ctx, _next: Next<'_>) -> Flow {
            Ok(Outbound::Accepted)
        }
    }

    struct Rejector;

    #[async_trait]
    impl Middleware for Rejector {
        async fn handle(&self, _ctx: Ctx, _next: Next<'_>) -> Flow {
            Err(Reject::Unauthenticated)
        }
    }

    struct SetCorrelation(&'static str);

    #[async_trait]
    impl Middleware for SetCorrelation {
        async fn handle(&self, mut ctx: Ctx, next: Next<'_>) -> Flow {
            ctx.correlation = CorrelationId(self.0.into());
            next.run(ctx).await
        }
    }

    struct SeeCorrelation {
        seen: Arc<Mutex<Option<String>>>,
    }

    #[async_trait]
    impl Middleware for SeeCorrelation {
        async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
            *self.seen.lock().unwrap() = Some(ctx.correlation.0.clone());
            next.run(ctx).await
        }
    }

    #[tokio::test]
    async fn seq_runs_middlewares_in_registration_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let chain = seq(vec![
            Arc::new(Recorder {
                label: "first",
                log: log.clone(),
            }),
            Arc::new(Recorder {
                label: "second",
                log: log.clone(),
            }),
        ]);

        let out = chain.handle(ctx(), Next::noop()).await.unwrap();

        assert_eq!(out, Outbound::Accepted);
        assert_eq!(*log.lock().unwrap(), vec!["first", "second"]);
    }

    #[tokio::test]
    async fn seq_short_circuits_on_first_reject() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let chain = seq(vec![
            Arc::new(Rejector),
            Arc::new(Recorder {
                label: "after",
                log: log.clone(),
            }),
        ]);

        let err = chain.handle(ctx(), Next::noop()).await.unwrap_err();

        assert_eq!(err, Reject::Unauthenticated);
        assert!(log.lock().unwrap().is_empty(), "a reject stops the chain");
    }

    #[tokio::test]
    async fn seq_short_circuits_on_terminal_outbound() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let chain = seq(vec![
            Arc::new(Terminate),
            Arc::new(Recorder {
                label: "after",
                log: log.clone(),
            }),
        ]);

        let out = chain.handle(ctx(), Next::noop()).await.unwrap();

        assert_eq!(out, Outbound::Accepted);
        assert!(
            log.lock().unwrap().is_empty(),
            "a terminal outbound stops the chain"
        );
    }

    #[tokio::test]
    async fn seq_propagates_ctx_mutations_to_later_layers() {
        let seen = Arc::new(Mutex::new(None));
        let chain = seq(vec![
            Arc::new(SetCorrelation("mutated")),
            Arc::new(SeeCorrelation { seen: seen.clone() }),
        ]);

        chain.handle(ctx(), Next::noop()).await.unwrap();

        assert_eq!(seen.lock().unwrap().as_deref(), Some("mutated"));
    }

    struct GatedBranch {
        gate: Arc<tokio::sync::Barrier>,
        ran: Arc<Mutex<u32>>,
    }

    #[async_trait]
    impl Middleware for GatedBranch {
        async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
            self.gate.wait().await;
            *self.ran.lock().unwrap() += 1;
            next.run(ctx).await
        }
    }

    #[tokio::test]
    async fn par_runs_branches_concurrently() {
        let gate = Arc::new(tokio::sync::Barrier::new(2));
        let ran = Arc::new(Mutex::new(0));
        let chain = par(vec![
            Arc::new(GatedBranch {
                gate: gate.clone(),
                ran: ran.clone(),
            }),
            Arc::new(GatedBranch {
                gate: gate.clone(),
                ran: ran.clone(),
            }),
        ]);

        // Run sequentially and the first branch would block on the barrier
        // forever; completing within the timeout proves concurrency.
        let out = tokio::time::timeout(Duration::from_secs(2), chain.handle(ctx(), Next::noop()))
            .await
            .expect("branches must run concurrently")
            .unwrap();

        assert_eq!(out, Outbound::Accepted);
        assert_eq!(*ran.lock().unwrap(), 2);
    }

    struct SlowBranch {
        done: Arc<Mutex<bool>>,
    }

    #[async_trait]
    impl Middleware for SlowBranch {
        async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
            tokio::time::sleep(Duration::from_millis(20)).await;
            *self.done.lock().unwrap() = true;
            next.run(ctx).await
        }
    }

    #[tokio::test]
    async fn par_joins_all_branches_before_continuing() {
        let done = Arc::new(Mutex::new(false));
        let chain = par(vec![Arc::new(SlowBranch { done: done.clone() })]);

        chain.handle(ctx(), Next::noop()).await.unwrap();

        assert!(
            *done.lock().unwrap(),
            "par returns only after every branch has completed"
        );
    }

    struct Panicker;

    #[async_trait]
    impl Middleware for Panicker {
        async fn handle(&self, _ctx: Ctx, _next: Next<'_>) -> Flow {
            panic!("branch boom");
        }
    }

    struct CountBranch {
        ran: Arc<Mutex<u32>>,
    }

    #[async_trait]
    impl Middleware for CountBranch {
        async fn handle(&self, ctx: Ctx, next: Next<'_>) -> Flow {
            *self.ran.lock().unwrap() += 1;
            next.run(ctx).await
        }
    }

    #[tokio::test]
    async fn par_isolates_a_panicking_branch() {
        let ran = Arc::new(Mutex::new(0));
        let chain = par(vec![
            Arc::new(Panicker),
            Arc::new(CountBranch { ran: ran.clone() }),
        ]);

        let out = chain.handle(ctx(), Next::noop()).await.unwrap();

        assert_eq!(*ran.lock().unwrap(), 1, "the healthy branch still ran");
        assert_eq!(
            out,
            Outbound::Accepted,
            "par propagates the continuation's flow even when a branch panics"
        );
    }

    #[tokio::test]
    async fn noop_continuation_accepts() {
        let out = Next::noop().run(ctx()).await.unwrap();
        assert_eq!(out, Outbound::Accepted);
    }

    #[tokio::test]
    async fn spy_continuation_records_when_run() {
        let (next, called) = Next::spy();
        assert!(!*called.lock().unwrap());

        next.run(ctx()).await.unwrap();

        assert!(*called.lock().unwrap(), "the spy records that it was run");
    }

    #[tokio::test]
    async fn spy_stays_unset_when_middleware_short_circuits() {
        let (next, called) = Next::spy();

        let out = Terminate.handle(ctx(), next).await.unwrap();

        assert_eq!(out, Outbound::Accepted);
        assert!(
            !*called.lock().unwrap(),
            "a short-circuiting middleware never runs next"
        );
    }
}
