//! stet: keeps Markdown knowledge repos in shape.
//!
//! The `stet` binary is the whole product. Everything it does lives in this library.

pub mod cli;
pub mod env;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
