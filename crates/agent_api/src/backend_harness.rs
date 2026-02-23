#![allow(dead_code)]
#![allow(clippy::type_complexity)]

use std::{collections::BTreeMap, future::Future, pin::Pin, time::Duration};

use futures_core::Stream;
use futures_util::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::{
    AgentWrapperCompletion, AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind,
    AgentWrapperKind, AgentWrapperRunHandle, AgentWrapperRunRequest,
};

/// BH-C04 bounded channel default; pinned to preserve existing backend behavior.
const DEFAULT_EVENT_CHANNEL_CAPACITY: usize = 32;

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

fn pump_error_event(agent_kind: AgentWrapperKind, message: String) -> AgentWrapperEvent {
    AgentWrapperEvent {
        agent_kind,
        kind: AgentWrapperEventKind::Error,
        channel: Some("error".to_string()),
        text: None,
        message: Some(message),
        data: None,
    }
}

async fn pump_backend_events<A: BackendHarnessAdapter>(
    adapter: std::sync::Arc<A>,
    mut events: DynBackendEventStream<A::BackendEvent, A::BackendError>,
    tx: mpsc::Sender<AgentWrapperEvent>,
) {
    // BH-C04 (SEAM-3) pinned semantics:
    // - Forward mapped + bounds-enforced universal events while the receiver is alive.
    // - Receiver drop MUST be detected only via `tx.send(...).await` returning `Err(_)`.
    // - After the first send failure, stop forwarding entirely (no further mapping/bounds/sends),
    //   but keep draining the typed backend stream until it ends.
    // - Finality signal for DR-0012 gating is the drop of this `Sender`; the sender MUST be
    //   dropped only once the backend stream has ended (receiver drop is not finality).
    let mut forward = true;
    while let Some(outcome) = events.next().await {
        if !forward {
            continue;
        }

        let mapped: Vec<AgentWrapperEvent> = match outcome {
            Ok(ev) => adapter.map_event(ev),
            Err(err) => vec![pump_error_event(
                adapter.kind(),
                adapter.redact_error(BackendHarnessErrorPhase::Stream, &err),
            )],
        };

        for event in mapped {
            for bounded in crate::bounds::enforce_event_bounds(event) {
                if tx.send(bounded).await.is_err() {
                    forward = false;
                    break;
                }
            }
            if !forward {
                break;
            }
        }
    }

    // Finality signal (BH-C04): drop the sender only after the backend stream ends.
    drop(tx);
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

fn merge_env_backend_defaults_then_request(
    defaults: &BTreeMap<String, String>,
    request: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut merged = defaults.clone();
    merged.extend(request.clone());
    merged
}

fn derive_effective_timeout(
    request_timeout: Option<Duration>,
    default_timeout: Option<Duration>,
) -> Option<Duration> {
    request_timeout.or(default_timeout)
}

pub(crate) fn normalize_request<A: BackendHarnessAdapter>(
    adapter: &A,
    defaults: &BackendDefaults,
    request: AgentWrapperRunRequest,
) -> Result<NormalizedRequest<A::Policy>, AgentWrapperError> {
    if request.prompt.trim().is_empty() {
        return Err(AgentWrapperError::InvalidRequest {
            message: "prompt must not be empty".to_string(),
        });
    }

    validate_extension_keys_fail_closed(adapter, &request)?;
    let policy = adapter.validate_and_extract_policy(&request)?;

    let env = merge_env_backend_defaults_then_request(&defaults.env, &request.env);
    let effective_timeout = derive_effective_timeout(request.timeout, defaults.default_timeout);

    let agent_kind = adapter.kind();
    let prompt = request.prompt;
    let working_dir = request.working_dir;

    Ok(NormalizedRequest {
        agent_kind,
        prompt,
        working_dir,
        effective_timeout,
        env,
        policy,
    })
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
    use std::collections::VecDeque;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };

    use futures_util::StreamExt;
    use serde_json::json;
    use tokio::sync::{mpsc, oneshot};

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

    #[tokio::test]
    async fn pump_backend_events_smoke_forwards_in_order() {
        let adapter = std::sync::Arc::new(ToyAdapter { fail_spawn: false });
        let events = futures_util::stream::iter([
            Ok::<ToyEvent, ToyBackendError>(ToyEvent::Text("one".to_string())),
            Ok::<ToyEvent, ToyBackendError>(ToyEvent::Text("two".to_string())),
        ]);
        let events: DynBackendEventStream<_, _> = Box::pin(events);

        let (tx, mut rx) = mpsc::channel::<AgentWrapperEvent>(DEFAULT_EVENT_CHANNEL_CAPACITY);
        let handle = tokio::spawn(pump_backend_events(adapter, events, tx));

        let mut texts = Vec::<String>::new();
        while let Some(ev) = rx.recv().await {
            if ev.kind == AgentWrapperEventKind::TextOutput {
                if let Some(text) = ev.text {
                    texts.push(text);
                }
            }
        }

        handle.await.expect("pump task completes");
        assert_eq!(texts, vec!["one".to_string(), "two".to_string()]);
    }

    #[tokio::test]
    async fn pump_blocks_under_backpressure_until_receiver_polls() {
        #[derive(Default)]
        struct BackpressureAdapter {
            call_count: AtomicUsize,
            second_mapped_tx: Mutex<Option<oneshot::Sender<()>>>,
        }

        impl BackendHarnessAdapter for BackpressureAdapter {
            fn kind(&self) -> AgentWrapperKind {
                toy_kind()
            }

            fn supported_extension_keys(&self) -> &'static [&'static str] {
                &[]
            }

            type Policy = ();

            fn validate_and_extract_policy(
                &self,
                _request: &AgentWrapperRunRequest,
            ) -> Result<Self::Policy, AgentWrapperError> {
                Ok(())
            }

            type BackendEvent = String;
            type BackendCompletion = ();
            type BackendError = ();

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
                panic!("spawn unused in pump tests");
            }

            fn map_event(&self, event: Self::BackendEvent) -> Vec<AgentWrapperEvent> {
                let call = self.call_count.fetch_add(1, Ordering::SeqCst) + 1;
                if call == 2 {
                    if let Some(tx) = self.second_mapped_tx.lock().unwrap().take() {
                        let _ = tx.send(());
                    }
                }
                vec![AgentWrapperEvent {
                    agent_kind: toy_kind(),
                    kind: AgentWrapperEventKind::TextOutput,
                    channel: Some("assistant".to_string()),
                    text: Some(event),
                    message: None,
                    data: None,
                }]
            }

            fn map_completion(
                &self,
                _completion: Self::BackendCompletion,
            ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
                panic!("map_completion unused in pump tests");
            }

            fn redact_error(
                &self,
                _phase: BackendHarnessErrorPhase,
                _err: &Self::BackendError,
            ) -> String {
                "unused".to_string()
            }
        }

        let (second_mapped_tx, second_mapped_rx) = oneshot::channel::<()>();
        let adapter = std::sync::Arc::new(BackpressureAdapter {
            call_count: AtomicUsize::new(0),
            second_mapped_tx: Mutex::new(Some(second_mapped_tx)),
        });

        let events = futures_util::stream::iter([
            Ok::<String, ()>("one".to_string()),
            Ok::<String, ()>("two".to_string()),
        ]);
        let events: DynBackendEventStream<_, _> = Box::pin(events);

        let (tx, mut rx) = mpsc::channel::<AgentWrapperEvent>(1);
        let handle = tokio::spawn(pump_backend_events(adapter, events, tx));

        second_mapped_rx.await.expect("second event mapped");
        tokio::task::yield_now().await;
        assert!(
            !handle.is_finished(),
            "pump must be blocked on bounded send"
        );

        let mut texts = Vec::<String>::new();
        while let Some(ev) = rx.recv().await {
            if ev.kind == AgentWrapperEventKind::TextOutput {
                if let Some(text) = ev.text {
                    texts.push(text);
                }
            }
        }

        handle.await.expect("pump task completes");
        assert_eq!(texts, vec!["one".to_string(), "two".to_string()]);
    }

    struct CountingStream<E, BE> {
        items: VecDeque<Result<E, BE>>,
        consumed: std::sync::Arc<AtomicUsize>,
    }

    impl<E, BE> Unpin for CountingStream<E, BE> {}

    impl<E, BE> Stream for CountingStream<E, BE> {
        type Item = Result<E, BE>;

        fn poll_next(
            self: Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            let this = self.get_mut();
            let next = this.items.pop_front();
            if next.is_some() {
                this.consumed.fetch_add(1, Ordering::SeqCst);
            }
            std::task::Poll::Ready(next)
        }
    }

    #[tokio::test]
    async fn pump_stops_forwarding_after_receiver_drop_but_drains_to_end() {
        struct DrainAdapter;

        impl BackendHarnessAdapter for DrainAdapter {
            fn kind(&self) -> AgentWrapperKind {
                toy_kind()
            }

            fn supported_extension_keys(&self) -> &'static [&'static str] {
                &[]
            }

            type Policy = ();

            fn validate_and_extract_policy(
                &self,
                _request: &AgentWrapperRunRequest,
            ) -> Result<Self::Policy, AgentWrapperError> {
                Ok(())
            }

            type BackendEvent = String;
            type BackendCompletion = ();
            type BackendError = ();

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
                panic!("spawn unused in pump tests");
            }

            fn map_event(&self, event: Self::BackendEvent) -> Vec<AgentWrapperEvent> {
                vec![AgentWrapperEvent {
                    agent_kind: toy_kind(),
                    kind: AgentWrapperEventKind::TextOutput,
                    channel: Some("assistant".to_string()),
                    text: Some(event),
                    message: None,
                    data: None,
                }]
            }

            fn map_completion(
                &self,
                _completion: Self::BackendCompletion,
            ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
                panic!("map_completion unused in pump tests");
            }

            fn redact_error(
                &self,
                _phase: BackendHarnessErrorPhase,
                _err: &Self::BackendError,
            ) -> String {
                "unused".to_string()
            }
        }

        let total = 20usize;
        let consumed = std::sync::Arc::new(AtomicUsize::new(0));
        let items: VecDeque<Result<String, ()>> = (0..total)
            .map(|i| Ok::<String, ()>(format!("ev-{i}")))
            .collect();

        let events = CountingStream {
            items,
            consumed: consumed.clone(),
        };
        let events: DynBackendEventStream<_, _> = Box::pin(events);

        let adapter = std::sync::Arc::new(DrainAdapter);
        let (tx, mut rx) = mpsc::channel::<AgentWrapperEvent>(1);
        let handle = tokio::spawn(pump_backend_events(adapter, events, tx));

        let first = rx.recv().await.expect("at least one forwarded event");
        assert_eq!(first.kind, AgentWrapperEventKind::TextOutput);
        drop(rx);

        handle.await.expect("pump task completes");
        assert_eq!(
            consumed.load(Ordering::SeqCst),
            total,
            "backend stream must be fully drained after receiver drop"
        );
    }

    #[tokio::test]
    async fn pump_enforces_bounds_before_forwarding() {
        struct BoundsAdapter;

        impl BackendHarnessAdapter for BoundsAdapter {
            fn kind(&self) -> AgentWrapperKind {
                toy_kind()
            }

            fn supported_extension_keys(&self) -> &'static [&'static str] {
                &[]
            }

            type Policy = ();

            fn validate_and_extract_policy(
                &self,
                _request: &AgentWrapperRunRequest,
            ) -> Result<Self::Policy, AgentWrapperError> {
                Ok(())
            }

            type BackendEvent = ();
            type BackendCompletion = ();
            type BackendError = ();

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
                panic!("spawn unused in pump tests");
            }

            fn map_event(&self, _event: Self::BackendEvent) -> Vec<AgentWrapperEvent> {
                vec![AgentWrapperEvent {
                    agent_kind: toy_kind(),
                    kind: AgentWrapperEventKind::Error,
                    channel: Some("error".to_string()),
                    text: None,
                    message: Some("a".repeat(crate::bounds::MESSAGE_BOUND_BYTES + 10)),
                    data: None,
                }]
            }

            fn map_completion(
                &self,
                _completion: Self::BackendCompletion,
            ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
                panic!("map_completion unused in pump tests");
            }

            fn redact_error(
                &self,
                _phase: BackendHarnessErrorPhase,
                _err: &Self::BackendError,
            ) -> String {
                "unused".to_string()
            }
        }

        let events = futures_util::stream::iter([Ok::<(), ()>(())]);
        let events: DynBackendEventStream<_, _> = Box::pin(events);

        let adapter = std::sync::Arc::new(BoundsAdapter);
        let (tx, mut rx) = mpsc::channel::<AgentWrapperEvent>(8);
        let handle = tokio::spawn(pump_backend_events(adapter, events, tx));

        let ev = rx.recv().await.expect("one event forwarded");
        let message = ev.message.as_deref().expect("message present");
        assert!(message.len() <= crate::bounds::MESSAGE_BOUND_BYTES);
        assert!(message.ends_with("…(truncated)"));

        while rx.recv().await.is_some() {}
        handle.await.expect("pump task completes");
    }

    struct ControlledEndStream<E, BE> {
        first: Option<Result<E, BE>>,
        finish_rx: oneshot::Receiver<()>,
    }

    impl<E, BE> Unpin for ControlledEndStream<E, BE> {}

    impl<E, BE> Stream for ControlledEndStream<E, BE> {
        type Item = Result<E, BE>;

        fn poll_next(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            let this = self.get_mut();
            if let Some(first) = this.first.take() {
                return std::task::Poll::Ready(Some(first));
            }

            match Pin::new(&mut this.finish_rx).poll(cx) {
                std::task::Poll::Ready(_) => std::task::Poll::Ready(None),
                std::task::Poll::Pending => std::task::Poll::Pending,
            }
        }
    }

    #[tokio::test]
    async fn pump_finality_sender_dropped_only_after_backend_stream_ends() {
        struct FinalityAdapter;

        impl BackendHarnessAdapter for FinalityAdapter {
            fn kind(&self) -> AgentWrapperKind {
                toy_kind()
            }

            fn supported_extension_keys(&self) -> &'static [&'static str] {
                &[]
            }

            type Policy = ();

            fn validate_and_extract_policy(
                &self,
                _request: &AgentWrapperRunRequest,
            ) -> Result<Self::Policy, AgentWrapperError> {
                Ok(())
            }

            type BackendEvent = String;
            type BackendCompletion = ();
            type BackendError = ();

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
                panic!("spawn unused in pump tests");
            }

            fn map_event(&self, event: Self::BackendEvent) -> Vec<AgentWrapperEvent> {
                vec![AgentWrapperEvent {
                    agent_kind: toy_kind(),
                    kind: AgentWrapperEventKind::TextOutput,
                    channel: Some("assistant".to_string()),
                    text: Some(event),
                    message: None,
                    data: None,
                }]
            }

            fn map_completion(
                &self,
                _completion: Self::BackendCompletion,
            ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
                panic!("map_completion unused in pump tests");
            }

            fn redact_error(
                &self,
                _phase: BackendHarnessErrorPhase,
                _err: &Self::BackendError,
            ) -> String {
                "unused".to_string()
            }
        }

        let (finish_tx, finish_rx) = oneshot::channel::<()>();
        let events = ControlledEndStream::<String, ()> {
            first: Some(Ok::<String, ()>("hello".to_string())),
            finish_rx,
        };
        let events: DynBackendEventStream<_, _> = Box::pin(events);

        let adapter = std::sync::Arc::new(FinalityAdapter);
        let (tx, mut rx) = mpsc::channel::<AgentWrapperEvent>(8);
        let handle = tokio::spawn(pump_backend_events(adapter, events, tx));

        let ev = rx.recv().await.expect("first event forwarded");
        assert_eq!(ev.kind, AgentWrapperEventKind::TextOutput);

        tokio::task::yield_now().await;
        assert!(
            matches!(
                rx.try_recv(),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty)
            ),
            "events stream must not be final before backend stream ends"
        );
        assert!(
            !handle.is_finished(),
            "pump must not finish before stream end"
        );

        let _ = finish_tx.send(());
        handle.await.expect("pump task completes");
        assert!(
            rx.recv().await.is_none(),
            "events stream must be final after backend stream ends"
        );
    }

    #[test]
    fn bh_c02_unknown_extension_key_is_rejected_via_normalize_request() {
        struct PanicOnPolicyAdapter;

        impl BackendHarnessAdapter for PanicOnPolicyAdapter {
            fn kind(&self) -> AgentWrapperKind {
                toy_kind()
            }

            fn supported_extension_keys(&self) -> &'static [&'static str] {
                &["known.key"]
            }

            type Policy = ToyPolicy;

            fn validate_and_extract_policy(
                &self,
                _request: &AgentWrapperRunRequest,
            ) -> Result<Self::Policy, AgentWrapperError> {
                panic!("validate_and_extract_policy must not be called for unknown keys");
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
                panic!("spawn must not be called from normalize_request");
            }

            fn map_event(&self, _event: Self::BackendEvent) -> Vec<AgentWrapperEvent> {
                panic!("map_event must not be called from normalize_request");
            }

            fn map_completion(
                &self,
                _completion: Self::BackendCompletion,
            ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
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
    fn bh_c02_multiple_unknown_extension_keys_report_lexicographically_smallest_via_normalize_request(
    ) {
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
    fn bh_c03_timeout_derivation_matrix_via_normalize_request() {
        let adapter = ToyAdapter { fail_spawn: false };

        struct Case {
            request: Option<Duration>,
            default: Option<Duration>,
            expected: Option<Duration>,
        }

        let cases = [
            Case {
                request: Some(Duration::from_secs(5)),
                default: Some(Duration::from_secs(7)),
                expected: Some(Duration::from_secs(5)),
            },
            Case {
                request: Some(Duration::from_secs(5)),
                default: None,
                expected: Some(Duration::from_secs(5)),
            },
            Case {
                request: None,
                default: Some(Duration::from_secs(7)),
                expected: Some(Duration::from_secs(7)),
            },
            Case {
                request: None,
                default: None,
                expected: None,
            },
        ];

        for case in cases {
            let defaults = BackendDefaults {
                env: BTreeMap::new(),
                default_timeout: case.default,
            };
            let request = AgentWrapperRunRequest {
                prompt: "hello".to_string(),
                timeout: case.request,
                ..Default::default()
            };
            let normalized = normalize_request(&adapter, &defaults, request).expect("normalizes");
            assert_eq!(normalized.effective_timeout, case.expected);
        }
    }

    #[test]
    fn bh_c03_timeout_duration_zero_is_preserved_via_normalize_request() {
        let adapter = ToyAdapter { fail_spawn: false };
        let defaults = BackendDefaults {
            env: BTreeMap::new(),
            default_timeout: Some(Duration::from_secs(7)),
        };
        let request = AgentWrapperRunRequest {
            prompt: "hello".to_string(),
            timeout: Some(Duration::ZERO),
            ..Default::default()
        };
        let normalized = normalize_request(&adapter, &defaults, request).expect("normalizes");
        assert_eq!(normalized.effective_timeout, Some(Duration::ZERO));
    }

    #[test]
    fn universal_invalid_request_empty_prompt_short_circuits_allowlist_and_policy() {
        struct PanicOnAllowlistAdapter;

        impl BackendHarnessAdapter for PanicOnAllowlistAdapter {
            fn kind(&self) -> AgentWrapperKind {
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
                panic!("spawn must not be called from normalize_request");
            }

            fn map_event(&self, _event: Self::BackendEvent) -> Vec<AgentWrapperEvent> {
                panic!("map_event must not be called from normalize_request");
            }

            fn map_completion(
                &self,
                _completion: Self::BackendCompletion,
            ) -> Result<AgentWrapperCompletion, AgentWrapperError> {
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
}
