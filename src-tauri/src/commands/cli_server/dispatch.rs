//! Per-connection dispatch context and per-terminal ring-buffer registry.
//!
//! `DispatchContext` is a cheap-to-clone bag of dependencies passed to
//! every handler in `super::handlers`. `TerminalBuffers` owns one
//! `RingBuffer` per active terminal id and lazily spawns a tokio task
//! that drains the `TerminalOutputBus` into the buffer the first time
//! a terminal is touched.

use super::frontend_bridge::PendingReplies;
use super::handlers;
use super::pane_map::PaneMap;
use super::ring_buffer::RingBuffer;
use super::signals::SignalRegistry;
use crate::commands::terminal::{TerminalOutputBus, TerminalOutputBusState, TerminalState};
use kiri_cli_proto::{ErrorCode, Request, Response};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub type SharedRingBuffer = Arc<Mutex<RingBuffer>>;

/// Per-terminal scrollback retained for `Read` / `Follow`.
const RING_BUFFER_CAP: usize = 256 * 1024;

#[derive(Default)]
pub struct TerminalBuffers {
    inner: Mutex<HashMap<u32, SharedRingBuffer>>,
}

impl TerminalBuffers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get-or-create the per-terminal ring buffer. The first call for a
    /// given `terminal_id` spawns a tokio task that subscribes to the
    /// bus and pushes every chunk into the buffer. Subsequent calls
    /// return the same `Arc`.
    pub fn ensure_subscribed(&self, terminal_id: u32, bus: &TerminalOutputBus) -> SharedRingBuffer {
        let mut map = self.inner.lock().expect("buffers mutex poisoned");
        if let Some(rb) = map.get(&terminal_id) {
            return rb.clone();
        }
        let rb = Arc::new(Mutex::new(RingBuffer::new(RING_BUFFER_CAP)));
        map.insert(terminal_id, rb.clone());

        // Subscribe BEFORE returning so callers don't race with output
        // arriving in the gap between insert and spawn.
        let mut rx = bus.subscribe(terminal_id);
        let rb_for_task = rb.clone();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(chunk) => {
                        let mut guard = rb_for_task
                            .lock()
                            .expect("ring buffer mutex poisoned in subscriber");
                        guard.push(&chunk);
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        rb
    }

    pub fn get(&self, terminal_id: u32) -> Option<SharedRingBuffer> {
        let map = self.inner.lock().expect("buffers mutex poisoned");
        map.get(&terminal_id).cloned()
    }

    pub fn retain_terminal_ids(&self, known: &HashSet<u32>) {
        let mut map = self.inner.lock().expect("buffers mutex poisoned");
        map.retain(|terminal_id, _| known.contains(terminal_id));
    }
}

#[derive(Clone)]
pub struct DispatchContext {
    pub label: String,
    pub app: Option<tauri::AppHandle>,
    pub terminals: TerminalState,
    pub bus: TerminalOutputBusState,
    pub pane_map: Arc<PaneMap>,
    pub pending: Arc<PendingReplies>,
    pub buffers: Arc<TerminalBuffers>,
    pub signals: Arc<SignalRegistry>,
}

/// Parse a single newline-delimited JSON line into a `Request` and run
/// the appropriate handler. Returns the `Response`s the connection
/// should write back. (Most requests produce a single response;
/// `Follow` produces a stream.)
pub async fn dispatch_line(ctx: &DispatchContext, line: &str) -> Vec<Response> {
    let req: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            return vec![Response::Error {
                code: ErrorCode::ProtocolError,
                message: format!("invalid request JSON: {e}"),
                detail: None,
            }];
        }
    };
    handlers::handle(ctx, req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;
    use std::time::Duration;

    fn make_ctx(terminals: TerminalState, bus: TerminalOutputBusState) -> DispatchContext {
        DispatchContext {
            label: "test".into(),
            app: None,
            terminals,
            bus,
            pane_map: Arc::new(PaneMap::new()),
            pending: Arc::new(PendingReplies::new()),
            buffers: Arc::new(TerminalBuffers::new()),
            signals: Arc::new(SignalRegistry::new()),
        }
    }

    #[tokio::test]
    async fn ensure_subscribed_pushes_published_bytes_to_ring_buffer() {
        let terminals: TerminalState = Arc::new(StdMutex::new(
            crate::commands::terminal::TerminalManager::new(),
        ));
        let bus: TerminalOutputBusState = Arc::new(TerminalOutputBus::new());
        let ctx = make_ctx(terminals, bus.clone());

        let rb = ctx.buffers.ensure_subscribed(1, &ctx.bus);
        // Give the spawned task a chance to install the receiver, then
        // publish.
        tokio::time::sleep(Duration::from_millis(20)).await;
        let n = bus.publish(1, b"abc");
        assert_eq!(n, 1);
        // Wait for the subscriber to drain.
        tokio::time::sleep(Duration::from_millis(20)).await;

        let guard = rb.lock().expect("rb");
        let (bytes, dropped) = guard.read_since(0);
        assert_eq!(bytes, b"abc");
        assert_eq!(dropped, 0);
    }

    #[tokio::test]
    async fn retain_terminal_ids_drops_closed_terminal_buffers() {
        let buffers = TerminalBuffers::new();
        let bus: TerminalOutputBusState = Arc::new(TerminalOutputBus::new());
        buffers.ensure_subscribed(1, &bus);
        buffers.ensure_subscribed(2, &bus);

        buffers.retain_terminal_ids(&HashSet::from([2]));

        assert!(buffers.get(1).is_none());
        assert!(buffers.get(2).is_some());
    }
}
