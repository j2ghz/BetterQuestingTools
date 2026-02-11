pub mod db;
//! bq-viewer — a small parser for BetterQuesting DefaultQuests
//!
//! This crate provides utilities to parse BetterQuesting's JSON export (the
//! `DefaultQuests` folder produced by the mod) into a Rust-friendly domain
//! model. It normalizes NBT-style key suffixes (like `name:8`) and numeric-keyed
//! maps into arrays, then exposes typed structs for quests, questlines and the
//! global settings.
//!
//! Basic example (no-run):
//!
//! ```rust,no_run
//! use bq_viewer::db::parse_default_quests_dir;
//! use std::path::Path;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let db = parse_default_quests_dir(Path::new("path/to/DefaultQuests"))?;
//!     println!("parsed {} quests", db.quests.len());
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod model;
pub mod nbt_norm;
pub mod parser;
pub mod db;

pub use crate::error::*;
pub use crate::model::*;
pub use crate::parser::*;
pub use crate::db::*;
