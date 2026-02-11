use crate::error::{ParseError, Result};
use crate::model::*;
use crate::nbt_norm::{map_to_array_if_numeric, normalize_value};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn parse_quest_from_reader<R: Read>(mut r: R) -> Result<Quest> {
    let mut s = String::new();
    r.read_to_string(&mut s)
        .map_err(|e| ParseError::Unexpected(e.to_string()))?;
    let v: Value = serde_json::from_str(&s)?;
    let norm = normalize_value(v);
    parse_quest_from_value(&norm)
}

pub fn parse_quest_from_file(path: &Path) -> Result<Quest> {
    let f = File::open(path).map_err(|e| ParseError::Unexpected(e.to_string()))?;
    parse_quest_from_reader(f)
}

fn parse_quest_from_value(v: &Value) -> Result<Quest> {
    let obj = v
        .as_object()
        .ok_or_else(|| ParseError::Unexpected("root not an object".into()))?;

    let high = get_i32(obj, "questIDHigh").unwrap_or(0);
    let low = get_i32(obj, "questIDLow").unwrap_or(0);
    let id = QuestId { high, low };

    // properties: may contain nested betterquesting key
    let properties = if let Some(pv) = obj.get("properties") {
        if let Some(map) = pv.as_object() {
            // unwrap betterquesting if present
            if let Some(bqv) = map.get("betterquesting") {
                parse_properties(bqv)?
            } else if let Some((_k, inner)) = map.iter().next() {
                // sometimes keyed by names like "betterquesting"
                parse_properties(inner)?
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let tasks = parse_indexed_array_of(obj.get("tasks"));
    let rewards = parse_indexed_array_of(obj.get("rewards"));

    let prerequisites = if let Some(pre) = obj.get("preRequisites") {
        if let Some(map) = pre.as_object() {
            if let Some(vec) = map_to_array_if_numeric(map) {
                vec.into_iter()
                    .filter_map(|v| {
                        v.as_object().map(|m| QuestId {
                            high: get_i32(m, "questIDHigh").unwrap_or(0),
                            low: get_i32(m, "questIDLow").unwrap_or(0),
                        })
                    })
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    Ok(Quest {
        id,
        properties,
        tasks,
        rewards,
        prerequisites,
    })
}

fn get_i32(m: &Map<String, Value>, k: &str) -> Option<i32> {
    m.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_i64().map(|x| x as i32),
        Value::String(s) => s.parse::<i32>().ok(),
        _ => None,
    })
}

fn parse_properties(v: &Value) -> Result<Option<QuestProperties>> {
    let map = match v.as_object() {
        Some(m) => m,
        None => return Ok(None),
    };
    let name = map
        .get("name")
        .and_then(|x| x.as_str().map(|s| s.to_string()));
    let desc = map
        .get("desc")
        .and_then(|x| x.as_str().map(|s| s.to_string()));
    let icon = map.get("icon").and_then(|x| parse_item(x));

    let is_main = map.get("isMain").and_then(parse_bool_like);
    let is_silent = map.get("isSilent").and_then(parse_bool_like);
    let auto_claim = map.get("autoClaim").and_then(parse_bool_like);
    let quest_logic = map
        .get("questLogic")
        .and_then(|x| x.as_str().map(|s| s.to_string()));
    let task_logic = map
        .get("taskLogic")
        .and_then(|x| x.as_str().map(|s| s.to_string()));
    let visibility = map
        .get("visibility")
        .and_then(|x| x.as_str().map(|s| s.to_string()));

    // collect extras
    let mut extra = HashMap::new();
    for (k, v) in map.iter() {
        if [
            "name",
            "desc",
            "icon",
            "isMain",
            "isSilent",
            "autoClaim",
            "questLogic",
            "taskLogic",
            "visibility",
        ]
        .contains(&k.as_str())
        {
            continue;
        }
        extra.insert(k.clone(), v.clone());
    }

    Ok(Some(QuestProperties {
        name,
        desc,
        icon,
        is_main,
        is_silent,
        auto_claim,
        quest_logic,
        task_logic,
        visibility,
        extra,
    }))
}

fn parse_bool_like(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::Number(n) => n.as_i64().map(|x| x != 0),
        Value::String(s) => match s.as_str() {
            "0" => Some(false),
            "1" => Some(true),
            _ => None,
        },
        _ => None,
    }
}

fn parse_item(v: &Value) -> Option<ItemStack> {
    let map = v.as_object()?;
    let id = map
        .get("id")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())?;
    let damage = map
        .get("Damage")
        .and_then(|x| x.as_i64().map(|n| n as i32))
        .or_else(|| map.get("damage").and_then(|x| x.as_i64().map(|n| n as i32)));
    let count = map
        .get("Count")
        .and_then(|x| x.as_i64().map(|n| n as i32))
        .or_else(|| map.get("count").and_then(|x| x.as_i64().map(|n| n as i32)));
    let oredict = map
        .get("OreDict")
        .and_then(|x| x.as_str().map(|s| s.to_string()))
        .or_else(|| {
            map.get("oreDict")
                .and_then(|x| x.as_str().map(|s| s.to_string()))
        });

    // extras
    let mut extra = HashMap::new();
    for (k, val) in map.iter() {
        if [
            "id", "Damage", "damage", "Count", "count", "OreDict", "oreDict",
        ]
        .contains(&k.as_str())
        {
            continue;
        }
        extra.insert(k.clone(), val.clone());
    }

    Some(ItemStack {
        id,
        damage,
        count,
        oredict,
        extra,
    })
}

fn parse_indexed_array_of<T>(opt: Option<&Value>) -> Vec<T>
where
    T: for<'de> serde::de::Deserialize<'de>,
{
    if let Some(v) = opt {
        if let Some(map) = v.as_object() {
            if let Some(arr) = map_to_array_if_numeric(map) {
                // try to deserialize each element into T
                return arr
                    .into_iter()
                    .filter_map(|elem| serde_json::from_value(elem).ok())
                    .collect();
            }
        }
    }
    Vec::new()
}

// File-system dependent tests belong in the integration test directory `tests/`.
