use std::{fs, path::Path, process::Command};

use regex::Regex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::{util, Error};

pub(super) fn deterministic_rfc3339_now() -> String {
    if let Ok(v) = std::env::var("SOURCE_DATE_EPOCH") {
        if let Ok(secs) = v.parse::<i64>() {
            if let Ok(ts) = OffsetDateTime::from_unix_timestamp(secs) {
                return ts
                    .format(&Rfc3339)
                    .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
            }
        }
    }
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

pub(super) struct BinaryMetadata {
    pub(super) sha256: String,
    pub(super) size_bytes: u64,
}

impl BinaryMetadata {
    pub(super) fn collect(path: &Path) -> Result<Self, Error> {
        let bytes = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha256 = hex::encode(hasher.finalize());
        let size_bytes = bytes.len() as u64;
        Ok(Self { sha256, size_bytes })
    }
}

type VersionProbe = (String, Option<String>, Option<String>, Option<String>);

pub(super) fn probe_version(codex_binary: &Path) -> Result<VersionProbe, Error> {
    let mut cmd = Command::new(codex_binary);
    cmd.arg("--version");
    cmd.env("NO_COLOR", "1");
    cmd.env("CLICOLOR", "0");
    cmd.env("TERM", "dumb");

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(Error::CommandFailed(util::command_failed_message(
            &cmd, &output,
        )));
    }
    let version_output = util::normalize_text(&output.stdout, &output.stderr)
        .trim()
        .to_string();

    let re_semver = Regex::new(r"(?P<v>\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)").unwrap();
    let semantic_version = re_semver
        .captures(&version_output)
        .and_then(|c| c.name("v").map(|m| m.as_str().to_string()));

    let channel = semantic_version.as_ref().map(|v| {
        if v.contains("nightly") {
            "nightly".to_string()
        } else if v.contains("beta") {
            "beta".to_string()
        } else {
            "stable".to_string()
        }
    });

    let re_commit = Regex::new(r"(?i)\b([0-9a-f]{7,40})\b").unwrap();
    let commit = re_commit
        .captures(&version_output)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()));

    Ok((version_output, semantic_version, channel, commit))
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct FeatureInfo {
    pub(super) name: String,
    pub(super) stage: String,
    pub(super) effective: bool,
}

pub(super) fn probe_features(codex_binary: &Path) -> (Option<Vec<FeatureInfo>>, Option<String>) {
    let mut cmd = Command::new(codex_binary);
    cmd.args(["features", "list"]);
    cmd.env("NO_COLOR", "1");
    cmd.env("CLICOLOR", "0");
    cmd.env("TERM", "dumb");

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => return (None, Some(format!("spawn failed: {e}"))),
    };
    if !output.status.success() {
        return (
            None,
            Some(
                util::command_failed_message(&cmd, &output)
                    .trim()
                    .to_string(),
            ),
        );
    }

    let text = util::normalize_text(&output.stdout, &output.stderr);
    let mut features = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let parts = t.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 3 {
            continue;
        }
        let effective = match parts[2].to_ascii_lowercase().as_str() {
            "true" => true,
            "false" => false,
            _ => continue,
        };
        features.push(FeatureInfo {
            name: parts[0].to_string(),
            stage: parts[1].to_string(),
            effective,
        });
    }
    features.sort_by(|a, b| a.name.cmp(&b.name));
    (Some(features), None)
}

pub(super) fn build_features_metadata(
    listed: Option<Vec<FeatureInfo>>,
    probe_error: Option<String>,
    enabled_feature_names: Vec<String>,
    commands_added: Option<Vec<Vec<String>>>,
) -> Option<serde_json::Value> {
    if listed.is_none() && probe_error.is_none() {
        return None;
    }

    let mut obj = serde_json::Map::new();
    obj.insert(
        "mode".to_string(),
        serde_json::Value::String("default_plus_all_enabled".to_string()),
    );
    if let Some(err) = probe_error {
        obj.insert("probe_error".to_string(), serde_json::Value::String(err));
    }
    if let Some(list) = listed {
        obj.insert(
            "listed".to_string(),
            serde_json::to_value(list).unwrap_or(serde_json::Value::Null),
        );
    }
    if !enabled_feature_names.is_empty() {
        obj.insert(
            "enabled_for_snapshot".to_string(),
            serde_json::to_value(enabled_feature_names).unwrap_or(serde_json::Value::Null),
        );
    }
    if let Some(added) = commands_added {
        obj.insert(
            "commands_added_when_all_enabled".to_string(),
            serde_json::to_value(added).unwrap_or(serde_json::Value::Null),
        );
    }

    Some(serde_json::Value::Object(obj))
}
