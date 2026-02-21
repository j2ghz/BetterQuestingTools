use better_questing_tools::parser::parse_quest_from_reader;
use insta::assert_json_snapshot;
use std::path::PathBuf;
use std::{fs, io::Cursor};
use zip::ZipArchive;

#[test]
fn snapshot_parse_all_sample_quests() {
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
            let file = archive.by_index(i).expect("failed to access zip entry");
            let name = file.name().to_string();

            // Only consider JSON quest files in the DefaultQuests/Quests path.
            if !name.ends_with(".json") {
                continue;
            }
            if !name.contains("config/betterquesting/DefaultQuests/Quests/") {
                continue;
            }

            let quest = parse_quest_from_reader(file).expect("parse failed");
            // serialize the quest to json value for snapshotting
            let quest = serde_json::to_value(&quest).expect("serialize failed");
            insta::with_settings!({
                snapshot_path => "snapshots/quests",
                snapshot_suffix => format!("{}/{}",zip_path.display(), name)},
            {
                assert_json_snapshot!(quest);
            });
        }
    }
}
