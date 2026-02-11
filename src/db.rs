use crate::error::{ParseError, Result};
use crate::model::*;
use crate::nbt_norm::normalize_value;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
                if quests.insert(quest.id.clone(), quest).is_some() {
                    return Err(ParseError::DuplicateQuestId(path.display().to_string()));
                }
            }
        }
    }

    // parse questlines
    let mut questlines: HashMap<QuestId, QuestLine> = HashMap::new();
    let mut questline_order: Vec<QuestId> = Vec::new();
    let qlines_dir = dir.join("QuestLines");
    if qlines_dir.is_dir() {
        for entry in fs::read_dir(&qlines_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // directory per questline; inside there may be QuestLine.json and many entry files
                let qline_json = path.join("QuestLine.json");
                let mut qline_opt: Option<QuestLine> = None;
                if qline_json.is_file() {
                    let s = fs::read_to_string(&qline_json)?;
                    let v: Value = serde_json::from_str(&s)?;
                    let norm = normalize_value(v);
                    // parse basic questline id & props
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
                        let id = QuestId { high, low };
                        let props = map
                            .get("properties")
                            .and_then(|p| crate::parser::parse_properties(p).ok().flatten());
                        qline_opt = Some(QuestLine {
                            id: id.clone(),
                            properties: props,
                            entries: Vec::new(),
                            extra: HashMap::new(),
                        });
                    }
                }

                // collect entry files
                let mut entries: Vec<(QuestId, QuestLineEntry)> = Vec::new();
                for e in fs::read_dir(&path)? {
                    let e = e?;
                    let p = e.path();
                    if p.is_file() && p.extension().map(|s| s == "json").unwrap_or(false) {
                        if p.file_name().and_then(|n| n.to_str()) == Some("QuestLine.json") {
                            continue;
                        }
                        let s = fs::read_to_string(&p)?;
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
                            let qid = QuestId { high, low };
                            let entry = QuestLineEntry {
                                index: None,
                                quest_id: qid.clone(),
                                x: map.get("x").and_then(|x| x.as_i64().map(|n| n as i32)),
                                y: map.get("y").and_then(|x| x.as_i64().map(|n| n as i32)),
                                size_x: map.get("sizeX").and_then(|x| x.as_i64().map(|n| n as i32)),
                                size_y: map.get("sizeY").and_then(|x| x.as_i64().map(|n| n as i32)),
                                extra: HashMap::new(),
                            };
                            entries.push((qid, entry));
                        }
                    }
                }

                if let Some(mut qline) = qline_opt {
                    // sort entries by filename order? they may be numeric in names; use filesystem order for now
                    entries.sort_by_key(|(qid, _entry)| qid.as_u64());
                    for (_qid, entry) in entries {
                        qline.entries.push(entry);
                    }
                    if questlines.insert(qline.id.clone(), qline).is_some() {
                        return Err(ParseError::DuplicateQuestId(path.display().to_string()));
                    }
                }
            }
        }
    }

    // derive questline order from keys if not present
    if questline_order.is_empty() {
        questline_order = questlines.keys().cloned().collect();
    }

    // resolve references (strict: fail on missing quest)
    for (qlid, qline) in &questlines {
        for entry in &qline.entries {
            if !quests.contains_key(&entry.quest_id) {
                return Err(ParseError::MissingQuestReference {
                    questline: qlid.as_u64(),
                    quest_id: entry.quest_id.clone(),
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
        if let Some(props_val) = map.get("properties") {
            if let Some(props_map) = props_val.as_object() {
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
        }

        // check direct betterquesting key
        if let Some(bq_val) = map.get("betterquesting") {
            if let Some(bq_map) = bq_val.as_object() {
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
