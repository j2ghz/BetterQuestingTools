use crate::error::{ParseError, Result};
use crate::model::*;
use crate::nbt_norm::normalize_value;
use crate::quest_id::QuestId;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Type alias for the result of parsing a questline directory.
type QuestlineDirParseResult = (Option<QuestLine>, Vec<(QuestId, QuestLineEntry)>);

/// Parse the DefaultQuests folder into a QuestDatabase. This is strict: missing references
/// will return Err(ParseError::Unexpected(...)).
pub fn parse_default_quests_dir(dir: &Path) -> Result<QuestDatabase> {
    if !dir.is_dir() {
        return Err(ParseError::InvalidFormat(format!(
            "not a dir: {}",
            dir.display()
        )));
    }

    // settings: optional file named QuestSettings.json or QuestSettings
    let mut settings: Option<QuestSettings> = None;
    let settings_paths = ["QuestSettings.json", "QuestSettings"];
    for p in &settings_paths {
        let fp = dir.join(p);
        if fp.is_file() {
            settings = Some(parse_settings_file(&fp)?);
            break;
        }
    }

    // parse quests
    let mut quests: HashMap<QuestId, Quest> = HashMap::new();
    let quests_dir = dir.join("Quests");
    if quests_dir.is_dir() {
        for entry in fs::read_dir(&quests_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map(|s| s == "json").unwrap_or(false) {
                let s = fs::read_to_string(&path)?;
                let v: Value = serde_json::from_str(&s)?;
                let norm = normalize_value(v);
                let quest = crate::parser::parse_quest_from_value(&norm)?;
                if quests.insert(quest.id, quest).is_some() {
                    return Err(ParseError::DuplicateQuestId(path.display().to_string()));
                }
            }
        }
    }

    // parse questlines
    let (questlines, questline_order) = parse_questlines_dir(&dir.join("QuestLines"))?;

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
fn parse_questlines_dir(qlines_dir: &Path) -> Result<(HashMap<QuestId, QuestLine>, Vec<QuestId>)> {
    let mut questlines: HashMap<QuestId, QuestLine> = HashMap::new();
    let mut questline_order: Vec<QuestId> = Vec::new();
    if qlines_dir.is_dir() {
        for entry in fs::read_dir(qlines_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let (qline_opt, entries) = parse_questline_dir(&path)?;
                if let Some(mut qline) = qline_opt {
                    let mut sorted_entries: Vec<(QuestId, QuestLineEntry)> = entries;
                    sorted_entries.sort_by_key(|(qid, _entry)| qid.as_u64());
                    for (_qid, entry) in sorted_entries {
                        qline.entries.push(entry);
                    }
                    if questlines.insert(qline.id, qline).is_some() {
                        return Err(ParseError::DuplicateQuestId(path.display().to_string()));
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
fn parse_questline_dir(path: &Path) -> Result<QuestlineDirParseResult> {
    let qline_json = path.join("QuestLine.json");
    let mut qline_opt: Option<QuestLine> = None;
    if qline_json.is_file() {
        let s = fs::read_to_string(&qline_json)?;
        let v: Value = serde_json::from_str(&s)?;
        let norm = normalize_value(v);
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
            let props = map
                .get("properties")
                .and_then(|p| crate::parser::parse_properties(p).ok().flatten());
            qline_opt = Some(QuestLine {
                id,
                properties: props,
                entries: Vec::new(),
                extra: HashMap::new(),
            });
        }
    }
    let mut entries: Vec<(QuestId, QuestLineEntry)> = Vec::new();
    for e in fs::read_dir(path)? {
        let e = e?;
        let p = e.path();
        if p.is_file() && p.extension().map(|s| s == "json").unwrap_or(false) {
            if p.file_name().and_then(|n| n.to_str()) == Some("QuestLine.json") {
                continue;
            }
            if let Some((qid, entry)) = parse_questline_entry_file(&p)? {
                entries.push((qid, entry));
            }
        }
    }
    Ok((qline_opt, entries))
}

/// Parse a questline entry file, returning the QuestId and QuestLineEntry if valid.
fn parse_questline_entry_file(p: &Path) -> Result<Option<(QuestId, QuestLineEntry)>> {
    let s = fs::read_to_string(p)?;
    let v: Value = serde_json::from_str(&s)?;
    let norm = normalize_value(v);
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

fn parse_settings_file(path: &Path) -> Result<QuestSettings> {
    let s = fs::read_to_string(path)?;
    let v: Value = serde_json::from_str(&s)?;
    let norm = normalize_value(v);
    Ok(parse_settings_value(&norm))
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
