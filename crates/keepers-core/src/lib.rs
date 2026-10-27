//! Keepers core: the library that decides everything.
//!
//! Nothing in this crate knows about the terminal or the desktop shell. The CLI and the
//! Tauri host are thin callers. Core code never reads the clock, randomness, the filesystem
//! or the network directly; they are injected (see [`env`]).

pub mod env;
pub mod error;

/// The crate version, which the CLI and the app report as the product version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
