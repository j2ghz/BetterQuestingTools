use crate::error::Result;
use crate::model_raw::RawQuest;
impl Quest {
    /// Convert a RawQuest (serde-deserialized) into the optimized Quest model.
    pub fn from_raw(raw: RawQuest) -> Result<Self> {
        // Extract quest id
        let id = QuestId::from_parts(
            raw.quest_id_high.unwrap_or(0) as i32,
            raw.quest_id_low.unwrap_or(0) as i32,
        );

        // Build a normalized view of top-level extra fields (strip NBT suffixes and convert numeric maps->arrays)
        let normalized_extra_opt: Option<serde_json::Map<String, serde_json::Value>> =
            if !raw.extra.is_empty() {
                let mut m = serde_json::Map::new();
                for (k, v) in raw.extra.iter() {
                    m.insert(k.clone(), v.clone());
                }
                match crate::nbt_norm::normalize_value(serde_json::Value::Object(m)) {
                    serde_json::Value::Object(obj) => Some(obj),
                    _ => None,
                }
            } else {
                None
            };

        // Properties: extract strongly typed betterquesting block
        fn convert_raw_props(props: &crate::model_raw::RawQuestProperties) -> QuestProperties {
            QuestProperties {
                name: props.name.clone(),
                desc: props.desc.clone(),
                icon: None, // TODO: parse icon if needed
                is_main: props.is_main,
                is_silent: props.is_silent,
                auto_claim: props.auto_claim,
                global_share: props.global_share,
                is_global: props.is_global,
                locked_progress: props.locked_progress,
                repeat_time: props.repeat_time,
                repeat_relative: props.repeat_relative,
                simultaneous: props.simultaneous,
                party_single_reward: props.party_single_reward,
                quest_logic: props.quest_logic.clone(),
                task_logic: props.task_logic.clone(),
                visibility: props.visibility.clone(),
                snd_complete: props.snd_complete.clone(),
                snd_update: props.snd_update.clone(),
                extra: props.extra.clone(),
            }
        }

        // Try wrapped betterquesting first; otherwise attempt to extract from the extra map (with normalization)
        let properties: Option<QuestProperties> = if let Some(wrapper) = raw.properties.as_ref() {
            if let Some(props) = wrapper.betterquesting.as_ref() {
                Some(convert_raw_props(props))
            } else if !wrapper.extra.is_empty() {
                // Convert the HashMap into a serde_json::Map and normalize it so keys like "betterquesting:8" become "betterquesting"
                let mut m = serde_json::Map::new();
                for (k, v) in wrapper.extra.iter() {
                    m.insert(k.clone(), v.clone());
                }
                let norm = crate::nbt_norm::normalize_value(serde_json::Value::Object(m));
                if let serde_json::Value::Object(obj) = norm {
                    if let Some(bqv) = obj.get("betterquesting") {
                        let bq_norm = crate::nbt_norm::normalize_value(bqv.clone());
                        if let Ok(rp) =
                            serde_json::from_value::<crate::model_raw::RawQuestProperties>(bq_norm)
                        {
                            Some(convert_raw_props(&rp))
                        } else {
                            None
                        }
                    } else if let Some((_k, inner)) = obj.iter().next() {
                        let inner_norm = crate::nbt_norm::normalize_value(inner.clone());
                        if let Ok(rp) = serde_json::from_value::<crate::model_raw::RawQuestProperties>(
                            inner_norm,
                        ) {
                            Some(convert_raw_props(&rp))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            // Fallback: look inside normalized top-level extra for a "properties" key
            if let Some(obj) = normalized_extra_opt.as_ref() {
                if let Some(prop_val) = obj.get("properties") {
                    let prop_norm = crate::nbt_norm::normalize_value(prop_val.clone());
                    if let serde_json::Value::Object(prop_obj) = prop_norm {
                        if let Some(bqv) = prop_obj.get("betterquesting") {
                            let bq_norm = crate::nbt_norm::normalize_value(bqv.clone());
                            if let Ok(rp) = serde_json::from_value::<
                                crate::model_raw::RawQuestProperties,
                            >(bq_norm)
                            {
                                Some(convert_raw_props(&rp))
                            } else {
                                None
                            }
                        } else if let Some((_k, inner)) = prop_obj.iter().next() {
                            let inner_norm = crate::nbt_norm::normalize_value(inner.clone());
                            if let Ok(rp) = serde_json::from_value::<
                                crate::model_raw::RawQuestProperties,
                            >(inner_norm)
                            {
                                Some(convert_raw_props(&rp))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };
        let properties = match properties {
            Some(p) => Some(p),
            None => {
                return Err(crate::error::ParseError::InvalidFormat(
                    "Quest is missing a name property".to_string(),
                ));
            }
        };

        // Tasks and rewards: flatten and parse
        // Build a normalized view of raw.extra (keys with NBT suffixes are stripped)
        let normalized_extra_opt: Option<serde_json::Map<String, serde_json::Value>> =
            if !raw.extra.is_empty() {
                let mut m = serde_json::Map::new();
                for (k, v) in raw.extra.iter() {
                    m.insert(k.clone(), v.clone());
                }
                match crate::nbt_norm::normalize_value(serde_json::Value::Object(m)) {
                    serde_json::Value::Object(obj) => Some(obj),
                    _ => None,
                }
            } else {
                None
            };

        // Tasks: prefer explicit Raw.tasks, otherwise look in normalized extra for a "tasks" key
        let mut tasks: Vec<Task> = Vec::new();
        let tasks_wrapper_opt: Option<crate::model_raw::RawTasksWrapper> =
            raw.tasks.clone().or_else(|| {
                normalized_extra_opt.as_ref().and_then(|obj| {
                    obj.get("tasks").and_then(|val| match val {
                        serde_json::Value::Array(arr) => {
                            Some(crate::model_raw::RawTasksWrapper::Array(arr.clone()))
                        }
                        serde_json::Value::Object(map) => {
                            let mut hm: std::collections::HashMap<String, serde_json::Value> =
                                std::collections::HashMap::new();
                            for (k, v) in map.iter() {
                                hm.insert(k.clone(), v.clone());
                            }
                            Some(crate::model_raw::RawTasksWrapper::Object(hm))
                        }
                        _ => None,
                    })
                })
            });

        if let Some(tasks_wrapper) = tasks_wrapper_opt {
            match tasks_wrapper {
                crate::model_raw::RawTasksWrapper::Array(arr) => {
                    for (i, v) in arr.into_iter().enumerate() {
                        let v_norm = crate::nbt_norm::normalize_value(v);
                        if let Ok(mut t) = serde_json::from_value::<Task>(v_norm) {
                            t.index = Some(i);
                            tasks.push(t);
                        }
                    }
                }
                crate::model_raw::RawTasksWrapper::Object(obj) => {
                    // convert HashMap -> serde_json::Map then normalize the full object; it may become an array
                    let mut m = serde_json::Map::new();
                    for (k, v) in obj.into_iter() {
                        m.insert(k, v);
                    }
                    let norm = crate::nbt_norm::normalize_value(serde_json::Value::Object(m));
                    if let serde_json::Value::Array(arr2) = norm {
                        for (i, v) in arr2.into_iter().enumerate() {
                            let v_norm = crate::nbt_norm::normalize_value(v);
                            if let Ok(mut t) = serde_json::from_value::<Task>(v_norm) {
                                t.index = Some(i);
                                tasks.push(t);
                            }
                        }
                    }
                }
            }
        }

        // Rewards: prefer explicit Raw.rewards, otherwise look in normalized extra for a "rewards" key
        let mut rewards: Vec<Reward> = Vec::new();
        let rewards_wrapper_opt: Option<crate::model_raw::RawRewardsWrapper> =
            raw.rewards.clone().or_else(|| {
                normalized_extra_opt.as_ref().and_then(|obj| {
                    obj.get("rewards").and_then(|val| match val {
                        serde_json::Value::Array(arr) => {
                            Some(crate::model_raw::RawRewardsWrapper::Array(arr.clone()))
                        }
                        serde_json::Value::Object(map) => {
                            let mut hm: std::collections::HashMap<String, serde_json::Value> =
                                std::collections::HashMap::new();
                            for (k, v) in map.iter() {
                                hm.insert(k.clone(), v.clone());
                            }
                            Some(crate::model_raw::RawRewardsWrapper::Object(hm))
                        }
                        _ => None,
                    })
                })
            });

        if let Some(rewards_wrapper) = rewards_wrapper_opt {
            match rewards_wrapper {
                crate::model_raw::RawRewardsWrapper::Array(arr) => {
                    for (i, v) in arr.into_iter().enumerate() {
                        let v_norm = crate::nbt_norm::normalize_value(v);
                        if let Ok(mut r) = serde_json::from_value::<Reward>(v_norm) {
                            r.index = Some(i);
                            rewards.push(r);
                        }
                    }
                }
                crate::model_raw::RawRewardsWrapper::Object(obj) => {
                    let mut m = serde_json::Map::new();
                    for (k, v) in obj.into_iter() {
                        m.insert(k, v);
                    }
                    let norm = crate::nbt_norm::normalize_value(serde_json::Value::Object(m));
                    if let serde_json::Value::Array(arr2) = norm {
                        for (i, v) in arr2.into_iter().enumerate() {
                            let v_norm = crate::nbt_norm::normalize_value(v);
                            if let Ok(mut r) = serde_json::from_value::<Reward>(v_norm) {
                                r.index = Some(i);
                                rewards.push(r);
                            }
                        }
                    }
                }
            }
        }

        // Prerequisites
        fn parse_prereqs(val: Option<crate::model_raw::RawQuestRefs>) -> Vec<QuestId> {
            let mut out = Vec::new();
            if let Some(wrapper) = val {
                match wrapper {
                    crate::model_raw::RawQuestRefs::Object(inner) => {
                        for (_k, v) in inner {
                            // normalize individual prereq object before inspecting fields
                            let v_norm = crate::nbt_norm::normalize_value(v.clone());
                            if let serde_json::Value::Object(obj_map) = v_norm {
                                let high = obj_map
                                    .get("questIDHigh")
                                    .and_then(|x| x.as_i64())
                                    .unwrap_or(0) as i32;
                                let low = obj_map
                                    .get("questIDLow")
                                    .and_then(|x| x.as_i64())
                                    .unwrap_or(0) as i32;
                                out.push(QuestId::from_parts(high, low));
                            }
                        }
                    }
                    crate::model_raw::RawQuestRefs::Array(arr) => {
                        for elem in arr {
                            let elem_norm = crate::nbt_norm::normalize_value(elem);
                            if let serde_json::Value::Object(obj_map) = elem_norm {
                                let high = obj_map
                                    .get("questIDHigh")
                                    .and_then(|x| x.as_i64())
                                    .unwrap_or(0) as i32;
                                let low = obj_map
                                    .get("questIDLow")
                                    .and_then(|x| x.as_i64())
                                    .unwrap_or(0) as i32;
                                out.push(QuestId::from_parts(high, low));
                            }
                        }
                    }
                }
            }
            out
        }

        let all_prereqs = parse_prereqs(raw.pre_requisites);
        let mut optional_prereqs = parse_prereqs(raw.optional_pre_requisites);

        // Decide which prereqs are required vs optional
        let mut required_prereqs = Vec::new();
        if !optional_prereqs.is_empty() {
            let optset: std::collections::HashSet<u64> = optional_prereqs
                .iter()
                .map(|q: &QuestId| q.as_u64())
                .collect();
            for q in all_prereqs.iter() {
                if !optset.contains(&q.as_u64()) {
                    required_prereqs.push(*q);
                }
            }
        } else {
            // Always check for quest_logic, but if not present, treat all as required
            let is_or = properties
                .as_ref()
                .and_then(|p: &QuestProperties| p.quest_logic.as_ref())
                .map(|s: &String| s.to_uppercase())
                .map(|s: String| s == "OR" || s == "ONE_OF" || s == "ANY" || s == "XOR")
                .unwrap_or(false);
            if is_or {
                optional_prereqs = all_prereqs.clone();
            } else {
                required_prereqs = all_prereqs.clone();
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
}
use crate::quest_id::QuestId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A parsed Quest object.
///
/// Contains the canonical quest identifier (`id`), optional `properties` with
/// user-facing metadata, a list of `tasks` and `rewards`, and any
/// `prerequisites` (references to other quests).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quest {
    /// Unique identifier for this quest.
    pub id: QuestId,
    /// High-level properties (name, description, icon and flags).
    pub properties: Option<QuestProperties>,
    /// Task entries for this quest.
    #[serde(default)]
    pub tasks: Vec<Task>,
    /// Reward entries for this quest.
    #[serde(default)]
    pub rewards: Vec<Reward>,
    /// Other quests that must be completed before this one.
    #[serde(default)]
    pub prerequisites: Vec<QuestId>,
    /// Required prerequisites (explicitly marked required by the source data).
    /// This is populated by the parser when the input distinguishes required vs
    /// optional prereqs. If empty, callers should fall back to `prerequisites`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_prerequisites: Vec<QuestId>,
    /// Optional prerequisites (alternatives / one-of groups). We flatten groups
    /// to a single vector; weight distribution is handled by the importance
    /// algorithm.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional_prerequisites: Vec<QuestId>,
}

/// Human-visible properties for a quest.
///
/// Unknown or extension fields are preserved in the `extra` map so callers can
/// round-trip or inspect unmodeled data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestProperties {
    /// Quest name (required).
    pub name: String,
    /// Short description or lore text.
    pub desc: Option<String>,
    /// Icon item for display purposes.
    pub icon: Option<ItemStack>,
    /// Is this quest considered a main quest?
    pub is_main: Option<bool>,
    /// Should quest progress be silent (no notifications)?
    pub is_silent: Option<bool>,
    /// Should rewards be auto-claimed?
    pub auto_claim: Option<bool>,
    /// Is the quest shared globally across players/world (often 0/1 in source)
    pub global_share: Option<bool>,
    /// Is this quest marked global (mirror of globalShare in some datasets)
    pub is_global: Option<bool>,
    /// Lock progress flag (numeric in source)
    pub locked_progress: Option<i32>,
    /// Repeat time in ticks/seconds (numeric)
    pub repeat_time: Option<i32>,
    /// Repeat relative flag (0/1)
    pub repeat_relative: Option<bool>,
    /// Allow simultaneous completion (0/1)
    pub simultaneous: Option<bool>,
    /// Whether party distributes single reward (0/1)
    pub party_single_reward: Option<bool>,
    /// Raw quest logic identifier (e.g. "AND"/"OR").
    pub quest_logic: Option<String>,
    /// Raw per-task logic identifier.
    pub task_logic: Option<String>,
    /// Visibility hint for UIs (string preserved as-is).
    pub visibility: Option<String>,
    /// Optional completion / update sound identifiers
    pub snd_complete: Option<String>,
    pub snd_update: Option<String>,
    /// Extra unknown fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Simplified ItemStack representation used in tasks/rewards/icons.
///
/// We intentionally keep a small, common subset of item fields (id, damage,
/// count, oredict) and preserve everything else in `extra` so the parser stays
/// tolerant of mod-specific data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    /// Item identifier (namespaced id like "minecraft:stone").
    pub id: String,
    /// Optional damage / meta value.
    pub damage: Option<i32>,
    /// Optional stack count.
    pub count: Option<i32>,
    /// Ore dictionary name if present.
    pub oredict: Option<String>,
    /// Any additional, unmodeled NBT/json data.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A quest Task entry.
///
/// `task_id` identifies the task implementation/type (plugins will vary). The
/// `required_items` vector holds ItemStacks required to complete the task. Any
/// task-specific options are kept in `options`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// Optional index within the containing quest or questline ordering.
    pub index: Option<usize>,
    /// Canonical identifier for the task implementation.
    pub task_id: String,
    /// Items required by this task (if applicable).
    #[serde(default)]
    pub required_items: Vec<ItemStack>,
    /// Common boolean-like flags found on many task types.
    pub ignore_nbt: Option<bool>,
    pub partial_match: Option<bool>,
    pub auto_consume: Option<bool>,
    pub consume: Option<bool>,
    pub group_detect: Option<bool>,
    /// Task-specific or unknown fields.
    #[serde(flatten)]
    pub options: HashMap<String, serde_json::Value>,
}

/// A quest Reward entry (items / commands / scripted rewards).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reward {
    /// Optional index within the containing quest.
    pub index: Option<usize>,
    /// Identifier for the reward type/handler.
    pub reward_id: String,
    /// Items granted by this reward (if any).
    #[serde(default)]
    pub items: Vec<ItemStack>,
    /// Alternative choices for choice-type rewards.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<ItemStack>,
    /// Common boolean-like flag indicating whether disabled rewards are ignored.
    pub ignore_disabled: Option<bool>,
    /// Any unknown or additional fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A QuestLine groups quests for UI presentation (layout, title and ordering).
///
/// QuestLines are typically directories containing a `QuestLine.json` and a
/// collection of entry files that reference quests by id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestLine {
    /// Identifier for the line (also stored as a questline id pair).
    pub id: QuestId,
    /// Optional properties for the line (title, icon, visibility, ...).
    pub properties: Option<QuestProperties>,
    /// Entries (positions) on the line.
    #[serde(default)]
    pub entries: Vec<QuestLineEntry>,
    /// Unknown or extension fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A single entry inside a `QuestLine` describing the layout of a quest tile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestLineEntry {
    /// Optional ordering index.
    pub index: Option<usize>,
    /// The referenced quest id.
    pub quest_id: QuestId,
    /// X coordinate in the questline layout.
    pub x: Option<i32>,
    /// Y coordinate in the questline layout.
    pub y: Option<i32>,
    /// Width of the tile.
    pub size_x: Option<i32>,
    /// Height of the tile.
    pub size_y: Option<i32>,
    /// Additional unmodeled fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Global settings for the DefaultQuests dataset (contains version and other
/// gameplay/display flags).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestSettings {
    /// Optional version string found in settings (useful for format compatibility).
    pub version: Option<String>,
    /// Any additional settings preserved verbatim.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Aggregated parsed representation of an entire `DefaultQuests` folder.
///
/// `QuestDatabase` ties together parsed quests, questlines and the global
/// settings. In strict mode (current behavior) references inside questlines are
/// validated and will cause parsing to fail if dangling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestDatabase {
    /// Optional global settings (may be absent).
    pub settings: Option<QuestSettings>,
    /// Map of quests by their `QuestId`.
    pub quests: HashMap<QuestId, Quest>,
    /// Parsed questlines keyed by their `QuestId`.
    pub questlines: HashMap<QuestId, QuestLine>,
    /// Ordering of questlines (useful for UI presentation).
    pub questline_order: Vec<QuestId>,
}
