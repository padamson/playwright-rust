//! One home for the "list of handlers plus list of one-shot waiters" pattern
//! that every page- and context-level event repeats.
//!
//! Before this existed each event carried two `Arc<Mutex<Vec<_>>>` fields, two
//! type aliases, a hand-rolled emptiness check to drive subscription, and its
//! own dispatch loop. The loops had drifted apart: 17 of 18 ran handlers before
//! waking a waiter and one did the reverse, and every one of the 19 waiter
//! drains used `Vec::pop` — last-in-first-out — while seven of them carried a
//! comment promising FIFO. Collapsing them here is mostly about having one
//! answer to those questions rather than about the line count.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use crate::error::Result;

/// A boxed future returned by an event handler.
pub(crate) type HandlerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// An async event handler. Cloned out of the registry before dispatch, so a
/// handler that registers another handler cannot deadlock on the lock.
pub(crate) type Handler<T> = Arc<dyn Fn(T) -> HandlerFuture + Send + Sync>;

/// Handlers and one-shot waiters for a single event.
pub(crate) struct EventRegistry<T> {
    /// Event name, used only for the handler-error log line.
    name: &'static str,
    handlers: Mutex<Vec<Handler<T>>>,
    waiters: Mutex<VecDeque<tokio::sync::oneshot::Sender<T>>>,
}

impl<T> EventRegistry<T> {
    pub(crate) fn new(name: &'static str) -> Arc<Self> {
        Arc::new(Self {
            name,
            handlers: Mutex::new(Vec::new()),
            waiters: Mutex::new(VecDeque::new()),
        })
    }

    /// Whether nobody is listening.
    ///
    /// The server only pushes an event once we subscribe, and callers use this
    /// to decide whether registering makes them the first listener. It has to
    /// consider waiters as well as handlers: an `expect_*` call with no
    /// handlers registered still needs the subscription.
    pub(crate) fn is_idle(&self) -> bool {
        self.handlers.lock().unwrap().is_empty() && self.waiters.lock().unwrap().is_empty()
    }

    pub(crate) fn add_handler(&self, handler: Handler<T>) {
        self.handlers.lock().unwrap().push(handler);
    }

    /// Enqueue a one-shot waiter and hand back the receiver.
    pub(crate) fn wait(&self) -> tokio::sync::oneshot::Receiver<T> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.waiters.lock().unwrap().push_back(tx);
        rx
    }

    /// The protocol event name, so callers driving `update_subscription` do not
    /// restate the string at every registration site.
    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    #[cfg(test)]
    pub(crate) fn handler_count(&self) -> usize {
        self.handlers.lock().unwrap().len()
    }

    #[cfg(test)]
    pub(crate) fn waiter_count(&self) -> usize {
        self.waiters.lock().unwrap().len()
    }
}

impl<T: Clone> EventRegistry<T> {
    /// Deliver one event: every handler, then the longest-waiting waiter.
    ///
    /// Handlers run first because 17 of the 18 hand-written dispatchers did,
    /// and because a waiter's continuation may observe state a handler set —
    /// an `expect_*` that resumes after the handlers cannot race their side
    /// effects. The cost, accepted deliberately: a slow handler delays the
    /// waiter's wake-up, and a hung one blocks it. That coupling was already
    /// the shipped behavior for those 17 events.
    ///
    /// Waiters are served oldest-first *per dispatch*, which is what their doc
    /// comments always claimed even while the code popped the newest. This is
    /// not a cross-event ordering guarantee: callers dispatch each event from
    /// its own spawned task, so two in-flight dispatches may reach the queue
    /// in either order regardless of which event the server emitted first.
    ///
    /// A handler that returns `Err` is logged and does not stop the others: one
    /// bad observer must not swallow an event for everyone else.
    pub(crate) async fn dispatch(&self, value: T) {
        let handlers = self.handlers.lock().unwrap().clone();
        for handler in handlers {
            if let Err(e) = handler(value.clone()).await {
                tracing::warn!("{} handler error: {}", self.name, e);
            }
        }
        self.notify_one_waiter(value);
    }

    /// Hand the value to the oldest waiter whose receiver is still alive.
    ///
    /// Skipping dead receivers matters: an `expect_*` that timed out or was
    /// cancelled leaves a sender whose receiver is gone, and the old
    /// pop-then-send would consume the event on it and starve the waiter behind
    /// it.
    fn notify_one_waiter(&self, value: T) {
        let mut waiters = self.waiters.lock().unwrap();
        while let Some(tx) = waiters.pop_front() {
            match tx.send(value.clone()) {
                Ok(()) => return,
                Err(_) => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn counting_handler(hits: Arc<AtomicUsize>) -> Handler<u32> {
        Arc::new(move |_| {
            let hits = hits.clone();
            Box::pin(async move {
                hits.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        })
    }

    fn failing_handler() -> Handler<u32> {
        Arc::new(|_| Box::pin(async { Err(crate::error::Error::ProtocolError("nope".into())) }))
    }

    #[test]
    fn the_name_survives_construction() {
        // Load-bearing, not cosmetic: subscribe_if_idle passes this to
        // update_subscription, so a corrupted name subscribes to the wrong
        // server event and the real one never arrives.
        assert_eq!(EventRegistry::<u32>::new("console").name(), "console");
    }

    #[tokio::test]
    async fn idle_until_someone_registers() {
        let reg = EventRegistry::<u32>::new("test");
        assert!(reg.is_idle());

        let _rx = reg.wait();
        assert!(
            !reg.is_idle(),
            "a waiter alone must still drive subscription"
        );

        let reg2 = EventRegistry::<u32>::new("test");
        reg2.add_handler(counting_handler(Arc::new(AtomicUsize::new(0))));
        assert!(!reg2.is_idle());
    }

    #[tokio::test]
    async fn every_handler_sees_every_event() {
        let hits = Arc::new(AtomicUsize::new(0));
        let reg = EventRegistry::<u32>::new("test");
        reg.add_handler(counting_handler(hits.clone()));
        reg.add_handler(counting_handler(hits.clone()));

        reg.dispatch(1).await;
        reg.dispatch(2).await;

        assert_eq!(hits.load(Ordering::SeqCst), 4);
        assert_eq!(
            reg.handler_count(),
            2,
            "handlers are retained across events"
        );
    }

    #[tokio::test]
    async fn a_failing_handler_does_not_starve_the_others() {
        let hits = Arc::new(AtomicUsize::new(0));
        let reg = EventRegistry::<u32>::new("test");
        reg.add_handler(failing_handler());
        reg.add_handler(counting_handler(hits.clone()));

        reg.dispatch(7).await;

        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn waiters_are_served_oldest_first() {
        let reg = EventRegistry::<u32>::new("test");
        let first = reg.wait();
        let second = reg.wait();

        reg.dispatch(10).await;
        reg.dispatch(20).await;

        assert_eq!(
            first.await.unwrap(),
            10,
            "the earliest waiter gets the earliest event"
        );
        assert_eq!(second.await.unwrap(), 20);
    }

    #[tokio::test]
    async fn a_waiter_is_one_shot() {
        let reg = EventRegistry::<u32>::new("test");
        let rx = reg.wait();

        reg.dispatch(1).await;
        assert_eq!(rx.await.unwrap(), 1);
        assert_eq!(reg.waiter_count(), 0);

        // Nothing left to notify, and dispatching again must not panic.
        reg.dispatch(2).await;
    }

    #[tokio::test]
    async fn an_abandoned_waiter_does_not_swallow_the_event() {
        let reg = EventRegistry::<u32>::new("test");
        let abandoned = reg.wait();
        let live = reg.wait();
        drop(abandoned);

        reg.dispatch(42).await;

        assert_eq!(
            live.await.unwrap(),
            42,
            "a cancelled expect_* must not consume the event for the waiter behind it"
        );
    }

    #[tokio::test]
    async fn handlers_run_before_the_waiter_is_woken() {
        let order = Arc::new(Mutex::new(Vec::new()));
        let reg = EventRegistry::<u32>::new("test");
        let recorded = order.clone();
        reg.add_handler(Arc::new(move |_| {
            let recorded = recorded.clone();
            Box::pin(async move {
                recorded.lock().unwrap().push("handler");
                Ok(())
            })
        }));
        let rx = reg.wait();

        reg.dispatch(1).await;
        order.lock().unwrap().push("dispatch returned");
        let _ = rx.await;

        assert_eq!(*order.lock().unwrap(), vec!["handler", "dispatch returned"]);
    }

    #[tokio::test]
    async fn dispatch_with_no_listeners_is_a_no_op() {
        let reg = EventRegistry::<u32>::new("test");
        reg.dispatch(1).await;
        assert!(reg.is_idle());
    }
}
