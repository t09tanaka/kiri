//! Shared wire protocol for the kiri CLI.
//!
//! See `docs/superpowers/specs/2026-04-29-kiri-cli-design.md` for the design.

pub mod types;
pub mod wire;

pub use types::{ErrorCode, PaneColor, PaneRef, SplitDirection};
pub use wire::{PaneInfo, Request, Response};

pub const SCHEMA_VERSION: u32 = 1;
