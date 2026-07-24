use std::{collections::BTreeSet, fs, io, path::Path};

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::ReportError;

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

pub(super) fn require_source_date_epoch_if_ci() -> Result<(), ReportError> {
    if std::env::var("CI").is_err() {
        return Ok(());
    }
    if std::env::var("SOURCE_DATE_EPOCH").is_err() {
        return Err(ReportError::Rules(
            "CI requires SOURCE_DATE_EPOCH for deterministic generated_at".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn write_json_pretty(path: &Path, json: &str) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

pub(super) fn intersect(a: &BTreeSet<String>, b: &BTreeSet<String>) -> BTreeSet<String> {
    a.intersection(b).cloned().collect()
}

pub(super) fn ordered_subset(
    expected_targets: &[String],
    targets: &BTreeSet<String>,
) -> Vec<String> {
    expected_targets
        .iter()
        .filter(|t| targets.contains(*t))
        .cloned()
        .collect()
}

pub(super) fn cmp_path(a: &[String], b: &[String]) -> std::cmp::Ordering {
    let mut i = 0usize;
    while i < a.len() && i < b.len() {
        match a[i].cmp(&b[i]) {
            std::cmp::Ordering::Equal => i += 1,
            non_eq => return non_eq,
        }
    }
    a.len().cmp(&b.len())
}

pub(super) fn format_path(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_string()
    } else {
        path.join(" ")
    }
}

pub(super) fn is_prefix(prefix: &[String], path: &[String]) -> bool {
    prefix.len() <= path.len() && prefix.iter().zip(path).all(|(a, b)| a == b)
}
