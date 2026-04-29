//! Per-window CLI server.
//!
//! Each kiri window gets its own Unix Domain Socket at
//! `~/.kiri/instances/<label>.sock`. The socket accepts newline-
//! delimited JSON requests, dispatches them to handlers, and replies
//! with newline-delimited JSON responses.
//!
//! This phase exposes the pure-logic submodules; the listener spawn,
//! dispatch context, request handlers, and Tauri commands land in a
//! follow-up commit.

pub mod frontend_bridge;
pub mod pane_map;
pub mod ring_buffer;
pub mod run_logic;
