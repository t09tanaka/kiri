//! Shared wire protocol for the kiri CLI.
//!
//! See `docs/superpowers/specs/2026-04-29-kiri-cli-design.md` for the design.

pub mod types;

pub use types::{ErrorCode, PaneRef, SplitDirection};

pub const SCHEMA_VERSION: u32 = 1;
