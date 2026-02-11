use serde_json::Value;
use std::io::Read;
use std::path::PathBuf;
use std::{fs, io::Cursor};

use better_questing_tools::parser::parse_quest_from_reader;
use zip::ZipArchive;

#[test]
fn parse_all_sample_quests_snapshot() {
    // Locate a samples zip file. Try a few common locations (project root or samples/ directory).
    let zip_path: PathBuf = {
        // commonly used paths
        let candidates = ["samples.zip", "samples/samples.zip"];
        let mut found: Option<PathBuf> = None;
        for c in &candidates {
            let p = PathBuf::from(c);
            if p.is_file() {
                found = Some(p);
                break;
            }
        }
        // If none of the above worked, scan the `samples` directory for the first .zip file.
        // This keeps the test flexible to where CI/tooling places the downloaded archive.
        if found.is_none() {
            if let Ok(entries) = fs::read_dir("samples") {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.extension().and_then(|s| s.to_str()) == Some("zip") {
                        found = Some(p);
                        break;
                    }
                }
            }
        }
        found.expect("could not find samples zip file; expected samples.zip or samples/*.zip")
    };

    // Read the zip fully into memory (do not extract to filesystem).
    let data = fs::read(&zip_path).expect("failed to read samples zip");
    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor).expect("failed to open zip archive");

    let mut snapshots: Vec<Value> = Vec::new();

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

    // Use insta snapshot assertion for the collected quests
    insta::with_settings!({omit_expression => true}, {
        insta::assert_debug_snapshot!(snapshots);
    });
}
