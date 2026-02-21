use crate::error::Result;
use crate::model::*;
use crate::model_raw::*;
use serde_json::Value;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Parse a quest from a reader using serde and the raw model, then convert to the optimized model.
pub fn parse_quest_from_reader<R: Read>(mut r: R) -> Result<Quest> {
    let mut s = String::new();
    r.read_to_string(&mut s)?;
    // Parse input to a serde_json::Value so we can normalize NBT-style keys
    // (these often include ":<type>" suffixes) before deserializing into the
    // strongly-typed raw model. Normalization converts keys like
    // "questIDLow:4" -> "questIDLow" and converts numeric-keyed maps into
    // arrays where appropriate.
    let v: Value = serde_json::from_str(&s)?;
    let v_norm = crate::nbt_norm::normalize_value(v);
    let raw: RawQuest = serde_json::from_value(v_norm)?;
    Quest::from_raw(raw)
}

pub fn parse_quest_from_file(path: &Path) -> Result<Quest> {
    let f = File::open(path)?;
    parse_quest_from_reader(f)
}

/// Deprecated: use parse_quest_from_reader or parse_quest_from_file instead.
pub fn parse_quest_from_value(v: &Value) -> Result<Quest> {
    let raw: RawQuest = serde_json::from_value(v.clone())?;
    Quest::from_raw(raw)
}
