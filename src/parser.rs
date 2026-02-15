use crate::error::{ParseError, Result};
use crate::model::*;
use crate::nbt_norm::{map_to_array_if_numeric, normalize_value};
use crate::quest_id::QuestId;
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
    let f = File::open(path)?;
    parse_quest_from_reader(f)
}

pub fn parse_quest_from_value(v: &Value) -> Result<Quest> {
    let obj = v
        .as_object()
        .ok_or_else(|| ParseError::InvalidFormat("root not an object".into()))?;

    let high = get_i32(obj, "questIDHigh").unwrap_or(0);
    let low = get_i32(obj, "questIDLow").unwrap_or(0);
    let id = QuestId::from_parts(high, low);

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

    // Parse all prerequisites into a flat list first.
    let mut all_prereqs: Vec<QuestId> = Vec::new();
    if let Some(pre) = obj.get("preRequisites") {
        match pre {
            Value::Object(map) => {
                if let Some(vec) = map_to_array_if_numeric(map) {
                    for v in vec.into_iter() {
                        if let Some(m) = v.as_object() {
                            all_prereqs.push(QuestId::from_parts(
                                get_i32(m, "questIDHigh").unwrap_or(0),
                                get_i32(m, "questIDLow").unwrap_or(0),
                            ));
                        }
                    }
                }
            }
            Value::Array(arr) => {
                for v in arr.iter() {
                    if let Some(m) = v.as_object() {
                        all_prereqs.push(QuestId::from_parts(
                            get_i32(m, "questIDHigh").unwrap_or(0),
                            get_i32(m, "questIDLow").unwrap_or(0),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    let mut optional_prereqs: Vec<QuestId> = Vec::new();
    if let Some(pre) = obj.get("optionalPreRequisites") {
        match pre {
            Value::Object(map) => {
                if let Some(vec) = map_to_array_if_numeric(map) {
                    for v in vec.into_iter() {
                        if let Some(m) = v.as_object() {
                            optional_prereqs.push(QuestId::from_parts(
                                get_i32(m, "questIDHigh").unwrap_or(0),
                                get_i32(m, "questIDLow").unwrap_or(0),
                            ));
                        }
                    }
                }
            }
            Value::Array(arr) => {
                for v in arr.iter() {
                    if let Some(m) = v.as_object() {
                        optional_prereqs.push(QuestId::from_parts(
                            get_i32(m, "questIDHigh").unwrap_or(0),
                            get_i32(m, "questIDLow").unwrap_or(0),
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    // Decide which prereqs are required vs optional. If an explicit optional
    // list is present, remove those from the required set. Otherwise, consult
    // the quest logic (OR/XOR means the list should be treated as optional).
    let mut required_prereqs: Vec<QuestId> = Vec::new();
    if !optional_prereqs.is_empty() {
        let optset: std::collections::HashSet<u64> =
            optional_prereqs.iter().map(|q| q.as_u64()).collect();
        for q in all_prereqs.into_iter() {
            if !optset.contains(&q.as_u64()) {
                required_prereqs.push(q);
            }
        }
    } else {
        let is_or = properties
            .as_ref()
            .and_then(|p| p.quest_logic.as_ref())
            .map(|s| s.to_uppercase())
            .map(|s| s == "OR" || s == "ONE_OF" || s == "ANY" || s == "XOR")
            .unwrap_or(false);
        if is_or {
            optional_prereqs = all_prereqs;
        } else {
            required_prereqs = all_prereqs;
        }
    }

    Ok(Quest {
        id,
        properties,
        tasks,
        rewards,
        prerequisites: required_prereqs.clone(),
        required_prerequisites: required_prereqs,
        optional_prerequisites: optional_prereqs,
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
    let global_share = map.get("globalShare").and_then(parse_bool_like);
    let is_global = map.get("isGlobal").and_then(parse_bool_like);
    let locked_progress = map
        .get("lockedProgress")
        .and_then(|x| x.as_i64().map(|n| n as i32));
    let repeat_time = map
        .get("repeatTime")
        .and_then(|x| x.as_i64().map(|n| n as i32));
    let repeat_relative = map.get("repeat_relative").and_then(parse_bool_like);
    let simultaneous = map.get("simultaneous").and_then(parse_bool_like);
    let party_single_reward = map.get("partySingleReward").and_then(parse_bool_like);
    let quest_logic = map
        .get("questLogic")
        .and_then(|x| x.as_str().map(|s| s.to_string()));
    let task_logic = map
        .get("taskLogic")
        .and_then(|x| x.as_str().map(|s| s.to_string()));
    let visibility = map
        .get("visibility")
        .and_then(|x| x.as_str().map(|s| s.to_string()));
    let snd_complete = map
        .get("snd_complete")
        .and_then(|x| x.as_str().map(|s| s.to_string()));
    let snd_update = map
        .get("snd_update")
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
            "globalShare",
            "isGlobal",
            "lockedProgress",
            "repeatTime",
            "repeat_relative",
            "simultaneous",
            "partySingleReward",
            "questLogic",
            "taskLogic",
            "visibility",
            "snd_complete",
            "snd_update",
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
        global_share,
        is_global,
        locked_progress,
        repeat_time,
        repeat_relative,
        simultaneous,
        party_single_reward,
        quest_logic,
        task_logic,
        visibility,
        snd_complete,
        snd_update,
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

// parse_indexed_array_of was removed — parsing is handled by dedicated helpers

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

    // extract common flags into typed fields when present
    let ignore_nbt = map
        .get("ignoreNBT")
        .or_else(|| map.get("ignore_nbt"))
        .and_then(parse_bool_like);
    let partial_match = map
        .get("partialMatch")
        .or_else(|| map.get("partial_match"))
        .and_then(parse_bool_like);
    let auto_consume = map
        .get("autoConsume")
        .or_else(|| map.get("auto_consume"))
        .and_then(parse_bool_like);
    let consume = map
        .get("consume")
        .or_else(|| map.get("consume"))
        .and_then(parse_bool_like);
    let group_detect = map
        .get("groupDetect")
        .or_else(|| map.get("group_detect"))
        .and_then(parse_bool_like);

    // collect options: everything except known keys
    let mut options = HashMap::new();
    for (k, val) in map.iter() {
        if [
            "taskID",
            "taskId",
            "task_id",
            "task",
            "requiredItems",
            "ignoreNBT",
            "ignore_nbt",
            "partialMatch",
            "partial_match",
            "autoConsume",
            "auto_consume",
            "consume",
            "groupDetect",
            "group_detect",
        ]
        .contains(&k.as_str())
        {
            continue;
        }
        options.insert(k.clone(), val.clone());
    }

    Some(Task {
        index: idx,
        task_id,
        required_items,
        ignore_nbt,
        partial_match,
        auto_consume,
        consume,
        group_detect,
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
    let choices = parse_items_vec(map.get("choices"));
    let ignore_disabled = map
        .get("ignoreDisabled")
        .or_else(|| map.get("ignore_disabled"))
        .and_then(parse_bool_like);

    let mut extra = HashMap::new();
    for (k, val) in map.iter() {
        if [
            "rewardID",
            "rewardId",
            "reward_id",
            "reward",
            "items",
            "rewards",
            "choices",
            "ignoreDisabled",
            "ignore_disabled",
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
        choices,
        ignore_disabled,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_tasks_array_and_numeric() {
        // array form
        let tasks_val = json!([
            {
                "taskID": "bq_standard:retrieval",
                "requiredItems": [
                    {"id": "minecraft:iron_ingot", "Damage": 0, "Count": 1}
                ],
                "ignoreNBT": 0,
                "partialMatch": 1
            }
        ]);

        let tasks = parse_tasks(Some(&tasks_val));
        assert_eq!(tasks.len(), 1);
        let t = &tasks[0];
        assert_eq!(t.task_id, "bq_standard:retrieval");
        assert_eq!(t.required_items.len(), 1);
        assert_eq!(t.required_items[0].id, "minecraft:iron_ingot");
        // flags were pulled into typed fields
        assert_eq!(t.ignore_nbt, Some(false));
        assert_eq!(t.partial_match, Some(true));

        // numeric-keyed map form
        let tasks_obj = json!({
            "0": {
                "taskID": "bq_standard:retrieval",
                "requiredItems": {"0": {"id": "mod:item", "Count": 2}}
            }
        });
        let tasks2 = parse_tasks(Some(&tasks_obj));
        assert_eq!(tasks2.len(), 1);
        assert_eq!(tasks2[0].required_items.len(), 1);
        assert_eq!(tasks2[0].required_items[0].id, "mod:item");
    }

    #[test]
    fn parse_rewards_array_and_numeric() {
        let rewards_val = json!([
            {
                "rewardID": "bq_standard:item",
                "items": [
                    {"id": "minecraft:nether_star", "Count": 4}
                ],
                "ignoreDisabled": 0
            }
        ]);

        let rewards = parse_rewards(Some(&rewards_val));
        assert_eq!(rewards.len(), 1);
        assert_eq!(rewards[0].reward_id, "bq_standard:item");
        assert_eq!(rewards[0].items.len(), 1);
        assert_eq!(rewards[0].items[0].id, "minecraft:nether_star");

        let rewards_obj = json!({
            "0": {
                "rewardID": "bq_standard:item",
                "items": {"0": {"id": "mod:star", "Count": 1}}
            }
        });
        let rewards2 = parse_rewards(Some(&rewards_obj));
        assert_eq!(rewards2.len(), 1);
        assert_eq!(rewards2[0].items.len(), 1);
        assert_eq!(rewards2[0].items[0].id, "mod:star");
    }

    #[test]
    fn parse_item_with_tag_and_extras() {
        let item = json!({
            "id": "Thaumcraft:WandCasting",
            "Count": 1,
            "Damage": 128,
            "tag": {
                "aer": 15000,
                "cap": "thaumium",
                "AttributeModifiers": {"0": {"Amount": 6.0, "AttributeName": "generic.attackDamage"}}
            }
        });

        let parsed = parse_item(&item).expect("parsed item");
        assert_eq!(parsed.id, "Thaumcraft:WandCasting");
        assert_eq!(parsed.count, Some(1));
        assert_eq!(parsed.damage, Some(128));
        assert!(parsed.extra.contains_key("tag"));
    }
}
