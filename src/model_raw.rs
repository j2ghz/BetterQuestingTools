use serde::de::{self, Deserializer};

fn bool_from_int<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Bool(b) => Ok(Some(b)),
        serde_json::Value::Number(n) => match n.as_i64() {
            Some(0) => Ok(Some(false)),
            Some(1) => Ok(Some(true)),
            _ => Err(de::Error::custom("invalid int for bool")),
        },
        serde_json::Value::Null => Ok(None),
        _ => Err(de::Error::custom("invalid type for bool")),
    }
}
// Raw models for deserializing the original quest JSON structure as closely as possible.
// These are not optimized for library use, but match the input format for serde.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawQuest {
    #[serde(rename = "questIDHigh")]
    pub quest_id_high: Option<i64>,
    #[serde(rename = "questIDLow")]
    pub quest_id_low: Option<i64>,
    pub properties: Option<RawPropertiesWrapper>,
    pub tasks: Option<RawTasksWrapper>,
    pub rewards: Option<RawRewardsWrapper>,
    #[serde(rename = "preRequisites")]
    pub pre_requisites: Option<RawQuestRefs>,
    #[serde(rename = "optionalPreRequisites")]
    pub optional_pre_requisites: Option<RawQuestRefs>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawPropertiesWrapper {
    #[serde(rename = "betterquesting")]
    pub betterquesting: Option<RawQuestProperties>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawQuestProperties {
    pub name: String,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub icon: Option<serde_json::Value>,
    #[serde(rename = "isMain", default, deserialize_with = "bool_from_int")]
    pub is_main: Option<bool>,
    #[serde(rename = "isSilent", default, deserialize_with = "bool_from_int")]
    pub is_silent: Option<bool>,
    #[serde(rename = "autoClaim", default, deserialize_with = "bool_from_int")]
    pub auto_claim: Option<bool>,
    #[serde(rename = "globalShare", default, deserialize_with = "bool_from_int")]
    pub global_share: Option<bool>,
    #[serde(rename = "isGlobal", default, deserialize_with = "bool_from_int")]
    pub is_global: Option<bool>,
    #[serde(rename = "lockedProgress", default)]
    pub locked_progress: Option<i32>,
    #[serde(rename = "repeatTime", default)]
    pub repeat_time: Option<i32>,
    #[serde(default, deserialize_with = "bool_from_int")]
    pub repeat_relative: Option<bool>,
    #[serde(default, deserialize_with = "bool_from_int")]
    pub simultaneous: Option<bool>,
    #[serde(
        rename = "partySingleReward",
        default,
        deserialize_with = "bool_from_int"
    )]
    pub party_single_reward: Option<bool>,
    #[serde(rename = "questLogic", default)]
    pub quest_logic: Option<String>,
    #[serde(rename = "taskLogic", default)]
    pub task_logic: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub snd_complete: Option<String>,
    #[serde(default)]
    pub snd_update: Option<String>,
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RawTasksWrapper {
    Object(HashMap<String, serde_json::Value>),
    Array(Vec<serde_json::Value>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RawRewardsWrapper {
    Object(HashMap<String, serde_json::Value>),
    Array(Vec<serde_json::Value>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RawQuestRefs {
    Object(HashMap<String, serde_json::Value>),
    Array(Vec<serde_json::Value>),
}
