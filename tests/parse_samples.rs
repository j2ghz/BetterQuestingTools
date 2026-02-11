use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;
use std::{fs, io::Cursor};

use better_questing_tools::parser::parse_quest_from_reader;
use zip::ZipArchive;

#[test]
fn parse_all_sample_quests_snapshot() {
    let mut snapshots: Vec<Value> = Vec::new();

    // Use every .zip file found in the samples/ directory (read and process entirely in-memory).
    let samples_dir = PathBuf::from("samples");
    let entries = fs::read_dir(&samples_dir).expect("failed to read samples directory");

    for entry in entries.flatten() {
        let zip_path = entry.path();
        if zip_path.extension().and_then(|s| s.to_str()) != Some("zip") {
            continue;
        }

        // Read the zip fully into memory (do not extract to filesystem).
        let data = fs::read(&zip_path).expect("failed to read samples zip");
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor).expect("failed to open zip archive");

        // Iterate entries in the zip and pick JSON files under the expected path inside the archive.
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).expect("failed to access zip entry");
            let name = file.name().to_string();

            // Only consider JSON quest files in the DefaultQuests/Quests path.
            if !name.ends_with(".json") {
                continue;
            }
            if !name.contains("config/betterquesting/DefaultQuests/Quests/") {
                continue;
            }

            // Read the file contents into memory and parse using the existing reader-based API.
            let mut buf: Vec<u8> = Vec::new();
            file.read_to_end(&mut buf)
                .expect("failed to read zip entry");
            let cursor = Cursor::new(buf);
            let quest = parse_quest_from_reader(cursor).expect("parse failed");
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
