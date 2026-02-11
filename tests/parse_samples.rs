use glob::glob;
use serde_json::Value;

use bq_viewer::parser::parse_quest_from_file;

#[test]
fn parse_all_sample_quests_snapshot() {
    let pattern = "samples/**/config/betterquesting/DefaultQuests/Quests/**/*.json";
    let mut snapshots: Vec<Value> = Vec::new();
    for entry in glob(pattern).expect("glob failed") {
        let path = entry.expect("glob entry");
        if path.is_file() {
            let quest = parse_quest_from_file(&path).expect("parse failed");
            // serialize the quest to json value for snapshotting
            let v = serde_json::to_value(&quest).expect("serialize failed");
            snapshots.push(v);
        }
    }
    // Use insta snapshot assertion for the collected quests
    insta::with_settings!({omit_expression => true}, {
        insta::assert_debug_snapshot!(snapshots);
    });
}
