//! Cross-pane named-signal queues with parent/child routing.
//!
//! Each pane gets its own FIFO queue of `Signal { name, data, ... }`
//! entries. `SignalSend` enqueues; `SignalWait` blocks until a matching
//! entry arrives (or a timeout fires). Parent/child relationships are
//! recorded when `Split` succeeds and torn down whenever a pane
//! disappears from the `PaneMap` (see `SignalRegistry::retain`).
//!
//! Concurrency model:
//! - All maps are guarded by `std::sync::Mutex` and held only for the
//!   duration of a single `lock()` / read / write tuple. Long async
//!   waits happen outside the lock via a per-pane `tokio::sync::Notify`.
//! - Enqueueing wakes every waiter on the target pane; each woken
//!   waiter then re-checks its queue for a matching name. Waiters that
//!   don't find a match go back to sleep on the same `Notify`.

use kiri_cli_proto::SignalEntry;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tokio::time::{timeout, Duration};

/// Hard upper bound on `signal wait` blocking time. Mirrors the CLI's
/// parser limit so clients that bypass clap (i.e. send raw JSON) still
/// can't hold a connection open indefinitely.
pub const MAX_SIGNAL_WAIT_SECS: u64 = 600;

#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    pub name: String,
    pub data: Option<serde_json::Value>,
    pub sender_pane_id: String,
    pub sent_at_ms: u64,
}

impl From<Signal> for SignalEntry {
    fn from(s: Signal) -> Self {
        SignalEntry {
            name: s.name,
            data: s.data,
            sender_pane_id: s.sender_pane_id,
            sent_at_ms: s.sent_at_ms,
        }
    }
}

#[derive(Default)]
struct Inner {
    /// `child_pane_id -> parent_pane_id`.
    parents: HashMap<String, String>,
    /// `pane_id -> FIFO queue of pending signals delivered to that pane`.
    queues: HashMap<String, VecDeque<Signal>>,
    /// `pane_id -> Notify used to wake `wait_for` calls`. Created lazily.
    notifiers: HashMap<String, Arc<Notify>>,
}

/// Registry of parent/child relationships and per-pane signal queues.
#[derive(Default)]
pub struct SignalRegistry {
    inner: Mutex<Inner>,
}

impl SignalRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `child` was spawned by `split`-ing `parent`. Overwrites
    /// any previous mapping (the most recent split wins — useful if pane
    /// ids ever get reused, though today they don't).
    pub fn register_parent(&self, parent: String, child: String) {
        let mut g = self.inner.lock().expect("signal registry mutex poisoned");
        g.parents.insert(child, parent);
    }

    /// Forget every pane id not in `known`. Drops their parent links,
    /// queues, and notifiers. Any waiter still hanging on a removed
    /// pane's `Notify` will wake and then see an empty queue with no
    /// matching entries (its outer `timeout` ultimately fires).
    ///
    /// Collects the `Notify`s of removed panes inside the lock, then
    /// drops the lock before calling `notify_waiters()` on each — so
    /// woken waiters that try to re-acquire the registry mutex don't
    /// block on us holding it.
    pub fn retain(&self, known: &HashSet<String>) {
        let drained_notifiers = {
            let mut g = self.inner.lock().expect("signal registry mutex poisoned");
            g.parents
                .retain(|child, parent| known.contains(child) && known.contains(parent));
            g.queues.retain(|pane, _| known.contains(pane));
            let to_drop: Vec<String> = g
                .notifiers
                .keys()
                .filter(|p| !known.contains(*p))
                .cloned()
                .collect();
            to_drop
                .into_iter()
                .filter_map(|p| g.notifiers.remove(&p))
                .collect::<Vec<_>>()
        };
        for n in drained_notifiers {
            n.notify_waiters();
        }
    }

    /// Look up the parent of `child`, if recorded.
    pub fn parent_of(&self, child: &str) -> Option<String> {
        let g = self.inner.lock().expect("signal registry mutex poisoned");
        g.parents.get(child).cloned()
    }

    /// All panes that were spawned by `split`-ing `parent`.
    pub fn children_of(&self, parent: &str) -> Vec<String> {
        let g = self.inner.lock().expect("signal registry mutex poisoned");
        g.parents
            .iter()
            .filter(|(_child, p)| p.as_str() == parent)
            .map(|(child, _p)| child.clone())
            .collect()
    }

    /// Enqueue `signal` on `pane_id`'s queue and wake every waiter on
    /// that pane. Always succeeds — unknown panes simply create a fresh
    /// queue. The caller is responsible for refusing sends to panes
    /// that don't exist in the pane map (see `handlers::signal_send`).
    pub fn enqueue(&self, pane_id: &str, signal: Signal) {
        let notify = {
            let mut g = self.inner.lock().expect("signal registry mutex poisoned");
            g.queues
                .entry(pane_id.to_string())
                .or_default()
                .push_back(signal);
            g.notifiers
                .entry(pane_id.to_string())
                .or_default()
                .clone()
        };
        notify.notify_waiters();
    }

    /// Drain the queue and return its current contents (without
    /// removing anything). Used by `SignalList`.
    pub fn list(&self, pane_id: &str) -> Vec<Signal> {
        let g = self.inner.lock().expect("signal registry mutex poisoned");
        g.queues
            .get(pane_id)
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Remove and return the first signal whose `name` matches. Returns
    /// `None` when the queue is empty or no entry matches.
    pub fn try_pop_named(&self, pane_id: &str, name: &str) -> Option<Signal> {
        let mut g = self.inner.lock().expect("signal registry mutex poisoned");
        let q = g.queues.get_mut(pane_id)?;
        let idx = q.iter().position(|s| s.name == name)?;
        q.remove(idx)
    }

    /// Get-or-create the `Notify` for `pane_id` so callers can `await`
    /// it without holding the registry mutex.
    fn notify_handle(&self, pane_id: &str) -> Arc<Notify> {
        let mut g = self.inner.lock().expect("signal registry mutex poisoned");
        g.notifiers
            .entry(pane_id.to_string())
            .or_default()
            .clone()
    }

    /// Block until a signal named `name` lands on `pane_id`'s queue, or
    /// until `total_timeout` elapses. Returns `None` on timeout.
    ///
    /// Race-free pattern (this is the bit that's easy to get wrong):
    /// `Notify::notified()` only registers a listener when the returned
    /// future is **first polled** — calling `notified()` is not enough.
    /// If we did `try_pop_named` → fail → `notified()` → `pin!` → `await`,
    /// an `enqueue + notify_waiters` that lands in the window between
    /// the queue check and the first poll would be dropped on the floor
    /// (no waiter registered yet) and we'd block until the outer
    /// timeout for a signal that already exists.
    ///
    /// The fix is `tokio::sync::Notify`'s documented pattern: build the
    /// future, pin it, call `enable()` to register the listener
    /// **before** checking the queue, then check, then `await`. Each
    /// wake-up rebuilds + re-enables before the next queue check.
    pub async fn wait_for(
        &self,
        pane_id: &str,
        name: &str,
        total_timeout: Duration,
    ) -> Option<Signal> {
        let deadline = tokio::time::Instant::now() + total_timeout;
        let notify = self.notify_handle(pane_id);
        let mut notified = Box::pin(notify.notified());
        // Register before the first queue check so enqueues racing with
        // us don't drop their wake-up.
        notified.as_mut().enable();
        loop {
            if let Some(s) = self.try_pop_named(pane_id, name) {
                return Some(s);
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return None;
            }
            match timeout(remaining, notified.as_mut()).await {
                Ok(()) => {
                    // Rearm: a fresh `notified()` future, registered
                    // again before we go back to checking the queue.
                    notified.set(notify.notified());
                    notified.as_mut().enable();
                    continue;
                }
                Err(_) => return None,
            }
        }
    }
}

/// Wall-clock milliseconds since the Unix epoch, used for `sent_at_ms`.
/// Saturates to `0` on clock failures (clock-before-1970) — readable
/// timestamps are best-effort metadata, not load-bearing for correctness.
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(name: &str, from: &str) -> Signal {
        Signal {
            name: name.into(),
            data: None,
            sender_pane_id: from.into(),
            sent_at_ms: 1,
        }
    }

    #[test]
    fn register_and_lookup_parent_child() {
        let r = SignalRegistry::new();
        r.register_parent("p".into(), "c".into());
        assert_eq!(r.parent_of("c").as_deref(), Some("p"));
        let kids = r.children_of("p");
        assert_eq!(kids, vec!["c".to_string()]);
    }

    #[test]
    fn children_of_returns_multiple() {
        let r = SignalRegistry::new();
        r.register_parent("p".into(), "c1".into());
        r.register_parent("p".into(), "c2".into());
        let mut kids = r.children_of("p");
        kids.sort();
        assert_eq!(kids, vec!["c1".to_string(), "c2".to_string()]);
    }

    #[test]
    fn enqueue_and_try_pop_named_round_trip() {
        let r = SignalRegistry::new();
        r.enqueue("t", mk("hello", "s"));
        let out = r.try_pop_named("t", "hello").unwrap();
        assert_eq!(out.name, "hello");
        assert!(r.try_pop_named("t", "hello").is_none());
    }

    #[test]
    fn try_pop_named_skips_non_matching_then_finds_match() {
        let r = SignalRegistry::new();
        r.enqueue("t", mk("a", "s1"));
        r.enqueue("t", mk("b", "s2"));
        let out = r.try_pop_named("t", "b").unwrap();
        assert_eq!(out.sender_pane_id, "s2");
        // 'a' is still queued
        assert!(r.try_pop_named("t", "a").is_some());
    }

    #[test]
    fn list_returns_all_without_consuming() {
        let r = SignalRegistry::new();
        r.enqueue("t", mk("a", "s"));
        r.enqueue("t", mk("b", "s"));
        let all = r.list("t");
        assert_eq!(all.len(), 2);
        // Still there after list.
        assert!(r.try_pop_named("t", "a").is_some());
        assert!(r.try_pop_named("t", "b").is_some());
    }

    #[test]
    fn retain_drops_unknown_panes() {
        let r = SignalRegistry::new();
        r.register_parent("p".into(), "c".into());
        r.enqueue("c", mk("x", "p"));

        // 'c' is gone but 'p' still exists.
        let known: HashSet<String> = ["p".to_string()].into_iter().collect();
        r.retain(&known);

        assert!(r.parent_of("c").is_none());
        assert!(r.list("c").is_empty());
    }

    #[test]
    fn retain_drops_parent_link_when_either_side_missing() {
        let r = SignalRegistry::new();
        r.register_parent("p".into(), "c".into());
        // Parent gone, child still around.
        let known: HashSet<String> = ["c".to_string()].into_iter().collect();
        r.retain(&known);
        assert!(r.parent_of("c").is_none());
    }

    #[tokio::test]
    async fn wait_for_returns_immediately_when_already_queued() {
        let r = SignalRegistry::new();
        r.enqueue("t", mk("ready", "s"));
        let got = r
            .wait_for("t", "ready", Duration::from_millis(100))
            .await
            .unwrap();
        assert_eq!(got.name, "ready");
    }

    #[tokio::test]
    async fn wait_for_returns_after_concurrent_enqueue() {
        let r = Arc::new(SignalRegistry::new());
        let r2 = r.clone();
        let task = tokio::spawn(async move {
            r2.wait_for("t", "ready", Duration::from_secs(2)).await
        });
        // Give the waiter a moment to register before enqueueing.
        tokio::time::sleep(Duration::from_millis(20)).await;
        r.enqueue("t", mk("ready", "s"));
        let got = task.await.unwrap();
        assert!(got.is_some());
    }

    #[tokio::test]
    async fn wait_for_returns_none_on_timeout() {
        let r = SignalRegistry::new();
        let got = r
            .wait_for("t", "never", Duration::from_millis(40))
            .await;
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn wait_for_ignores_non_matching_name() {
        let r = Arc::new(SignalRegistry::new());
        let r2 = r.clone();
        let task = tokio::spawn(async move {
            r2.wait_for("t", "wanted", Duration::from_millis(200)).await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        // Enqueue a signal with a different name. The waiter should
        // stay blocked.
        r.enqueue("t", mk("other", "s"));
        tokio::time::sleep(Duration::from_millis(20)).await;
        r.enqueue("t", mk("wanted", "s"));
        let got = task.await.unwrap().unwrap();
        assert_eq!(got.name, "wanted");
    }

    #[tokio::test]
    async fn wait_for_does_not_miss_wakeups_from_a_tight_race() {
        // Regression test for the "Notify listener not registered before
        // queue check" race: `Notify::notify_waiters()` only wakes
        // already-registered listeners, so if `enqueue` fires in the
        // gap between the wait's queue check and its first poll of the
        // `notified()` future, the wake-up is dropped.
        //
        // Loop many iterations to catch the race even when the
        // scheduler hides it on a single run.
        let r = Arc::new(SignalRegistry::new());
        for i in 0..200 {
            let r2 = r.clone();
            let task = tokio::spawn(async move {
                r2.wait_for("t", "race", Duration::from_millis(500)).await
            });
            // Yield once so the spawned task gets scheduled; if the
            // implementation registers before checking the queue, this
            // race is impossible. Tight loop without sleeps so the
            // window is as small as the scheduler allows.
            tokio::task::yield_now().await;
            r.enqueue(
                "t",
                mk("race", "sender"),
            );
            let got = task.await.unwrap();
            assert!(got.is_some(), "iteration {i} missed the wake-up");
        }
    }

    #[tokio::test]
    async fn only_one_waiter_consumes_each_signal() {
        let r = Arc::new(SignalRegistry::new());
        let r1 = r.clone();
        let r2 = r.clone();
        let w1 = tokio::spawn(async move {
            r1.wait_for("t", "go", Duration::from_secs(1)).await
        });
        let w2 = tokio::spawn(async move {
            r2.wait_for("t", "go", Duration::from_secs(1)).await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        r.enqueue("t", mk("go", "s"));
        let r1res = w1.await.unwrap();
        // Second waiter must time out — only one signal was sent.
        let r2res = w2.await.unwrap();
        let got = r1res.is_some() as u32 + r2res.is_some() as u32;
        assert_eq!(got, 1, "exactly one waiter should see the signal");
    }
}
