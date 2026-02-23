#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use std::{collections::BTreeMap, future::Future, pin::Pin, time::Duration};

use futures_core::Stream;
use serde_json::Value;

use crate::{
    AgentWrapperCompletion, AgentWrapperError, AgentWrapperEvent, AgentWrapperKind,
    AgentWrapperRunHandle, AgentWrapperRunRequest,
};

pub(crate) type DynBackendEventStream<E, BE> =
    Pin<Box<dyn Stream<Item = Result<E, BE>> + Send + 'static>>;

pub(crate) type DynBackendCompletionFuture<C, BE> =
    Pin<Box<dyn Future<Output = Result<C, BE>> + Send + 'static>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BackendHarnessErrorPhase {
    Spawn,
    Stream,
    Completion,
}

pub(crate) struct BackendSpawn<E, C, BE> {
    pub events: DynBackendEventStream<E, BE>,
    pub completion: DynBackendCompletionFuture<C, BE>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct BackendDefaults {
    pub env: BTreeMap<String, String>,
    pub default_timeout: Option<Duration>,
}

pub(crate) struct NormalizedRequest<P> {
    /// Stable identity for error reporting and event stamping.
    pub agent_kind: AgentWrapperKind,

    /// Preserved from `AgentWrapperRunRequest` (must be non-empty after trimming).
    pub prompt: String,

    /// Preserved from `AgentWrapperRunRequest` (no harness defaulting in v1).
    pub working_dir: Option<std::path::PathBuf>,

    /// Derived per BH-C03. `Some(Duration::ZERO)` is an explicit “no timeout” request.
    pub effective_timeout: Option<Duration>,

    /// Derived per BH-C03: `defaults.env` overridden by `request.env`.
    pub env: BTreeMap<String, String>,

    /// Backend-owned extracted policy derived from `request.extensions` after the allowlist check.
    pub policy: P,
}

pub(crate) trait BackendHarnessAdapter: Send + Sync + 'static {
    /// MUST return a stable, lower_snake_case id (see `AgentWrapperKind` rules).
    fn kind(&self) -> AgentWrapperKind;

    /// Supported extension keys for this backend (exact string match; case-sensitive).
    ///
    /// This list MUST include both:
    /// - core keys under `agent_api.*` that the backend supports, and
    /// - backend keys under `backend.<agent_kind>.*` owned by the backend.
    fn supported_extension_keys(&self) -> &'static [&'static str];

    /// Backend-owned policy extracted from known extension keys only.
    ///
    /// This hook MUST NOT implement “unknown key” rejection (that is BH-C02, harness-owned).
    type Policy: Send + 'static;

    fn validate_and_extract_policy(
        &self,
        request: &AgentWrapperRunRequest,
    ) -> Result<Self::Policy, AgentWrapperError>;

    /// Typed backend event and completion types emitted by the wrapper runtime.
    type BackendEvent: Send + 'static;
    type BackendCompletion: Send + 'static;

    /// Backend error type used at spawn/stream/completion boundaries.
    type BackendError: Send + Sync + 'static;

    /// Spawns the backend run using only the normalized request.
    ///
    /// The returned stream MUST be drained to completion by the harness pump (BH-C04).
    fn spawn(
        &self,
        req: NormalizedRequest<Self::Policy>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        BackendSpawn<
                            Self::BackendEvent,
                            Self::BackendCompletion,
                            Self::BackendError,
                        >,
                        Self::BackendError,
                    >,
                > + Send
                + 'static,
        >,
    >;

    /// Maps one typed backend event into 0..N universal events.
    ///
    /// Mapping is **infallible** by contract: backends MUST convert parse errors into
    /// `BackendError` at the stream boundary, not here.
    fn map_event(&self, event: Self::BackendEvent) -> Vec<AgentWrapperEvent>;

    /// Maps a typed backend completion value to the universal completion payload.
    fn map_completion(
        &self,
        completion: Self::BackendCompletion,
    ) -> Result<AgentWrapperCompletion, AgentWrapperError>;

    /// Produces a safe/redacted message for a backend error at a given phase.
    ///
    /// This message MUST NOT contain raw backend stdout/stderr lines or raw JSONL lines.
    /// It MAY include bounded metadata such as `line_bytes=<n>` or a stable error kind tag.
    fn redact_error(&self, phase: BackendHarnessErrorPhase, err: &Self::BackendError) -> String;
}

fn validate_extension_keys_fail_closed<A: BackendHarnessAdapter>(
    adapter: &A,
    request: &AgentWrapperRunRequest,
) -> Result<(), AgentWrapperError> {
    let supported: &[&str] = adapter.supported_extension_keys();
    for key in request.extensions.keys() {
        if !supported.contains(&key.as_str()) {
            return Err(AgentWrapperError::UnsupportedCapability {
                agent_kind: adapter.kind().as_str().to_string(),
                capability: key.clone(),
            });
        }
    }
    Ok(())
}

fn validate_and_extract_policy_pre_spawn<A: BackendHarnessAdapter>(
    adapter: &A,
    request: &AgentWrapperRunRequest,
) -> Result<A::Policy, AgentWrapperError> {
    if request.prompt.trim().is_empty() {
        return Err(AgentWrapperError::InvalidRequest {
            message: "prompt must not be empty".to_string(),
        });
    }

    validate_extension_keys_fail_closed(adapter, request)?;
    adapter.validate_and_extract_policy(request)
}

fn parse_ext_bool(value: &Value, key: &str) -> Result<bool, AgentWrapperError> {
    value
        .as_bool()
        .ok_or_else(|| AgentWrapperError::InvalidRequest {
            message: format!("{key} must be a boolean"),
        })
}

fn parse_ext_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, AgentWrapperError> {
    value
        .as_str()
        .ok_or_else(|| AgentWrapperError::InvalidRequest {
            message: format!("{key} must be a string"),
        })
}

fn parse_ext_string_enum<'a>(
    value: &'a Value,
    key: &str,
    allowed: &[&str],
) -> Result<&'a str, AgentWrapperError> {
    let raw = parse_ext_string(value, key)?;
    if allowed.contains(&raw) {
        return Ok(raw);
    }

    let allowed = allowed.join(" | ");
    Err(AgentWrapperError::InvalidRequest {
        message: format!("{key} must be one of: {allowed}"),
    })
}

pub(crate) fn run_harnessed_backend<A: BackendHarnessAdapter>(
    _adapter: std::sync::Arc<A>,
    _defaults: BackendDefaults,
    _request: AgentWrapperRunRequest,
) -> Result<AgentWrapperRunHandle, AgentWrapperError> {
    Err(AgentWrapperError::Backend {
        message: "backend harness not implemented (BH-C01 contract-only)".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use futures_util::StreamExt;
    use serde_json::json;

    use super::*;
    use crate::AgentWrapperEventKind;

    fn success_exit_status() -> std::process::ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(0)
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(0)
        }
    }

    fn toy_kind() -> AgentWrapperKind {
        AgentWrapperKind::new("toy").expect("toy kind is valid")
    }

    struct ToyAdapter {
        fail_spawn: bool,
    }

    struct ToyPolicy;

    enum ToyEvent {
        Text(String),
    }

    struct ToyCompletion;

    #[derive(Debug)]
    struct ToyBackendError {
        secret: String,
    }

    impl BackendHarnessAdapter for ToyAdapter {
        fn kind(&self) -> AgentWrapperKind {
            toy_kind()
        }

        fn supported_extension_keys(&self) -> &'static [&'static str] {
            &["agent_api.exec.non_interactive", "backend.toy.example"]
        }

        type Policy = ToyPolicy;

        fn validate_and_extract_policy(
            &self,
            _request: &AgentWrapperRunRequest,
        ) -> Result<Self::Policy, AgentWrapperError> {
            Ok(ToyPolicy)
        }

        type BackendEvent = ToyEvent;
        type BackendCompletion = ToyCompletion;
        type BackendError = ToyBackendError;

        fn spawn(
            &self,
            _req: NormalizedRequest<Self::Policy>,
        ) -> Pin<
            Box<
                dyn Future<
                        Output = Result<
                            BackendSpawn<
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
            let fail_spawn = self.fail_spawn;
            Box::pin(async move {
                if fail_spawn {
                    return Err(ToyBackendError {
                        secret: "SECRET_SPAWN".to_string(),
                    });
                }

                let events = futures_util::stream::iter([
                    Ok(ToyEvent::Text("one".to_string())),
                    Ok(ToyEvent::Text("two".to_string())),
                ]);

                Ok(BackendSpawn {
                    events: Box::pin(events),
                    completion: Box::pin(async move { Ok(ToyCompletion) }),
                })
            })
        }

        fn map_event(&self, event: Self::BackendEvent) -> Vec<AgentWrapperEvent> {
            match event {
                ToyEvent::Text(text) => vec![AgentWrapperEvent {
                    agent_kind: toy_kind(),
                    kind: AgentWrapperEventKind::TextOutput,
                    channel: Some("assistant".to_string()),
                    text: Some(text),
                    message: None,
                    data: None,
                }],
            }
        }

        fn map_completion(
            &self,
            _completion: Self::BackendCompletion,
        ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
            Ok(AgentWrapperCompletion {
                status: success_exit_status(),
                final_text: Some("done".to_string()),
                data: None,
            })
        }

        fn redact_error(
            &self,
            phase: BackendHarnessErrorPhase,
            _err: &Self::BackendError,
        ) -> String {
            let phase = match phase {
                BackendHarnessErrorPhase::Spawn => "spawn",
                BackendHarnessErrorPhase::Stream => "stream",
                BackendHarnessErrorPhase::Completion => "completion",
            };
            format!("toy backend error (redacted): phase={phase}")
        }
    }

    #[tokio::test]
    async fn toy_adapter_success_smoke() {
        let adapter = ToyAdapter { fail_spawn: false };

        let request = AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            ..Default::default()
        };
        let policy = adapter
            .validate_and_extract_policy(&request)
            .expect("policy extraction succeeds");

        let req = NormalizedRequest {
            agent_kind: adapter.kind(),
            prompt: "hello".to_string(),
            working_dir: None,
            effective_timeout: None,
            env: BTreeMap::new(),
            policy,
        };

        let spawned = adapter.spawn(req).await.expect("spawn succeeds");

        let mut universal = Vec::<AgentWrapperEvent>::new();
        let mut events = spawned.events;
        while let Some(item) = events.next().await {
            let event = item.expect("toy stream yields Ok");
            universal.extend(adapter.map_event(event));
        }

        assert_eq!(universal.len(), 2);
        assert_eq!(universal[0].agent_kind.as_str(), "toy");
        assert_eq!(universal[0].kind, AgentWrapperEventKind::TextOutput);
        assert_eq!(universal[0].text.as_deref(), Some("one"));
        assert_eq!(universal[1].agent_kind.as_str(), "toy");
        assert_eq!(universal[1].kind, AgentWrapperEventKind::TextOutput);
        assert_eq!(universal[1].text.as_deref(), Some("two"));

        let completion = spawned.completion.await.expect("typed completion ok");
        let mapped = adapter
            .map_completion(completion)
            .expect("completion mapping ok");
        assert_eq!(mapped.final_text.as_deref(), Some("done"));
    }

    #[tokio::test]
    async fn toy_adapter_spawn_failure_is_redacted() {
        let adapter = ToyAdapter { fail_spawn: true };
        let req = NormalizedRequest {
            agent_kind: adapter.kind(),
            prompt: "hello".to_string(),
            working_dir: None,
            effective_timeout: None,
            env: BTreeMap::new(),
            policy: ToyPolicy,
        };

        let err = match adapter.spawn(req).await {
            Ok(_) => panic!("spawn expected to fail"),
            Err(err) => err,
        };
        let redacted = adapter.redact_error(BackendHarnessErrorPhase::Spawn, &err);
        assert!(!redacted.contains("SECRET_SPAWN"));
        assert!(redacted.contains("spawn"));
    }

    #[test]
    fn bh_c02_unknown_extension_key_is_rejected() {
        let adapter = ToyAdapter { fail_spawn: false };
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
            .insert("backend.toy.unknown".to_string(), Value::Bool(true));

        let err = validate_extension_keys_fail_closed(&adapter, &request)
            .expect_err("unknown key must fail closed");
        match err {
            AgentWrapperError::UnsupportedCapability {
                agent_kind,
                capability,
            } => {
                assert_eq!(agent_kind, "toy");
                assert_eq!(capability, "backend.toy.unknown");
            }
            other => panic!("expected UnsupportedCapability, got: {other:?}"),
        }
    }

    #[test]
    fn bh_c02_multiple_unknown_extension_keys_report_lexicographically_smallest() {
        let adapter = ToyAdapter { fail_spawn: false };
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

        let err = validate_extension_keys_fail_closed(&adapter, &request)
            .expect_err("unknown key must fail closed");
        match err {
            AgentWrapperError::UnsupportedCapability { capability, .. } => {
                assert_eq!(capability, "aaa.unknown");
            }
            other => panic!("expected UnsupportedCapability, got: {other:?}"),
        }
    }

    #[test]
    fn bh_c02_all_keys_allowed_passes() {
        let adapter = ToyAdapter { fail_spawn: false };
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

        validate_extension_keys_fail_closed(&adapter, &request).expect("all keys allowed");
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
}
