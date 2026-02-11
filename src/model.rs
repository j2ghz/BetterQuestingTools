use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quest {
    pub id: QuestId,
    pub properties: Option<QuestProperties>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub rewards: Vec<Reward>,
    #[serde(default)]
    pub prerequisites: Vec<QuestId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestId {
    pub high: i32,
    pub low: i32,
}

impl QuestId {
    pub fn as_u64(&self) -> u64 {
        ((self.high as i64 as u64) << 32) | (self.low as u64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestProperties {
    pub name: Option<String>,
    pub desc: Option<String>,
    pub icon: Option<ItemStack>,
    pub is_main: Option<bool>,
    pub is_silent: Option<bool>,
    pub auto_claim: Option<bool>,
    pub quest_logic: Option<String>,
    pub task_logic: Option<String>,
    pub visibility: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub id: String,
    pub damage: Option<i32>,
    pub count: Option<i32>,
    pub oredict: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub index: Option<usize>,
    pub task_id: String,
    #[serde(default)]
    pub required_items: Vec<ItemStack>,
    #[serde(flatten)]
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reward {
    pub index: Option<usize>,
    pub reward_id: String,
    #[serde(default)]
    pub items: Vec<ItemStack>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
