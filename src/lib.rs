//! stet: keeps Markdown knowledge repos in shape. One shared lint setup, the house rules
//! no linter knows, the inbox as data, and a safe way to publish a note.
//!
//! The `stet` binary is the whole product. Everything it does lives in this library, and
//! `contract` holds the shape of every `--json` document it prints.

pub mod check;
pub mod cli;
pub mod config;
pub mod contract;
pub mod env;
pub mod generate;
pub mod help;
pub mod markdown;
pub mod rules;
pub mod sync;
pub mod tools;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
