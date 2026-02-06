use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use super::{Args, Error};

#[derive(Debug, Deserialize)]
struct RulesFile {
    union: RulesUnion,
    sorting: RulesSorting,
}

#[derive(Debug, Deserialize)]
struct RulesUnion {
    expected_targets: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RulesSorting {
    commands: String,
    flags: String,
    args: String,
}

pub(super) fn resolve_outputs(
    args: &Args,
    version_dir: &str,
) -> Result<(PathBuf, Option<PathBuf>, String), Error> {
    let snapshot_out_path = if let Some(path) = args.out_file.as_ref() {
        path.clone()
    } else {
        args.out_dir
            .as_ref()
            .expect("clap enforces one of out_dir/out_file")
            .join("current.json")
    };

    let (codex_root, target_triple) = if let Some(out_file) = args.out_file.as_ref() {
        let version_path = out_file.parent().ok_or_else(|| {
            Error::InvalidOutFileLayout("could not infer snapshots/<version> directory".to_string())
        })?;
        let got_version = version_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>");
        if got_version != version_dir {
            return Err(Error::InvalidOutFileLayout(format!(
                "expected parent dir name {version_dir}, got {got_version}"
            )));
        }

        let snapshots_path = version_path.parent().ok_or_else(|| {
            Error::InvalidOutFileLayout("could not infer snapshots directory".to_string())
        })?;
        let got_snapshots = snapshots_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>");
        if got_snapshots != "snapshots" {
            return Err(Error::InvalidOutFileLayout(format!(
                "expected snapshots/<version> layout, got .../{got_snapshots}/{got_version}"
            )));
        }

        let inferred_from_out_file = out_file
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_suffix(".json"))
            .ok_or_else(|| {
                Error::InvalidOutFileLayout(
                    "expected out-file name like <target_triple>.json".into(),
                )
            })?
            .to_string();
        let target = args
            .raw_help_target
            .as_ref()
            .cloned()
            .unwrap_or(inferred_from_out_file);

        let expected = format!("{target}.json");
        let got = out_file
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>");
        if got != expected {
            return Err(Error::InvalidOutFileLayout(format!(
                "expected filename {expected}, got {got}"
            )));
        }

        let codex_root = snapshots_path
            .parent()
            .ok_or_else(|| Error::InvalidOutFileLayout("could not infer codex root".to_string()))?;
        (codex_root.to_path_buf(), target)
    } else {
        (
            args.out_dir
                .as_ref()
                .expect("clap enforces one of out_dir/out_file")
                .clone(),
            args.raw_help_target
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
        )
    };

    let raw_help_dir = if args.capture_raw_help {
        let base = codex_root.join("raw_help").join(version_dir);
        if args.raw_help_target.is_some() || args.out_file.is_some() {
            if target_triple == "unknown" {
                return Err(Error::MissingRawHelpTarget);
            }
            Some(base.join(&target_triple))
        } else {
            Some(base)
        }
    } else {
        None
    };

    if args.out_file.is_some() {
        let rules_path = codex_root.join("RULES.json");
        let rules: RulesFile = serde_json::from_slice(
            &fs::read(&rules_path)
                .map_err(|err| Error::RulesRead(format!("{}: {err}", rules_path.display())))?,
        )?;
        assert_supported_sorting(&rules.sorting)?;

        if !rules
            .union
            .expected_targets
            .iter()
            .any(|t| t == &target_triple)
        {
            return Err(Error::RawHelpTargetNotExpected(target_triple));
        }
    }

    Ok((snapshot_out_path, raw_help_dir, target_triple))
}

fn assert_supported_sorting(sorting: &RulesSorting) -> Result<(), Error> {
    let mut unsupported = Vec::new();

    if sorting.commands != "lexicographic_path" {
        unsupported.push(format!("sorting.commands={}", sorting.commands));
    }
    if sorting.flags != "by_key_then_long_then_short" {
        unsupported.push(format!("sorting.flags={}", sorting.flags));
    }
    if sorting.args != "by_name" {
        unsupported.push(format!("sorting.args={}", sorting.args));
    }

    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(Error::RulesUnsupported(unsupported.join(", ")))
    }
}

pub(super) fn write_raw_help(
    raw_help_dir: &Path,
    path: &[String],
    help: &str,
) -> Result<(), Error> {
    let rel = if path.is_empty() {
        PathBuf::from("help.txt")
    } else {
        let mut p = PathBuf::from("commands");
        for token in path {
            p.push(token);
        }
        p.join("help.txt")
    };
    let full = raw_help_dir.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(full, help)?;
    Ok(())
}
