use std::{collections::BTreeMap, time::Duration};

use futures_util::StreamExt;

use super::support::*;

async fn spawn_and_drain(model_id: Option<String>, env: BTreeMap<String, String>, prompt: &str) {
    let _env_lock = test_env_lock().lock().await;

    let adapter = new_adapter_with_config(ClaudeCodeBackendConfig {
        binary: Some(fake_claude_binary()),
        env,
        ..Default::default()
    });

    let spawned = adapter
        .spawn(crate::backend_harness::NormalizedRequest {
            agent_kind: adapter.kind(),
            prompt: prompt.to_string(),
            model_id,
            working_dir: None,
            effective_timeout: None,
            env: BTreeMap::new(),
            policy: super::super::harness::ClaudeExecPolicy {
                non_interactive: true,
                external_sandbox: false,
                resume: None,
                fork: None,
                resolved_working_dir: None,
                add_dirs: Vec::new(),
            },
        })
        .await
        .expect("spawn succeeds");

    let _events: Vec<_> = spawned
        .events
        .map(|result| result.expect("backend event stream is infallible for fake Claude"))
        .collect()
        .await;

    let completion = tokio::time::timeout(Duration::from_secs(2), spawned.completion)
        .await
        .expect("completion resolves")
        .expect("completion is Ok for fake Claude");
    assert!(
        completion.status.success(),
        "expected successful fake Claude run, completion: {completion:?}"
    );
}

#[tokio::test]
async fn claude_model_id_is_mapped_to_print_request_model_flag() {
    let prompt = "hello world";
    let env = BTreeMap::from([
        ("FAKE_CLAUDE_SCENARIO".to_string(), "fresh_assert".to_string()),
        ("FAKE_CLAUDE_EXPECT_PROMPT".to_string(), prompt.to_string()),
        (
            "FAKE_CLAUDE_EXPECT_MODEL".to_string(),
            "request-model".to_string(),
        ),
        (
            "FAKE_CLAUDE_EXPECT_NO_FALLBACK_MODEL".to_string(),
            "true".to_string(),
        ),
    ]);

    spawn_and_drain(Some("request-model".to_string()), env, prompt).await;
}

#[tokio::test]
async fn claude_absent_model_id_emits_no_model_flag() {
    let prompt = "hello world";
    let env = BTreeMap::from([
        ("FAKE_CLAUDE_SCENARIO".to_string(), "fresh_assert".to_string()),
        ("FAKE_CLAUDE_EXPECT_PROMPT".to_string(), prompt.to_string()),
        ("FAKE_CLAUDE_EXPECT_NO_MODEL".to_string(), "true".to_string()),
        (
            "FAKE_CLAUDE_EXPECT_NO_FALLBACK_MODEL".to_string(),
            "true".to_string(),
        ),
    ]);

    spawn_and_drain(None, env, prompt).await;
}

