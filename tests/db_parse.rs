use std::fs::{create_dir_all, write};
use std::path::PathBuf;

use better_questing_tools::{ParseError, QuestId, db::parse_default_quests_dir};

fn mk_tmp_dir(suffix: &str) -> PathBuf {
    let mut base = std::env::temp_dir();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time");
    base.push(format!(
        "better_questing_tools_test_{}_{}",
        suffix,
        now.as_millis()
    ));
    base
}

#[test]
fn parse_default_quests_dir_success() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = mk_tmp_dir("success");
    let dq = tmp.join("DefaultQuests");
    create_dir_all(dq.join("Quests"))?;
    create_dir_all(dq.join("QuestLines").join("Line1"))?;

    let quest_json = r#"{
        "questIDHigh:4": 0,
        "questIDLow:4": 1,
        "properties:10": {"betterquesting:10": {"name:8": "Test Quest"}}
    }"#;
    write(dq.join("Quests").join("quest1.json"), quest_json)?;

    let qline_json = r#"{
       "properties:10": {"betterquesting:10": {"name:8": "Line1"}},
       "questLineIDHigh:4": 0,
       "questLineIDLow:4": 100
    }"#;
    write(
        dq.join("QuestLines").join("Line1").join("QuestLine.json"),
        qline_json,
    )?;

    let qline_entry = r#"{
        "questIDHigh:4": 0,
        "questIDLow:4": 1,
        "x:3": 10,
        "y:3": 20
    }"#;
    write(
        dq.join("QuestLines").join("Line1").join("entry1.json"),
        qline_entry,
    )?;

    let db = parse_default_quests_dir(&dq).expect("parse db");
    assert!(db.quests.contains_key(&QuestId { high: 0, low: 1 }));
    assert!(db.questlines.values().any(|ql| ql.entries.len() == 1));
    Ok(())
}

#[test]
fn parse_default_quests_dir_missing_reference() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = mk_tmp_dir("missing_ref");
    let dq = tmp.join("DefaultQuests");
    create_dir_all(dq.join("QuestLines").join("LineA"))?;

    let qline_json = r#"{
       "properties:10": {"betterquesting:10": {"name:8": "LineA"}},
       "questLineIDHigh:4": 0,
       "questLineIDLow:4": 200
    }"#;
    write(
        dq.join("QuestLines").join("LineA").join("QuestLine.json"),
        qline_json,
    )?;

    // entry references quest id (0,999) which does not exist
    let qline_entry = r#"{
        "questIDHigh:4": 0,
        "questIDLow:4": 999,
        "x:3": 5,
        "y:3": 6
    }"#;
    write(
        dq.join("QuestLines")
            .join("LineA")
            .join("entry_missing.json"),
        qline_entry,
    )?;

    let res = parse_default_quests_dir(&dq);
    assert!(res.is_err());
    match res.err().unwrap() {
        ParseError::MissingQuestReference {
            questline: _ql,
            quest_id: _qid,
        } => { /* expected */ }
        other => panic!("unexpected error: {:?}", other),
    }

    Ok(())
}
