//! Parsing utilities for BetterQuesting "DefaultQuests" data.
//!
//! This module parses the directory layout produced by the BetterQuesting mod
//! (optional `QuestSettings` file, `Quests/` and `QuestLines/`) and converts the
//! JSON/NBT-like contents into the crate's in-memory model types (`QuestDatabase`,
//! `Quest`, `QuestLine`, `QuestLineEntry`, `QuestSettings`, and friends).
//!
//! Parsing is performed with `serde_json` after normalizing values using
//! `nbt_norm::normalize_value`. The module is filesystem-agnostic: callers provide
//! a `QuestDataSource` implementation which abstracts listing directories and
//! reading files (this makes testing easier and keeps parsing logic decoupled
//! from IO).
//!
//! The primary entry point is `parse_default_quests_dir_from_source`. IDs are
//! constructed from "High"/"Low" components (e.g. `questIDHigh`/`questIDLow`),
//! and questline parsing validates references - missing or duplicate IDs yield
//! `crate::error::ParseError` values rather than panics.
//!
//! Settings parsing prefers `properties -> betterquesting -> ...`, then a direct
//! `betterquesting` object, and finally falls back to top-level keys.
//!
//! Public functions return `Result<...>` to allow callers to handle parse errors.
use crate::error::{ParseError, Result};
use crate::model::*;
use crate::quest_id::QuestId;
use serde_json::Value;
use std::collections::HashMap;

/// Type alias for the result of parsing a questline directory.
type QuestlineDirParseResult = (Option<QuestLine>, Vec<(QuestId, QuestLineEntry)>);

/// Abstracts file/directory access for quest parsing.
pub trait QuestDataSource {
    /// List entries in a directory (returns file/dir names, not full paths).
    fn list_dir(&self, path: &str) -> Result<Vec<String>>;
    /// Returns true if the path is a directory.
    fn is_dir(&self, path: &str) -> bool;
    /// Returns true if the path is a file.
    fn is_file(&self, path: &str) -> bool;
    /// Reads the file at path to a string.
    fn read_to_string(&self, path: &str) -> Result<String>;
}

/// Parse the DefaultQuests folder into a QuestDatabase using an abstract data source.
pub fn parse_default_quests_dir_from_source(
    source: &dyn QuestDataSource,
    root: &str,
) -> Result<QuestDatabase> {
    if !source.is_dir(root) {
        return Err(ParseError::InvalidFormat(format!("not a dir: {}", root)));
    }

    // settings: optional file named QuestSettings.json or QuestSettings
    let mut settings: Option<QuestSettings> = None;
    let settings_paths = ["QuestSettings.json", "QuestSettings"];
    for p in &settings_paths {
        let fp = format!("{}/{}", root, p);
        if source.is_file(&fp) {
            settings = Some(parse_settings_file_from_source(source, &fp)?);
            break;
        }
    }

    // parse quests
    let mut quests: HashMap<QuestId, Quest> = HashMap::new();
    let quests_dir = format!("{}/Quests", root);
    if source.is_dir(&quests_dir) {
        for entry in source.list_dir(&quests_dir)? {
            let path = format!("{}/{}", &quests_dir, entry);
            if source.is_file(&path) && path.ends_with(".json") {
                let s = source.read_to_string(&path)?;
                // Deserialize into the RawQuest directly; normalization happens during conversion
                let raw: crate::model_raw::RawQuest = serde_json::from_str(&s)?;
                let quest = Quest::from_raw(raw)?;
                if quests.insert(quest.id, quest).is_some() {
                    return Err(ParseError::DuplicateQuestId(path));
                }
            }
        }
    }

    // parse questlines
    let (questlines, questline_order) =
        parse_questlines_dir_from_source(source, &format!("{}/QuestLines", root))?;

    // resolve references (strict: fail on missing quest)
    for (qlid, qline) in &questlines {
        for entry in &qline.entries {
            if !quests.contains_key(&entry.quest_id) {
                return Err(ParseError::MissingQuestReference {
                    questline: qlid.as_u64(),
                    quest_id: entry.quest_id,
                });
            }
        }
    }

    Ok(QuestDatabase {
        settings,
        quests,
        questlines,
        questline_order,
    })
}

/// Parse the QuestLines directory into a map of QuestLine and their order.
fn parse_questlines_dir_from_source(
    source: &dyn QuestDataSource,
    qlines_dir: &str,
) -> Result<(HashMap<QuestId, QuestLine>, Vec<QuestId>)> {
    let mut questlines: HashMap<QuestId, QuestLine> = HashMap::new();
    let mut questline_order: Vec<QuestId> = Vec::new();
    if source.is_dir(qlines_dir) {
        for entry in source.list_dir(qlines_dir)? {
            let path = format!("{}/{}", qlines_dir, entry);
            if source.is_dir(&path) {
                let (qline_opt, entries) = parse_questline_dir_from_source(source, &path)?;
                if let Some(mut qline) = qline_opt {
                    let mut sorted_entries: Vec<(QuestId, QuestLineEntry)> = entries;
                    sorted_entries.sort_by_key(|(qid, _entry)| qid.as_u64());
                    for (_qid, entry) in sorted_entries {
                        qline.entries.push(entry);
                    }
                    if questlines.insert(qline.id, qline).is_some() {
                        return Err(ParseError::DuplicateQuestId(path));
                    }
                }
            }
        }
    }
    if questline_order.is_empty() {
        questline_order = questlines.keys().cloned().collect();
    }
    Ok((questlines, questline_order))
}

/// Parse a single questline directory, returning the QuestLine (if present) and its entries.
fn parse_questline_dir_from_source(
    source: &dyn QuestDataSource,
    path: &str,
) -> Result<QuestlineDirParseResult> {
    let qline_json = format!("{}/QuestLine.json", path);
    let mut qline_opt: Option<QuestLine> = None;
    if source.is_file(&qline_json) {
        let s = source.read_to_string(&qline_json)?;
        let v: Value = serde_json::from_str(&s)?;
        // Normalize only the questline object for field extraction
        let norm = crate::nbt_norm::normalize_value(v);
        if let Value::Object(map) = norm {
            let high = map
                .get("questLineIDHigh")
                .and_then(|x| x.as_i64())
                .map(|n| n as i32)
                .unwrap_or(0);
            let low = map
                .get("questLineIDLow")
                .and_then(|x| x.as_i64())
                .map(|n| n as i32)
                .unwrap_or(0);
            let id = QuestId::from_parts(high, low);
            let props = map.get("properties").and_then(|p| {
                if let Some(obj) = p.as_object() {
                    if let Some(bqv) = obj.get("betterquesting") {
                        let bq_norm = crate::nbt_norm::normalize_value(bqv.clone());
                        serde_json::from_value::<QuestProperties>(bq_norm).ok()
                    } else if let Some((_k, inner)) = obj.iter().next() {
                        let inner_norm = crate::nbt_norm::normalize_value(inner.clone());
                        serde_json::from_value::<QuestProperties>(inner_norm).ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            qline_opt = Some(QuestLine {
                id,
                properties: props,
                entries: Vec::new(),
                extra: HashMap::new(),
            });
        }
    }
    let mut entries: Vec<(QuestId, QuestLineEntry)> = Vec::new();
    if source.is_dir(path) {
        for entry in source.list_dir(path)? {
            let p = format!("{}/{}", path, entry);
            if source.is_file(&p) && p.ends_with(".json") {
                if entry == "QuestLine.json" {
                    continue;
                }
                if let Some((qid, entry)) = parse_questline_entry_file_from_source(source, &p)? {
                    entries.push((qid, entry));
                }
            }
        }
    }
    Ok((qline_opt, entries))
}

/// Parse a questline entry file, returning the QuestId and QuestLineEntry if valid.
fn parse_questline_entry_file_from_source(
    source: &dyn QuestDataSource,
    p: &str,
) -> Result<Option<(QuestId, QuestLineEntry)>> {
    let s = source.read_to_string(p)?;
    let v: Value = serde_json::from_str(&s)?;
    // Normalize this entry object before extracting fields
    let norm = crate::nbt_norm::normalize_value(v);
    if let Value::Object(map) = norm {
        let high = map
            .get("questIDHigh")
            .and_then(|x| x.as_i64())
            .map(|n| n as i32)
            .unwrap_or(0);
        let low = map
            .get("questIDLow")
            .and_then(|x| x.as_i64())
            .map(|n| n as i32)
            .unwrap_or(0);
        let qid = QuestId::from_parts(high, low);
        let entry = QuestLineEntry {
            index: None,
            quest_id: qid,
            x: map.get("x").and_then(|x| x.as_i64().map(|n| n as i32)),
            y: map.get("y").and_then(|x| x.as_i64().map(|n| n as i32)),
            size_x: map.get("sizeX").and_then(|x| x.as_i64().map(|n| n as i32)),
            size_y: map.get("sizeY").and_then(|x| x.as_i64().map(|n| n as i32)),
            extra: HashMap::new(),
        };
        Ok(Some((qid, entry)))
    } else {
        Ok(None)
    }
}

fn parse_settings_file_from_source(
    source: &dyn QuestDataSource,
    path: &str,
) -> Result<QuestSettings> {
    let s = source.read_to_string(path)?;
    let v: Value = serde_json::from_str(&s)?;
    // Do targeted normalization inside parse_settings_value if needed; pass raw value here
    Ok(parse_settings_value(&v))
}

fn parse_settings_value(v: &Value) -> QuestSettings {
    let mut version: Option<String> = None;
    let mut extra: HashMap<String, Value> = HashMap::new();

    if let Some(map) = v.as_object() {
        // prefer properties -> betterquesting -> inner
        if let Some(props_val) = map.get("properties")
            && let Some(props_map) = props_val.as_object()
        {
            let inner_val = if let Some(bq) = props_map.get("betterquesting") {
                bq
            } else if let Some((_k, v)) = props_map.iter().next() {
                v
            } else {
                &Value::Null
            };
            if let Some(inner_map) = inner_val.as_object() {
                version = inner_map
                    .get("version")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                for (k, val) in inner_map.iter() {
                    if k == "version" {
                        continue;
                    }
                    extra.insert(k.clone(), val.clone());
                }
                return QuestSettings { version, extra };
            }
        }

        // check direct betterquesting key
        if let Some(bq_val) = map.get("betterquesting")
            && let Some(bq_map) = bq_val.as_object()
        {
            version = bq_map
                .get("version")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string());
            for (k, val) in bq_map.iter() {
                if k == "version" {
                    continue;
                }
                extra.insert(k.clone(), val.clone());
            }
            return QuestSettings { version, extra };
        }

        // fallback: top-level version + extras
        version = map
            .get("version")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        for (k, val) in map.iter() {
            if k == "version" {
                continue;
            }
            extra.insert(k.clone(), val.clone());
        }
    }

    QuestSettings { version, extra }
}
