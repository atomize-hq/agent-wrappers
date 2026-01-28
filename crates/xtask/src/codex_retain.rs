use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};

use clap::Parser;
use semver::Version;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Parser)]
pub struct Args {
    /// Root `cli_manifests/codex` directory.
    #[arg(long, default_value = "cli_manifests/codex")]
    pub root: PathBuf,

    /// Path to `RULES.json` (default: <root>/RULES.json).
    #[arg(long)]
    pub rules: Option<PathBuf>,

    /// Apply deletions (default is dry-run).
    #[arg(long)]
    pub apply: bool,
}

#[derive(Debug, Error)]
pub enum RetainError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct RulesFile {
    storage: RulesStorage,
}

#[derive(Debug, Deserialize)]
struct RulesStorage {
    retention: RulesRetention,
}

#[derive(Debug, Deserialize)]
struct RulesRetention {
    keep_last_validated: usize,
}

#[derive(Debug, Deserialize)]
struct VersionMetadata {
    semantic_version: String,
    status: String,
}

pub fn run(args: Args) -> Result<(), RetainError> {
    let root = fs::canonicalize(&args.root).unwrap_or(args.root.clone());
    let rules_path = args
        .rules
        .clone()
        .unwrap_or_else(|| root.join("RULES.json"));
    let rules: RulesFile = serde_json::from_slice(&fs::read(&rules_path)?)?;

    let mut keep: BTreeSet<Version> = BTreeSet::new();

    // Always keep pointer versions (if present).
    add_pointer_version(&root.join("min_supported.txt"), &mut keep)?;
    add_pointer_version(&root.join("latest_validated.txt"), &mut keep)?;
    add_versions_from_pointers_tree(&root.join("pointers"), &mut keep)?;

    // Keep last N validated/supported versions by semver ordering.
    let validated = validated_versions_from_metadata(&root.join("versions"))?;
    for v in validated
        .into_iter()
        .rev()
        .take(rules.storage.retention.keep_last_validated)
    {
        keep.insert(v);
    }

    let snapshots_versions = list_semver_dirs(&root.join("snapshots"))?;
    let reports_versions = list_semver_dirs(&root.join("reports"))?;

    let mut delete: BTreeSet<Version> = BTreeSet::new();
    for v in snapshots_versions.union(&reports_versions) {
        if !keep.contains(v) {
            delete.insert(v.clone());
        }
    }

    println!("apply: {}", args.apply);
    println!("keep_versions:");
    for v in keep.iter().map(Version::to_string) {
        println!("  {v}");
    }
    println!("delete_versions:");
    for v in delete.iter().map(Version::to_string) {
        println!("  {v}");
    }

    if !args.apply {
        return Ok(());
    }

    for v in &delete {
        let name = v.to_string();
        let snap_dir = root.join("snapshots").join(&name);
        let rep_dir = root.join("reports").join(&name);

        remove_dir_all_safe(&snap_dir)?;
        remove_dir_all_safe(&rep_dir)?;
    }

    Ok(())
}

fn add_pointer_version(path: &Path, keep: &mut BTreeSet<Version>) -> Result<(), io::Error> {
    if !path.is_file() {
        return Ok(());
    }
    let raw = fs::read_to_string(path)?;
    let value = raw.trim();
    if value.is_empty() || value == "none" {
        return Ok(());
    }
    if let Ok(v) = Version::parse(value) {
        keep.insert(v);
    }
    Ok(())
}

fn add_versions_from_pointers_tree(
    root: &Path,
    keep: &mut BTreeSet<Version>,
) -> Result<(), io::Error> {
    if !root.is_dir() {
        return Ok(());
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                stack.push(path);
                continue;
            }
            if meta.is_file() && path.extension().and_then(|e| e.to_str()) == Some("txt") {
                add_pointer_version(&path, keep)?;
            }
        }
    }
    Ok(())
}

fn validated_versions_from_metadata(versions_dir: &Path) -> Result<Vec<Version>, RetainError> {
    if !versions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut out: Vec<Version> = Vec::new();
    for entry in fs::read_dir(versions_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read(&path)?;
        let meta: VersionMetadata = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if !matches!(meta.status.as_str(), "validated" | "supported") {
            continue;
        }
        if let Ok(v) = Version::parse(&meta.semantic_version) {
            out.push(v);
        }
    }
    out.sort();
    Ok(out)
}

fn list_semver_dirs(root: &Path) -> Result<BTreeSet<Version>, io::Error> {
    let mut out = BTreeSet::new();
    if !root.is_dir() {
        return Ok(out);
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if !meta.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Ok(v) = Version::parse(name) {
            out.insert(v);
        }
    }
    Ok(out)
}

fn remove_dir_all_safe(path: &Path) -> Result<(), io::Error> {
    if !path.exists() {
        return Ok(());
    }
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Ok(());
    }
    fs::remove_dir_all(path)?;
    Ok(())
}
