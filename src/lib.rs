//! BetterQuestingTools — a small parser for BetterQuesting DefaultQuests
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
//! ```

pub mod db;
pub mod error;
pub mod importance;
pub mod model;
pub mod model_raw;
pub mod nbt_norm;
pub mod parser;
pub mod quest_id;

pub use crate::db::*;
pub use crate::error::*;
pub use crate::importance::*;
pub use crate::model::*;
pub use crate::parser::{parse_quest_from_file, parse_quest_from_reader, parse_quest_from_value};
