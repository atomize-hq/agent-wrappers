use std::{collections::BTreeMap, fs, path::PathBuf};

use codex::wrapper_coverage_manifest::{wrapper_coverage_manifest, CoverageLevel};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ExamplesManifest {
    schema_version: u32,
    commands: Vec<ExamplesManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct ExamplesManifestEntry {
    path: Vec<String>,
    examples: Vec<String>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/codex has repo root parent")
        .parent()
        .expect("repo root exists")
        .to_path_buf()
}

#[test]
fn examples_manifest_covers_all_explicit_wrapper_commands() {
    let manifest_path = repo_root()
        .join("crates")
        .join("codex")
        .join("examples")
        .join("examples_manifest.json");
    let bytes = fs::read(&manifest_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest_path.display()));
    let manifest: ExamplesManifest = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("parse {}: {e}", manifest_path.display()));
    assert_eq!(manifest.schema_version, 1);

    let mut map: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
    for entry in manifest.commands {
        assert!(
            !entry.path.is_empty(),
            "examples manifest must not include empty root path"
        );
        assert!(
            !entry.examples.is_empty(),
            "examples manifest entry for {:?} must list at least one example",
            entry.path
        );
        map.insert(entry.path, entry.examples);
    }

    let examples_dir = repo_root().join("crates").join("codex").join("examples");

    let wrapper = wrapper_coverage_manifest();
    let mut missing = Vec::new();
    for cmd in wrapper.coverage {
        if cmd.level != CoverageLevel::Explicit {
            continue;
        }
        if cmd.path.is_empty() {
            continue;
        }
        if !map.contains_key(&cmd.path) {
            missing.push(cmd.path);
        }
    }
    assert!(
        missing.is_empty(),
        "missing example coverage for explicit wrapper commands: {missing:?}"
    );

    for (path, examples) in map {
        for example in examples {
            let file = examples_dir.join(format!("{example}.rs"));
            assert!(
                file.is_file(),
                "examples manifest entry for {path:?} references missing file {}",
                file.display()
            );
        }
    }
}
