//! Tracks pending oneshot waiters keyed by request id.
//!
//! `Split` and `Close` round-trip through the frontend (via Tauri events
//! like `cli:pane-split`). The handler `register`s a waiter, emits the
//! event, and awaits the receiver. The frontend later calls the
//! `cli_resolve_pending` Tauri command which calls `resolve` here.

use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;

#[derive(Default)]
pub struct PendingReplies {
    inner: Mutex<HashMap<String, oneshot::Sender<serde_json::Value>>>,
}

impl PendingReplies {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a waiter for `request_id`. The returned receiver
    /// resolves when `resolve(request_id, ..)` is called.
    pub fn register(&self, request_id: String) -> oneshot::Receiver<serde_json::Value> {
        let (tx, rx) = oneshot::channel();
        let mut map = self.inner.lock().expect("pending mutex poisoned");
        map.insert(request_id, tx);
        rx
    }

    /// Resolve a registered waiter. Returns `true` if a sender was found
    /// and notified, `false` otherwise.
    pub fn resolve(&self, request_id: &str, value: serde_json::Value) -> bool {
        let sender = {
            let mut map = self.inner.lock().expect("pending mutex poisoned");
            map.remove(request_id)
        };
        match sender {
            Some(tx) => tx.send(value).is_ok(),
            None => false,
        }
    }

    /// Drop a registered sender without sending a value. Used by the
    /// caller to clean up after `emit_to` failures or timeouts so the
    /// map doesn't accumulate stranded entries.
    pub fn cancel(&self, request_id: &str) {
        let mut map = self.inner.lock().expect("pending mutex poisoned");
        map.remove(request_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_then_resolve_delivers_value() {
        let pr = PendingReplies::new();
        let rx = pr.register("req-1".into());
        let payload = serde_json::json!({ "ok": true });
        assert!(pr.resolve("req-1", payload.clone()));
        let got = rx.await.expect("channel sender dropped");
        assert_eq!(got, payload);
    }

    #[test]
    fn resolve_unknown_id_returns_false() {
        let pr = PendingReplies::new();
        assert!(!pr.resolve("nobody-home", serde_json::json!(null)));
    }

    #[tokio::test]
    async fn cancel_removes_entry_so_subsequent_resolve_is_noop() {
        let pr = PendingReplies::new();
        let rx = pr.register("req-c".into());
        pr.cancel("req-c");
        assert!(!pr.resolve("req-c", serde_json::json!({})));
        assert!(rx.await.is_err()); // sender dropped
    }
}
