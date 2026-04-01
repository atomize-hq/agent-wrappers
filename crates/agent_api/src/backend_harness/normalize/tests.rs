use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::{json, Value};

use super::super::test_support::{
    toy_kind, ToyAdapter, ToyBackendError, ToyCompletion, ToyEvent, ToyPolicy,
};
use super::super::{
    BackendDefaults, BackendHarnessAdapter, BackendHarnessErrorPhase, NormalizedRequest,
};
use super::{
    normalize_model_id_v1, normalize_request, parse_ext_bool, parse_ext_string_enum,
};
use crate::{AgentWrapperCompletion, AgentWrapperError, AgentWrapperRunRequest};

mod c02_add_dirs;
mod c03_timeout;

const MODEL_ID_KEY: &str = "agent_api.config.model.v1";

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read source directory") {
        let entry = entry.expect("read source entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

#[test]
fn bh_c02_unknown_extension_key_is_rejected_via_normalize_request() {
    struct PanicOnPolicyAdapter;

    impl BackendHarnessAdapter for PanicOnPolicyAdapter {
        fn kind(&self) -> crate::AgentWrapperKind {
            toy_kind()
        }

        fn supported_extension_keys(&self) -> &'static [&'static str] {
            &["known.key"]
        }

        type Policy = ToyPolicy;

        fn validate_and_extract_policy(
            &self,
            _request: &AgentWrapperRunRequest,
        ) -> Result<Self::Policy, crate::AgentWrapperError> {
            panic!("validate_and_extract_policy must not be called for unknown keys");
        }

        type BackendEvent = ToyEvent;
        type BackendCompletion = ToyCompletion;
        type BackendError = ToyBackendError;

        fn spawn(
            &self,
            _req: NormalizedRequest<Self::Policy>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            super::super::contract::BackendSpawn<
                                Self::BackendEvent,
                                Self::BackendCompletion,
                                Self::BackendError,
                            >,
                            Self::BackendError,
                        >,
                    > + Send
                    + 'static,
            >,
        > {
            panic!("spawn must not be called from normalize_request");
        }

        fn map_event(&self, _event: Self::BackendEvent) -> Vec<crate::AgentWrapperEvent> {
            panic!("map_event must not be called from normalize_request");
        }

        fn map_completion(
            &self,
            _completion: Self::BackendCompletion,
        ) -> Result<AgentWrapperCompletion, crate::AgentWrapperError> {
            panic!("map_completion must not be called from normalize_request");
        }

        fn redact_error(
            &self,
            _phase: BackendHarnessErrorPhase,
            _err: &Self::BackendError,
        ) -> String {
            panic!("redact_error must not be called from normalize_request");
        }
    }

    let adapter = PanicOnPolicyAdapter;
    let defaults = BackendDefaults::default();
    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request.extensions.insert(
        "unknown.key".to_string(),
        Value::String("SECRET_SHOULD_NOT_LEAK".to_string()),
    );

    let err = match normalize_request(&adapter, &defaults, request) {
        Ok(_) => panic!("unknown key must fail closed"),
        Err(err) => err,
    };
    match &err {
        AgentWrapperError::UnsupportedCapability {
            agent_kind,
            capability,
        } => {
            assert_eq!(agent_kind, "toy");
            assert_eq!(capability, "unknown.key");
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
    assert!(!err.to_string().contains("SECRET_SHOULD_NOT_LEAK"));
}

#[test]
fn bh_r0_external_sandbox_key_is_rejected_before_policy_validation_via_normalize_request() {
    struct PanicOnPolicyAdapter;

    impl BackendHarnessAdapter for PanicOnPolicyAdapter {
        fn kind(&self) -> crate::AgentWrapperKind {
            toy_kind()
        }

        fn supported_extension_keys(&self) -> &'static [&'static str] {
            &["agent_api.exec.non_interactive"]
        }

        type Policy = ToyPolicy;

        fn validate_and_extract_policy(
            &self,
            _request: &AgentWrapperRunRequest,
        ) -> Result<Self::Policy, crate::AgentWrapperError> {
            panic!("validate_and_extract_policy must not be called for unsupported keys");
        }

        type BackendEvent = ToyEvent;
        type BackendCompletion = ToyCompletion;
        type BackendError = ToyBackendError;

        fn spawn(
            &self,
            _req: NormalizedRequest<Self::Policy>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            super::super::contract::BackendSpawn<
                                Self::BackendEvent,
                                Self::BackendCompletion,
                                Self::BackendError,
                            >,
                            Self::BackendError,
                        >,
                    > + Send
                    + 'static,
            >,
        > {
            panic!("spawn must not be called from normalize_request");
        }

        fn map_event(&self, _event: Self::BackendEvent) -> Vec<crate::AgentWrapperEvent> {
            panic!("map_event must not be called from normalize_request");
        }

        fn map_completion(
            &self,
            _completion: Self::BackendCompletion,
        ) -> Result<AgentWrapperCompletion, crate::AgentWrapperError> {
            panic!("map_completion must not be called from normalize_request");
        }

        fn redact_error(
            &self,
            _phase: BackendHarnessErrorPhase,
            _err: &Self::BackendError,
        ) -> String {
            panic!("redact_error must not be called from normalize_request");
        }
    }

    let adapter = PanicOnPolicyAdapter;
    let defaults = BackendDefaults::default();
    let secret = "SECRET_SHOULD_NOT_LEAK";

    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request.extensions.insert(
        "agent_api.exec.external_sandbox.v1".to_string(),
        Value::String(secret.to_string()),
    );
    request.extensions.insert(
        "agent_api.exec.non_interactive".to_string(),
        Value::Bool(false),
    );

    let err = match normalize_request(&adapter, &defaults, request) {
        Ok(_) => panic!("unsupported key must fail closed"),
        Err(err) => err,
    };
    match &err {
        AgentWrapperError::UnsupportedCapability {
            agent_kind,
            capability,
        } => {
            assert_eq!(agent_kind, "toy");
            assert_eq!(capability, "agent_api.exec.external_sandbox.v1");
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
    assert!(!err.to_string().contains(secret));
}

#[test]
fn bh_r0_unsupported_capability_beats_contradiction_rules_for_resume_fork_via_normalize_request() {
    struct PanicOnPolicyAdapter;

    impl BackendHarnessAdapter for PanicOnPolicyAdapter {
        fn kind(&self) -> crate::AgentWrapperKind {
            toy_kind()
        }

        fn supported_extension_keys(&self) -> &'static [&'static str] {
            const SUPPORTED: [&str; 1] = ["agent_api.session.resume.v1"];
            &SUPPORTED
        }

        type Policy = ToyPolicy;

        fn validate_and_extract_policy(
            &self,
            _request: &AgentWrapperRunRequest,
        ) -> Result<Self::Policy, crate::AgentWrapperError> {
            panic!("validate_and_extract_policy must not be called when any key is unsupported");
        }

        type BackendEvent = ToyEvent;
        type BackendCompletion = ToyCompletion;
        type BackendError = ToyBackendError;

        fn spawn(
            &self,
            _req: NormalizedRequest<Self::Policy>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            super::super::contract::BackendSpawn<
                                Self::BackendEvent,
                                Self::BackendCompletion,
                                Self::BackendError,
                            >,
                            Self::BackendError,
                        >,
                    > + Send
                    + 'static,
            >,
        > {
            panic!("spawn must not be called from normalize_request");
        }

        fn map_event(&self, _event: Self::BackendEvent) -> Vec<crate::AgentWrapperEvent> {
            panic!("map_event must not be called from normalize_request");
        }

        fn map_completion(
            &self,
            _completion: Self::BackendCompletion,
        ) -> Result<AgentWrapperCompletion, crate::AgentWrapperError> {
            panic!("map_completion must not be called from normalize_request");
        }

        fn redact_error(
            &self,
            _phase: BackendHarnessErrorPhase,
            _err: &Self::BackendError,
        ) -> String {
            panic!("redact_error must not be called from normalize_request");
        }
    }

    let adapter = PanicOnPolicyAdapter;
    let defaults = BackendDefaults::default();
    let secret = "SECRET_SHOULD_NOT_LEAK";

    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request.extensions.insert(
        "agent_api.session.fork.v1".to_string(),
        Value::String(secret.to_string()),
    );
    request.extensions.insert(
        "agent_api.session.resume.v1".to_string(),
        json!({"selector": "last"}),
    );

    let err = match normalize_request(&adapter, &defaults, request) {
        Ok(_) => panic!("unsupported fork key must fail closed before contradiction rules"),
        Err(err) => err,
    };
    match &err {
        AgentWrapperError::UnsupportedCapability { capability, .. } => {
            assert_eq!(capability, "agent_api.session.fork.v1");
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
    assert!(!err.to_string().contains(secret));
}

#[test]
fn bh_r0_unsupported_capability_beats_contradiction_rules_for_fork_resume_via_normalize_request() {
    struct PanicOnPolicyAdapter;

    impl BackendHarnessAdapter for PanicOnPolicyAdapter {
        fn kind(&self) -> crate::AgentWrapperKind {
            toy_kind()
        }

        fn supported_extension_keys(&self) -> &'static [&'static str] {
            const SUPPORTED: [&str; 1] = ["agent_api.session.fork.v1"];
            &SUPPORTED
        }

        type Policy = ToyPolicy;

        fn validate_and_extract_policy(
            &self,
            _request: &AgentWrapperRunRequest,
        ) -> Result<Self::Policy, crate::AgentWrapperError> {
            panic!("validate_and_extract_policy must not be called when any key is unsupported");
        }

        type BackendEvent = ToyEvent;
        type BackendCompletion = ToyCompletion;
        type BackendError = ToyBackendError;

        fn spawn(
            &self,
            _req: NormalizedRequest<Self::Policy>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            super::super::contract::BackendSpawn<
                                Self::BackendEvent,
                                Self::BackendCompletion,
                                Self::BackendError,
                            >,
                            Self::BackendError,
                        >,
                    > + Send
                    + 'static,
            >,
        > {
            panic!("spawn must not be called from normalize_request");
        }

        fn map_event(&self, _event: Self::BackendEvent) -> Vec<crate::AgentWrapperEvent> {
            panic!("map_event must not be called from normalize_request");
        }

        fn map_completion(
            &self,
            _completion: Self::BackendCompletion,
        ) -> Result<AgentWrapperCompletion, crate::AgentWrapperError> {
            panic!("map_completion must not be called from normalize_request");
        }

        fn redact_error(
            &self,
            _phase: BackendHarnessErrorPhase,
            _err: &Self::BackendError,
        ) -> String {
            panic!("redact_error must not be called from normalize_request");
        }
    }

    let adapter = PanicOnPolicyAdapter;
    let defaults = BackendDefaults::default();
    let secret = "SECRET_SHOULD_NOT_LEAK";

    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request.extensions.insert(
        "agent_api.session.fork.v1".to_string(),
        json!({"selector": "last"}),
    );
    request.extensions.insert(
        "agent_api.session.resume.v1".to_string(),
        Value::String(secret.to_string()),
    );

    let err = match normalize_request(&adapter, &defaults, request) {
        Ok(_) => panic!("unsupported resume key must fail closed before contradiction rules"),
        Err(err) => err,
    };
    match &err {
        AgentWrapperError::UnsupportedCapability { capability, .. } => {
            assert_eq!(capability, "agent_api.session.resume.v1");
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
    assert!(!err.to_string().contains(secret));
}

#[test]
fn bh_r0_resume_fork_contradiction_applies_only_after_all_keys_are_supported_via_normalize_request()
{
    struct ResumeForkContradictionAdapter;

    impl BackendHarnessAdapter for ResumeForkContradictionAdapter {
        fn kind(&self) -> crate::AgentWrapperKind {
            toy_kind()
        }

        fn supported_extension_keys(&self) -> &'static [&'static str] {
            const SUPPORTED: [&str; 2] =
                ["agent_api.session.fork.v1", "agent_api.session.resume.v1"];
            &SUPPORTED
        }

        type Policy = ToyPolicy;

        fn validate_and_extract_policy(
            &self,
            request: &AgentWrapperRunRequest,
        ) -> Result<Self::Policy, crate::AgentWrapperError> {
            if request
                .extensions
                .contains_key("agent_api.session.resume.v1")
                && request.extensions.contains_key("agent_api.session.fork.v1")
            {
                return Err(AgentWrapperError::InvalidRequest {
                    message: "agent_api.session.resume.v1 and agent_api.session.fork.v1 are mutually exclusive"
                        .to_string(),
                });
            }

            Ok(ToyPolicy)
        }

        type BackendEvent = ToyEvent;
        type BackendCompletion = ToyCompletion;
        type BackendError = ToyBackendError;

        fn spawn(
            &self,
            _req: NormalizedRequest<Self::Policy>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            super::super::contract::BackendSpawn<
                                Self::BackendEvent,
                                Self::BackendCompletion,
                                Self::BackendError,
                            >,
                            Self::BackendError,
                        >,
                    > + Send
                    + 'static,
            >,
        > {
            panic!("spawn must not be called from normalize_request");
        }

        fn map_event(&self, _event: Self::BackendEvent) -> Vec<crate::AgentWrapperEvent> {
            panic!("map_event must not be called from normalize_request");
        }

        fn map_completion(
            &self,
            _completion: Self::BackendCompletion,
        ) -> Result<AgentWrapperCompletion, crate::AgentWrapperError> {
            panic!("map_completion must not be called from normalize_request");
        }

        fn redact_error(
            &self,
            _phase: BackendHarnessErrorPhase,
            _err: &Self::BackendError,
        ) -> String {
            panic!("redact_error must not be called from normalize_request");
        }
    }

    let adapter = ResumeForkContradictionAdapter;
    let defaults = BackendDefaults::default();
    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request.extensions.insert(
        "agent_api.session.fork.v1".to_string(),
        json!({"selector": "last"}),
    );
    request.extensions.insert(
        "agent_api.session.resume.v1".to_string(),
        json!({"selector": "last"}),
    );

    let err = match normalize_request(&adapter, &defaults, request) {
        Ok(_) => panic!("expected mutual-exclusion InvalidRequest"),
        Err(err) => err,
    };
    match err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(
                message,
                "agent_api.session.resume.v1 and agent_api.session.fork.v1 are mutually exclusive"
            );
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }
}

#[test]
fn bh_c02_multiple_unknown_extension_keys_report_lexicographically_smallest_via_normalize_request()
{
    let adapter = ToyAdapter { fail_spawn: false };
    let defaults = BackendDefaults::default();
    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request
        .extensions
        .insert("zzz.unknown".to_string(), Value::Bool(true));
    request
        .extensions
        .insert("aaa.unknown".to_string(), Value::Bool(true));

    let err = match normalize_request(&adapter, &defaults, request) {
        Ok(_) => panic!("unknown key must fail closed"),
        Err(err) => err,
    };
    match err {
        AgentWrapperError::UnsupportedCapability { capability, .. } => {
            assert_eq!(capability, "aaa.unknown");
        }
        other => panic!("expected UnsupportedCapability, got: {other:?}"),
    }
}

#[test]
fn bh_c02_all_keys_allowed_passes_via_normalize_request() {
    let adapter = ToyAdapter { fail_spawn: false };
    let defaults = BackendDefaults::default();
    let mut request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };
    request.extensions.insert(
        "agent_api.exec.non_interactive".to_string(),
        Value::Bool(true),
    );
    request
        .extensions
        .insert("backend.toy.example".to_string(), Value::Bool(true));

    let normalized = normalize_request(&adapter, &defaults, request).expect("all keys allowed");
    assert_eq!(normalized.agent_kind.as_str(), "toy");
    assert_eq!(normalized.prompt, "hello");
    assert_eq!(normalized.model_id, None);
}

#[test]
fn bh_c03_env_merge_precedence_via_normalize_request() {
    let adapter = ToyAdapter { fail_spawn: false };
    let defaults = BackendDefaults {
        env: BTreeMap::from([
            ("A".to_string(), "1".to_string()),
            ("B".to_string(), "1".to_string()),
        ]),
        default_timeout: None,
    };

    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        env: BTreeMap::from([("B".to_string(), "2".to_string())]),
        ..Default::default()
    };

    let normalized = normalize_request(&adapter, &defaults, request).expect("normalizes");
    assert_eq!(normalized.env.get("A").map(String::as_str), Some("1"));
    assert_eq!(normalized.env.get("B").map(String::as_str), Some("2"));
}

#[test]
fn bh_c03_env_merge_empty_cases_via_normalize_request() {
    let adapter = ToyAdapter { fail_spawn: false };

    let defaults = BackendDefaults {
        env: BTreeMap::new(),
        default_timeout: None,
    };
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        env: BTreeMap::from([("X".to_string(), "x".to_string())]),
        ..Default::default()
    };
    let normalized = normalize_request(&adapter, &defaults, request).expect("normalizes");
    assert_eq!(normalized.env.get("X").map(String::as_str), Some("x"));

    let defaults = BackendDefaults {
        env: BTreeMap::from([("Y".to_string(), "y".to_string())]),
        default_timeout: None,
    };
    let request = AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        env: BTreeMap::new(),
        ..Default::default()
    };
    let normalized = normalize_request(&adapter, &defaults, request).expect("normalizes");
    assert_eq!(normalized.env.get("Y").map(String::as_str), Some("y"));
}

#[test]
fn universal_invalid_request_empty_prompt_short_circuits_allowlist_and_policy() {
    struct PanicOnAllowlistAdapter;

    impl BackendHarnessAdapter for PanicOnAllowlistAdapter {
        fn kind(&self) -> crate::AgentWrapperKind {
            toy_kind()
        }

        fn supported_extension_keys(&self) -> &'static [&'static str] {
            panic!("supported_extension_keys must not be called for empty prompt");
        }

        type Policy = ToyPolicy;

        fn validate_and_extract_policy(
            &self,
            _request: &AgentWrapperRunRequest,
        ) -> Result<Self::Policy, AgentWrapperError> {
            panic!("validate_and_extract_policy must not be called for empty prompt");
        }

        type BackendEvent = ToyEvent;
        type BackendCompletion = ToyCompletion;
        type BackendError = ToyBackendError;

        fn spawn(
            &self,
            _req: NormalizedRequest<Self::Policy>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            super::super::contract::BackendSpawn<
                                Self::BackendEvent,
                                Self::BackendCompletion,
                                Self::BackendError,
                            >,
                            Self::BackendError,
                        >,
                    > + Send
                    + 'static,
            >,
        > {
            panic!("spawn must not be called from normalize_request");
        }

        fn map_event(&self, _event: Self::BackendEvent) -> Vec<crate::AgentWrapperEvent> {
            panic!("map_event must not be called from normalize_request");
        }

        fn map_completion(
            &self,
            _completion: Self::BackendCompletion,
        ) -> Result<AgentWrapperCompletion, crate::AgentWrapperError> {
            panic!("map_completion must not be called from normalize_request");
        }

        fn redact_error(
            &self,
            _phase: BackendHarnessErrorPhase,
            _err: &Self::BackendError,
        ) -> String {
            panic!("redact_error must not be called from normalize_request");
        }
    }

    let adapter = PanicOnAllowlistAdapter;
    let defaults = BackendDefaults::default();
    let request = AgentWrapperRunRequest {
        prompt: "   ".to_string(),
        timeout: Some(Duration::from_secs(123)),
        env: BTreeMap::from([("SECRET_ENV".to_string(), "SECRET_VAL".to_string())]),
        extensions: BTreeMap::from([(
            "unknown.key".to_string(),
            Value::String("SECRET_SHOULD_NOT_LEAK".to_string()),
        )]),
        ..Default::default()
    };

    let err = match normalize_request(&adapter, &defaults, request) {
        Ok(_) => panic!("empty prompt must be rejected"),
        Err(err) => err,
    };
    match err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(message, "prompt must not be empty");
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }
}

#[test]
fn parse_ext_bool_rejects_non_boolean() {
    let err = parse_ext_bool(&json!("nope"), "k").expect_err("expected bool parse failure");
    match err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(message, "k must be a boolean");
            assert!(!message.contains("nope"));
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }
}

#[test]
fn parse_ext_string_enum_rejects_unknown_value_without_leaking_value() {
    let err = parse_ext_string_enum(&json!("nope"), "k", &["a", "b", "c"])
        .expect_err("expected enum parse failure");
    match err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(message, "k must be one of: a | b | c");
            assert!(!message.contains("nope"));
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }
}

#[test]
fn normalize_model_id_v1_absent_returns_none() {
    let normalized = normalize_model_id_v1(None).expect("absent model id is allowed");
    assert_eq!(normalized, None);
}

#[test]
fn normalize_model_id_v1_rejects_non_string_without_echoing_value() {
    let raw = json!(false);
    let err = normalize_model_id_v1(Some(&raw)).expect_err("expected string parse failure");
    match err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(message, "invalid agent_api.config.model.v1");
            assert!(!message.contains("false"));
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }
}

#[test]
fn normalize_model_id_v1_rejects_whitespace_only_after_trim() {
    let err = normalize_model_id_v1(Some(&json!("  \t \n  ")))
        .expect_err("expected whitespace-only failure");
    match err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(message, "invalid agent_api.config.model.v1");
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }
}

#[test]
fn normalize_model_id_v1_rejects_oversize_after_trim_without_echoing_value() {
    let raw = format!("  {}  ", "a".repeat(129));
    let err = normalize_model_id_v1(Some(&json!(raw.clone())))
        .expect_err("expected oversize failure");
    match err {
        AgentWrapperError::InvalidRequest { message } => {
            assert_eq!(message, "invalid agent_api.config.model.v1");
            assert!(!message.contains(&raw));
        }
        other => panic!("expected InvalidRequest, got: {other:?}"),
    }
}

#[test]
fn normalize_model_id_v1_trims_and_returns_success() {
    let normalized = normalize_model_id_v1(Some(&json!("  agent-model-1  ")))
        .expect("expected trimmed success");
    assert_eq!(normalized, Some("agent-model-1".to_string()));
}

#[test]
fn bh_r0_agent_api_config_model_v1_is_rejected_before_value_shape_validation_via_normalize_request()
{
    struct PanicOnPolicyAdapter;

    impl BackendHarnessAdapter for PanicOnPolicyAdapter {
        fn kind(&self) -> crate::AgentWrapperKind {
            toy_kind()
        }

        fn supported_extension_keys(&self) -> &'static [&'static str] {
            &["backend.toy.example"]
        }

        type Policy = ToyPolicy;

        fn validate_and_extract_policy(
            &self,
            _request: &AgentWrapperRunRequest,
        ) -> Result<Self::Policy, crate::AgentWrapperError> {
            panic!("validate_and_extract_policy must not be called for unsupported keys");
        }

        type BackendEvent = ToyEvent;
        type BackendCompletion = ToyCompletion;
        type BackendError = ToyBackendError;

        fn spawn(
            &self,
            _req: NormalizedRequest<Self::Policy>,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<
                            super::super::contract::BackendSpawn<
                                Self::BackendEvent,
                                Self::BackendCompletion,
                                Self::BackendError,
                            >,
                            Self::BackendError,
                        >,
                    > + Send
                    + 'static,
            >,
        > {
            panic!("spawn must not be called from normalize_request");
        }

        fn map_event(&self, _event: Self::BackendEvent) -> Vec<crate::AgentWrapperEvent> {
            panic!("map_event must not be called from normalize_request");
        }

        fn map_completion(
            &self,
            _completion: Self::BackendCompletion,
        ) -> Result<crate::AgentWrapperCompletion, crate::AgentWrapperError> {
            panic!("map_completion must not be called from normalize_request");
        }

        fn redact_error(
            &self,
            _phase: BackendHarnessErrorPhase,
            _err: &Self::BackendError,
        ) -> String {
            panic!("redact_error must not be called from normalize_request");
        }
    }

    let adapter = PanicOnPolicyAdapter;
    let defaults = BackendDefaults::default();

    for raw in [json!(false), json!("x".repeat(256))] {
        let mut request = AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            ..Default::default()
        };
        request
            .extensions
            .insert(MODEL_ID_KEY.to_string(), raw.clone());

        let err = match normalize_request(&adapter, &defaults, request) {
            Ok(_) => panic!("unsupported model key must fail closed"),
            Err(err) => err,
        };
        match err {
            AgentWrapperError::UnsupportedCapability {
                agent_kind,
                capability,
            } => {
                assert_eq!(agent_kind, "toy");
                assert_eq!(capability, MODEL_ID_KEY);
            }
            other => panic!("expected UnsupportedCapability, got: {other:?}"),
        }
    }
}

#[test]
fn bh_r0_agent_api_config_model_v1_is_confined_to_normalize_rs_in_production_code() {
    let src_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let allowed_normalize = src_root.join("backend_harness/normalize.rs");

    let mut files = Vec::new();
    collect_rs_files(&src_root, &mut files);

    let mut offenders = Vec::new();
    for path in files {
        let is_test_source = path
            .components()
            .any(|component| component.as_os_str() == "tests")
            || path.file_name().and_then(|name| name.to_str()) == Some("tests.rs");

        if is_test_source || path == allowed_normalize {
            continue;
        }

        let contents = fs::read_to_string(&path).expect("read source file");
        if contents.contains(MODEL_ID_KEY) {
            offenders.push(
                path.strip_prefix(&src_root)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }

    assert!(
        offenders.is_empty(),
        "agent_api.config.model.v1 leaked outside backend_harness/normalize.rs and its tests: {offenders:?}"
    );
}
