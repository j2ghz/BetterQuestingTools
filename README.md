# BetterQuestingTools

Small Rust library to parse BetterQuesting's `DefaultQuests` folder (the mod's
quest database export) into a convenient Rust domain model.

Features

- Normalizes NBT-like key suffixes (e.g. `name:8`) and converts numeric-keyed
  maps into arrays.
- Parses Quests, QuestLines and QuestSettings.
- Returns a strict `QuestDatabase` that fails on dangling references.

Quick example

```rust,no_run
use better_questing_tools::db::parse_default_quests_dir;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = parse_default_quests_dir(Path::new("/path/to/DefaultQuests"))?;
    println!("{} quests parsed", db.quests.len());
    Ok(())
}
```

Running tests

cargo test

Snapshots are under `tests/snapshots` and are used by the integration test
`tests/parse_samples.rs`.

License: MIT/Apache-2.0
