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
    r.read_to_string(&mut s)?;
    let v: Value = serde_json::from_str(&s)?;
    let norm = normalize_value(v);
    parse_quest_from_value(&norm)
}

pub fn parse_quest_from_file(path: &Path) -> Result<Quest> {
    let f = File::open(path).map_err(|e| ParseError::Unexpected(e.to_string()))?;
    parse_quest_from_reader(f)
}

pub fn parse_quest_from_value(v: &Value) -> Result<Quest> {
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

    let tasks = parse_tasks(obj.get("tasks"));
    let rewards = parse_rewards(obj.get("rewards"));

    let prerequisites = if let Some(pre) = obj.get("preRequisites") {
        match pre {
            Value::Object(map) => {
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
            }
            Value::Array(arr) => arr
                .iter()
                .filter_map(|v| {
                    v.as_object().map(|m| QuestId {
                        high: get_i32(m, "questIDHigh").unwrap_or(0),
                        low: get_i32(m, "questIDLow").unwrap_or(0),
                    })
                })
                .collect(),
            _ => vec![],
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

pub fn parse_properties(v: &Value) -> Result<Option<QuestProperties>> {
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

fn get_string_field(m: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    for &k in keys {
        if let Some(v) = m.get(k) {
            if let Some(s) = v.as_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn parse_items_vec(opt: Option<&Value>) -> Vec<ItemStack> {
    if let Some(v) = opt {
        match v {
            Value::Array(arr) => arr.iter().filter_map(|e| parse_item(e)).collect(),
            Value::Object(map) => {
                // try numeric-keyed map
                if let Some(vec) = map_to_array_if_numeric(map) {
                    return vec.into_iter().filter_map(|e| parse_item(&e)).collect();
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

fn parse_task_entry(idx: Option<usize>, v: &Value) -> Option<Task> {
    let map = v.as_object()?;
    // common possible field names for task id
    let task_id = get_string_field(map, &["taskID", "taskId", "task_id", "task"])?;
    let required_items = parse_items_vec(
        map.get("requiredItems")
            .or_else(|| map.get("requiredItems")),
    );

    // collect options: everything except known keys
    let mut options = HashMap::new();
    for (k, val) in map.iter() {
        if ["taskID", "taskId", "task_id", "task", "requiredItems"].contains(&k.as_str()) {
            continue;
        }
        options.insert(k.clone(), val.clone());
    }

    Some(Task {
        index: idx,
        task_id,
        required_items,
        options,
    })
}

fn parse_tasks(opt: Option<&Value>) -> Vec<Task> {
    if let Some(v) = opt {
        match v {
            Value::Array(arr) => arr
                .iter()
                .enumerate()
                .filter_map(|(i, e)| parse_task_entry(Some(i), e))
                .collect(),
            Value::Object(map) => {
                // try numeric keyed map -> preserve index
                let mut numeric_keys: std::collections::BTreeMap<usize, Value> =
                    std::collections::BTreeMap::new();
                for (k, val) in map.iter() {
                    if let Ok(idx) = k.parse::<usize>() {
                        numeric_keys.insert(idx, val.clone());
                    } else {
                        // not numeric keyed: try to parse as single task object
                        if let Some(t) = parse_task_entry(None, &Value::Object(map.clone())) {
                            return vec![t];
                        } else {
                            return vec![];
                        }
                    }
                }
                numeric_keys
                    .into_iter()
                    .filter_map(|(idx, val)| parse_task_entry(Some(idx), &val))
                    .collect()
            }
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

fn parse_reward_entry(idx: Option<usize>, v: &Value) -> Option<Reward> {
    let map = v.as_object()?;
    let reward_id = get_string_field(map, &["rewardID", "rewardId", "reward_id", "reward"])?;
    let items = parse_items_vec(
        map.get("items")
            .or_else(|| map.get("rewards"))
            .or_else(|| map.get("rewards")),
    );

    let mut extra = HashMap::new();
    for (k, val) in map.iter() {
        if [
            "rewardID",
            "rewardId",
            "reward_id",
            "reward",
            "items",
            "rewards",
        ]
        .contains(&k.as_str())
        {
            continue;
        }
        extra.insert(k.clone(), val.clone());
    }

    Some(Reward {
        index: idx,
        reward_id,
        items,
        extra,
    })
}

fn parse_rewards(opt: Option<&Value>) -> Vec<Reward> {
    if let Some(v) = opt {
        match v {
            Value::Array(arr) => arr
                .iter()
                .enumerate()
                .filter_map(|(i, e)| parse_reward_entry(Some(i), e))
                .collect(),
            Value::Object(map) => {
                let mut numeric_keys: std::collections::BTreeMap<usize, Value> =
                    std::collections::BTreeMap::new();
                for (k, val) in map.iter() {
                    if let Ok(idx) = k.parse::<usize>() {
                        numeric_keys.insert(idx, val.clone());
                    } else {
                        if let Some(r) = parse_reward_entry(None, &Value::Object(map.clone())) {
                            return vec![r];
                        } else {
                            return vec![];
                        }
                    }
                }
                numeric_keys
                    .into_iter()
                    .filter_map(|(idx, val)| parse_reward_entry(Some(idx), &val))
                    .collect()
            }
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

// File-system dependent tests belong in the integration test directory `tests/`.
