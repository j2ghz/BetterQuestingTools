use std::io::Read;
use std::path::PathBuf;
use std::{collections::HashMap, fs, io::Cursor};

use better_questing_tools::model::Quest;
use better_questing_tools::parser::parse_quest_from_reader;
use better_questing_tools::quest_id::QuestId;
use zip::ZipArchive;

/// Test to generate a Graphviz (DOT) diagram of the quests and their prerequisite edges.
#[test]
fn quest_graph_dot_snapshot() {
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
            quests.entry(quest.id).or_insert(quest);
        }
    }

    // Change quests HashMap to a sorted Vec for deterministic output
    let mut quest_vec: Vec<_> = quests.iter().collect();
    quest_vec.sort_by_key(|(qid, _)| *qid);

    /// Strips Minecraft-style formatting codes (e.g., §b§l) from quest names.
    /// Currently removes these codes for readability; future work could parse and display them for styled output.
    fn strip_minecraft_format_codes(text: &str) -> String {
        // Remove sequences of '§' followed by one character (color/font styling)
        let mut result = String::new();
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '§' {
                // Skip the next character (format code)
                chars.next();
            } else {
                result.push(c);
            }
        }
        result
    }

    // DOT header
    let mut dot = String::from("digraph quests {\n");
    // Add all nodes: id [label="name (#id)"]
    for (qid, quest) in &quest_vec {
        let label = if let Some(props) = &quest.properties {
            if let Some(name) = &props.name {
                // Remove Minecraft formatting codes from labels for debug/snapshot readability.
                // These codes can be parsed to display color/font styling in future if needed.
                format!(
                    "{} ({})",
                    strip_minecraft_format_codes(&name.replace('"', "\\\"")),
                    qid.as_u64()
                )
            } else {
                format!("{}", qid.as_u64())
            }
        } else {
            format!("{}", qid.as_u64())
        };
        dot.push_str(&format!("  {} [label=\"{}\"]\n", qid.as_u64(), label));
    }

    // Add edges for all prerequisites (including required and optional)
    for (qid, quest) in &quest_vec {
        // Exclude prerequisite edges for quests with quest_logic == "XOR"
        let is_xor = quest
            .properties
            .as_ref()
            .and_then(|props| props.quest_logic.as_deref())
            .is_some_and(|logic| logic.eq_ignore_ascii_case("XOR"));
        if is_xor {
            continue;
        }
        let src = qid.as_u64();
        let prereq_ids = if !quest.required_prerequisites.is_empty() {
            quest.required_prerequisites.iter()
        } else {
            quest.prerequisites.iter()
        };
        for target in prereq_ids.clone() {
            dot.push_str(&format!("  {} -> {}\n", target.as_u64(), src));
        }
        // Also add optional dependencies (different edge style)
        for target in &quest.optional_prerequisites {
            dot.push_str(&format!(
                "  {} -> {} [style=dashed]\n",
                target.as_u64(),
                src
            ));
        }
    }
    dot.push_str("}\n");

    // Snapshot DOT output
    insta::with_settings!({omit_expression => true}, {
        insta::assert_snapshot!(dot);
    });
}
