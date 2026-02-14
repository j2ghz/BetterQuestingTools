use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;
use std::{collections::HashMap, fs, io::Cursor};

use better_questing_tools::importance::compute_importance_scores;
use better_questing_tools::model::{Quest, QuestDatabase, QuestId};
use better_questing_tools::parser::parse_quest_from_reader;
use zip::ZipArchive;

#[test]
fn importance_on_db_snapshot() {
    // Collect quests from any zip in samples/ that contains DefaultQuests/Quests/*.json
    let samples_dir = PathBuf::from("samples");
    let entries = fs::read_dir(&samples_dir).expect("failed to read samples directory");

    let mut quests: HashMap<QuestId, Quest> = HashMap::new();

    for entry in entries.flatten() {
        let zip_path = entry.path();
        if zip_path.extension().and_then(|s| s.to_str()) != Some("zip") {
            continue;
        }

        let data = fs::read(&zip_path).expect("failed to read samples zip");
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor).expect("failed to open zip archive");

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).expect("failed to access zip entry");
            let name = file.name().to_string();

            if !name.ends_with(".json") {
                continue;
            }
            if !name.contains("config/betterquesting/DefaultQuests/Quests/") {
                continue;
            }

            let mut buf: Vec<u8> = Vec::new();
            file.read_to_end(&mut buf)
                .expect("failed to read zip entry");
            let cursor = Cursor::new(buf);
            let quest = parse_quest_from_reader(cursor).expect("parse failed");
            // prefer first-seen on duplicates
            quests.entry(quest.id.clone()).or_insert(quest);
        }
    }

    let db = QuestDatabase {
        settings: None,
        quests,
        questlines: HashMap::new(),
        questline_order: vec![],
    };

    // compute scores and produce a compact, deterministic snapshot
    let scores = compute_importance_scores(&db, 0.25, true, true).expect("compute scores");

    let mut vec: Vec<(u64, f64)> = scores
        .into_iter()
        .map(|(qid, s)| {
            (qid.as_u64(), {
                // round to 12 decimal places for snapshot stability
                let r = (s * 1e12).round() / 1e12;
                if r == -0.0 {
                    0.0
                } else {
                    r
                }
            })
        })
        .collect();

    // sort by score desc, tie-breaker by id asc
    vec.sort_by(
        |(a_id, a_s), (b_id, b_s)| match b_s.partial_cmp(a_s).unwrap() {
            std::cmp::Ordering::Equal => a_id.cmp(b_id),
            ord => ord,
        },
    );

    insta::with_settings!({omit_expression => true}, {
        insta::assert_debug_snapshot!(vec);
    });
}
