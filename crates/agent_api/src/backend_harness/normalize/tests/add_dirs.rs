use std::{
    fs,
    path::Path,
};

use serde_json::{json, Value};
use tempfile::tempdir;

use super::super::normalize_add_dirs_v1;
use crate::AgentWrapperError;

#[test]
fn ad_c02_absent_key_returns_empty_vec() {
    let temp = tempdir().expect("tempdir");
    let normalized = normalize_add_dirs_v1(None, temp.path()).expect("absent key normalizes");
    assert!(normalized.is_empty());
}

#[test]
fn ad_c02_non_object_payload_uses_safe_root_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let secret = "/tmp/secret-path";
    let payload = Value::String(secret.to_string());

    assert_invalid_message(
        Some(&payload),
        temp.path(),
        "invalid agent_api.exec.add_dirs.v1",
        secret,
    );
}

#[test]
fn ad_c02_unknown_key_uses_safe_root_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let secret = "/tmp/secret-path";
    let payload = json!({ "unexpected": secret });

    assert_invalid_message(
        Some(&payload),
        temp.path(),
        "invalid agent_api.exec.add_dirs.v1",
        secret,
    );
}

#[test]
fn ad_c02_missing_dirs_uses_safe_root_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let payload = json!({});

    let err = normalize_add_dirs_v1(Some(&payload), temp.path()).expect_err("expected invalid");
    match &err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(message, "invalid agent_api.exec.add_dirs.v1");
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }
}

#[test]
fn ad_c02_non_array_dirs_uses_safe_container_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let secret = "/tmp/not-an-array";
    let payload = json!({ "dirs": secret });

    assert_invalid_message(
        Some(&payload),
        temp.path(),
        "invalid agent_api.exec.add_dirs.v1.dirs",
        secret,
    );
}

#[test]
fn ad_c02_empty_dirs_uses_safe_container_message() {
    let temp = tempdir().expect("tempdir");
    let payload = json!({ "dirs": [] });

    assert_invalid_message(
        Some(&payload),
        temp.path(),
        "invalid agent_api.exec.add_dirs.v1.dirs",
        "[]",
    );
}

#[test]
fn ad_c02_too_many_dirs_uses_safe_container_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let oversized: Vec<String> = (0..17)
        .map(|index| format!("/tmp/secret-path-{index}"))
        .collect();
    let leaked = oversized[16].clone();
    let payload = json!({ "dirs": oversized });

    assert_invalid_message(
        Some(&payload),
        temp.path(),
        "invalid agent_api.exec.add_dirs.v1.dirs",
        &leaked,
    );
}

#[test]
fn ad_c02_non_string_entry_uses_safe_indexed_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let payload = json!({ "dirs": [5] });

    assert_invalid_message(
        Some(&payload),
        temp.path(),
        "invalid agent_api.exec.add_dirs.v1.dirs[0]",
        "5",
    );
}

#[test]
fn ad_c02_trimmed_empty_entry_uses_safe_indexed_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let payload = json!({ "dirs": [" \t\n "] });

    assert_invalid_message(
        Some(&payload),
        temp.path(),
        "invalid agent_api.exec.add_dirs.v1.dirs[0]",
        " \t\n ",
    );
}

#[test]
fn ad_c02_over_byte_limit_entry_uses_safe_indexed_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let oversized = "a".repeat(1025);
    let payload = json!({ "dirs": [oversized] });

    assert_invalid_message(
        Some(&payload),
        temp.path(),
        "invalid agent_api.exec.add_dirs.v1.dirs[0]",
        &"a".repeat(32),
    );
}

#[test]
fn ad_c02_missing_directory_uses_safe_indexed_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let missing = "missing-dir";
    let payload = json!({ "dirs": [missing] });

    assert_invalid_message(
        Some(&payload),
        temp.path(),
        "invalid agent_api.exec.add_dirs.v1.dirs[0]",
        missing,
    );
}

#[test]
fn ad_c02_non_directory_path_uses_safe_indexed_message_without_leakage() {
    let temp = tempdir().expect("tempdir");
    let file_path = temp.path().join("not-a-dir.txt");
    fs::write(&file_path, "content").expect("write file");
    let payload = json!({ "dirs": [file_path.to_string_lossy().to_string()] });

    assert_invalid_message(
        Some(&payload),
        temp.path(),
        "invalid agent_api.exec.add_dirs.v1.dirs[0]",
        &file_path.to_string_lossy(),
    );
}

#[test]
fn ad_c02_accepts_absolute_directory_entries() {
    let temp = tempdir().expect("tempdir");
    let absolute_dir = temp.path().join("absolute");
    fs::create_dir(&absolute_dir).expect("create dir");
    let payload = json!({ "dirs": [absolute_dir.to_string_lossy().to_string()] });

    let normalized = normalize_add_dirs_v1(Some(&payload), temp.path()).expect("normalize");
    assert_eq!(normalized, vec![absolute_dir]);
}

#[test]
fn ad_c02_resolves_relative_entries_from_effective_working_dir() {
    let temp = tempdir().expect("tempdir");
    let effective = temp.path().join("repo");
    let relative_dir = effective.join("docs");
    fs::create_dir_all(&relative_dir).expect("create dir");
    let payload = json!({ "dirs": ["docs"] });

    let normalized = normalize_add_dirs_v1(Some(&payload), &effective).expect("normalize");
    assert_eq!(normalized, vec![relative_dir]);
}

#[test]
fn ad_c02_lexically_normalizes_and_deduplicates_while_preserving_first_order() {
    let temp = tempdir().expect("tempdir");
    let effective = temp.path().join("repo");
    let first = effective.join("nested");
    let second = effective.join("other");
    fs::create_dir_all(&first).expect("create first dir");
    fs::create_dir_all(&second).expect("create second dir");

    let payload = json!({
        "dirs": [
            "./nested/./",
            "nested/../nested",
            "./other/../other"
        ]
    });

    let normalized = normalize_add_dirs_v1(Some(&payload), &effective).expect("normalize");
    assert_eq!(normalized, vec![first, second]);
}

fn assert_invalid_message(
    raw: Option<&Value>,
    effective_working_dir: &Path,
    expected_message: &str,
    leaked_text: &str,
) {
    let err = normalize_add_dirs_v1(raw, effective_working_dir).expect_err("expected invalid");
    match &err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(message, expected_message);
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }

    assert!(
        !err.to_string().contains(leaked_text),
        "error display leaked sensitive text: {leaked_text}"
    );
}
