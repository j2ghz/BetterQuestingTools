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
    /// Quest name (if present).
    pub name: Option<String>,
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
