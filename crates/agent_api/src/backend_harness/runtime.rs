use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use futures_util::StreamExt;
use tokio::sync::{mpsc, oneshot, Notify};

use super::normalize_request;
use super::{
    BackendDefaults, BackendHarnessAdapter, BackendHarnessErrorPhase, BackendSpawn,
    DynBackendCompletionFuture, DynBackendEventStream, NormalizedRequest,
};
use crate::{
    AgentWrapperCancelHandle, AgentWrapperCompletion, AgentWrapperError, AgentWrapperEvent,
    AgentWrapperEventKind, AgentWrapperRunControl, AgentWrapperRunHandle, AgentWrapperRunRequest,
};

#[allow(dead_code)]
type RequestTerminationHook = Arc<dyn Fn() + Send + Sync + 'static>;

#[derive(Clone)]
#[allow(dead_code)]
struct HarnessCancelSignal {
    inner: Arc<HarnessCancelInner>,
}

#[allow(dead_code)]
struct HarnessCancelInner {
    cancelled: AtomicBool,
    notify: Notify,
}

#[allow(dead_code)]
impl HarnessCancelSignal {
    fn new() -> Self {
        Self {
            inner: Arc::new(HarnessCancelInner {
                cancelled: AtomicBool::new(false),
                notify: Notify::new(),
            }),
        }
    }

    fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    fn cancel(&self) {
        if self.inner.cancelled.swap(true, Ordering::SeqCst) {
            return;
        }
        self.inner.notify.notify_waiters();
    }

    async fn cancelled(&self) {
        let notified = self.inner.notify.notified();
        if self.is_cancelled() {
            return;
        }
        notified.await;
    }
}

fn pump_error_event(agent_kind: crate::AgentWrapperKind, message: String) -> AgentWrapperEvent {
    AgentWrapperEvent {
        agent_kind,
        kind: AgentWrapperEventKind::Error,
        channel: Some("error".to_string()),
        text: None,
        message: Some(message),
        data: None,
    }
}

#[allow(dead_code)]
fn cancelled_completion_error() -> AgentWrapperError {
    AgentWrapperError::Backend {
        message: "cancelled".to_string(),
    }
}

fn request_termination_best_effort(request_termination: Option<&RequestTerminationHook>) {
    if let Some(request_termination) = request_termination {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (request_termination)();
        }));
    }
}

async fn surface_spawn_failure(
    agent_kind: crate::AgentWrapperKind,
    message: String,
    tx: mpsc::Sender<AgentWrapperEvent>,
    completion_tx: oneshot::Sender<Result<AgentWrapperCompletion, AgentWrapperError>>,
) {
    for bounded in
        crate::bounds::enforce_event_bounds(pump_error_event(agent_kind, message.clone()))
    {
        if tx.send(bounded).await.is_err() {
            break;
        }
    }
    drop(tx);
    let _ = completion_tx.send(Err(AgentWrapperError::Backend { message }));
}

async fn surface_control_startup_failure(
    agent_kind: crate::AgentWrapperKind,
    message: String,
    cancel_signal: HarnessCancelSignal,
    request_termination: Option<RequestTerminationHook>,
    tx: mpsc::Sender<AgentWrapperEvent>,
    completion_tx: oneshot::Sender<Result<AgentWrapperCompletion, AgentWrapperError>>,
) {
    if cancel_signal.is_cancelled() {
        request_termination_best_effort(request_termination.as_ref());
        drop(tx);
        let _ = completion_tx.send(Err(cancelled_completion_error()));
        return;
    }

    for bounded in
        crate::bounds::enforce_event_bounds(pump_error_event(agent_kind, message.clone()))
    {
        tokio::select! {
            biased;
            _ = cancel_signal.cancelled() => {
                request_termination_best_effort(request_termination.as_ref());
                drop(tx);
                let _ = completion_tx.send(Err(cancelled_completion_error()));
                return;
            }
            send_outcome = tx.send(bounded) => {
                if send_outcome.is_err() {
                    break;
                }
            }
        }
    }

    if cancel_signal.is_cancelled() {
        request_termination_best_effort(request_termination.as_ref());
        drop(tx);
        let _ = completion_tx.send(Err(cancelled_completion_error()));
        return;
    }

    drop(tx);
    let _ = completion_tx.send(Err(AgentWrapperError::Backend { message }));
}

async fn drive_control_backend_startup<A: BackendHarnessAdapter>(
    adapter: Arc<A>,
    normalized: NormalizedRequest<A::Policy>,
    agent_kind: crate::AgentWrapperKind,
    cancel_signal: HarnessCancelSignal,
    request_termination: Option<RequestTerminationHook>,
    tx: mpsc::Sender<AgentWrapperEvent>,
    completion_tx: oneshot::Sender<Result<AgentWrapperCompletion, AgentWrapperError>>,
) {
    let spawn = adapter.spawn(normalized);
    tokio::pin!(spawn);

    tokio::select! {
        biased;
        _ = cancel_signal.cancelled() => {
            request_termination_best_effort(request_termination.as_ref());
            // Dropping the in-flight startup future aborts startup probes/commands, which use
            // kill-on-drop wrapper clients.
            drop(tx);
            let _ = completion_tx.send(Err(cancelled_completion_error()));
        }
        spawn_outcome = &mut spawn => {
            let spawned = match spawn_outcome {
                Ok(spawned) => spawned,
                Err(err) => {
                    let message = adapter.redact_error(BackendHarnessErrorPhase::Spawn, &err);
                    surface_control_startup_failure(
                        agent_kind,
                        message,
                        cancel_signal,
                        request_termination,
                        tx,
                        completion_tx,
                    )
                    .await;
                    return;
                }
            };

            let BackendSpawn { events, completion } = spawned;

            tokio::spawn(send_completion_with_cancel(
                adapter.clone(),
                completion,
                cancel_signal.clone(),
                request_termination,
                completion_tx,
            ));

            tokio::spawn(pump_backend_events_with_cancel(
                adapter,
                events,
                tx,
                cancel_signal,
            ));
        }
    }
}

fn start_backend_runtime<A: BackendHarnessAdapter>(
    adapter: Arc<A>,
    spawned: BackendSpawn<A::BackendEvent, A::BackendCompletion, A::BackendError>,
    tx: mpsc::Sender<AgentWrapperEvent>,
    completion_tx: oneshot::Sender<Result<AgentWrapperCompletion, AgentWrapperError>>,
) {
    let BackendSpawn { events, completion } = spawned;

    tokio::spawn({
        let adapter = adapter.clone();
        async move {
            let completion_outcome = completion.await;
            let completion_outcome: Result<AgentWrapperCompletion, AgentWrapperError> =
                match completion_outcome {
                    Ok(typed) => adapter.map_completion(typed),
                    Err(err) => Err(AgentWrapperError::Backend {
                        message: adapter.redact_error(BackendHarnessErrorPhase::Completion, &err),
                    }),
                }
                .map(crate::bounds::enforce_completion_bounds);

            let _ = completion_tx.send(completion_outcome);
        }
    });

    tokio::spawn(pump_backend_events(adapter, events, tx));
}

async fn pump_backend_events<A: BackendHarnessAdapter>(
    adapter: Arc<A>,
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

#[allow(dead_code)]
async fn pump_backend_events_with_cancel<A: BackendHarnessAdapter>(
    adapter: Arc<A>,
    mut events: DynBackendEventStream<A::BackendEvent, A::BackendError>,
    tx: mpsc::Sender<AgentWrapperEvent>,
    cancel: HarnessCancelSignal,
) {
    // CA-C02 (SEAM-2) semantics:
    // - Explicit cancellation is orthogonal to receiver drop.
    // - On cancellation: stop forwarding immediately, proactively close the universal stream by
    //   dropping the sender, but keep draining the typed backend stream to completion.
    // - On receiver drop: preserve BH-C04 drain-on-drop posture (stop forwarding on first send
    //   failure, but keep draining; sender is dropped only after stream end).
    let mut tx = Some(tx);
    let mut forward = true;
    let mut cancelled = false;

    loop {
        tokio::select! {
            biased;
            _ = cancel.cancelled(), if !cancelled => {
                cancelled = true;
                forward = false;
                // Close the universal stream (consumer sees `None`) without affecting drain.
                drop(tx.take());
            }
            maybe_outcome = events.next() => {
                let Some(outcome) = maybe_outcome else { break; };
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

                'mapped: for event in mapped {
                    for bounded in crate::bounds::enforce_event_bounds(event) {
                        let Some(sender) = tx.clone() else {
                            forward = false;
                            break 'mapped;
                        };

                        tokio::select! {
                            biased;
                            _ = cancel.cancelled(), if !cancelled => {
                                cancelled = true;
                                forward = false;
                                drop(tx.take());
                                break 'mapped;
                            }
                            send_outcome = sender.send(bounded) => {
                                if send_outcome.is_err() {
                                    forward = false;
                                    break 'mapped;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // BH-C04 finality posture: if cancellation never occurred, drop the sender only after typed
    // backend stream end.
    drop(tx);
}

#[allow(dead_code)]
async fn send_completion_with_cancel<A: BackendHarnessAdapter>(
    adapter: Arc<A>,
    mut completion: DynBackendCompletionFuture<A::BackendCompletion, A::BackendError>,
    cancel: HarnessCancelSignal,
    request_termination: Option<RequestTerminationHook>,
    completion_tx: oneshot::Sender<Result<AgentWrapperCompletion, AgentWrapperError>>,
) {
    // CA-C02 completion sender semantics:
    // - Cancellation selects the pinned `"cancelled"` error if cancellation is requested before
    //   backend completion is ready; cancellation wins ties.
    // - Completion timing MUST still be gated by backend process exit; cancellation changes value
    //   selection, not timing.
    tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            request_termination_best_effort(request_termination.as_ref());

            // Still await backend completion (process exit), then override the completion value.
            let _ = completion.await;
            let _ = completion_tx.send(Err(cancelled_completion_error()));
        }
        completion_outcome = &mut completion => {
            let completion_outcome: Result<AgentWrapperCompletion, AgentWrapperError> =
                match completion_outcome {
                    Ok(typed) => adapter.map_completion(typed),
                    Err(err) => Err(AgentWrapperError::Backend {
                        message: adapter.redact_error(BackendHarnessErrorPhase::Completion, &err),
                    }),
                }
                .map(crate::bounds::enforce_completion_bounds);

            let _ = completion_tx.send(completion_outcome);
        }
    }
}

#[allow(dead_code)]
pub(crate) async fn run_harnessed_backend_control<A: BackendHarnessAdapter>(
    adapter: Arc<A>,
    defaults: BackendDefaults,
    request: AgentWrapperRunRequest,
    request_termination: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
) -> Result<AgentWrapperRunControl, AgentWrapperError> {
    let normalized = normalize_request(adapter.as_ref(), &defaults, request)?;
    let agent_kind = normalized.agent_kind.clone();
    let (tx, rx) = mpsc::channel::<AgentWrapperEvent>(super::DEFAULT_EVENT_CHANNEL_CAPACITY);
    let (completion_tx, completion_rx) =
        oneshot::channel::<Result<AgentWrapperCompletion, AgentWrapperError>>();

    let cancel_signal = HarnessCancelSignal::new();
    let cancel = AgentWrapperCancelHandle::new({
        let cancel_signal = cancel_signal.clone();
        move || cancel_signal.cancel()
    });

    tokio::spawn(drive_control_backend_startup(
        adapter,
        normalized,
        agent_kind,
        cancel_signal,
        request_termination,
        tx,
        completion_tx,
    ));

    Ok(AgentWrapperRunControl {
        handle: crate::run_handle_gate::build_gated_run_handle(rx, completion_rx),
        cancel,
    })
}

pub(crate) async fn run_harnessed_backend<A: BackendHarnessAdapter>(
    adapter: Arc<A>,
    defaults: BackendDefaults,
    request: AgentWrapperRunRequest,
) -> Result<AgentWrapperRunHandle, AgentWrapperError> {
    let normalized = normalize_request(adapter.as_ref(), &defaults, request)?;
    let agent_kind = normalized.agent_kind.clone();
    let (tx, rx) = mpsc::channel::<AgentWrapperEvent>(super::DEFAULT_EVENT_CHANNEL_CAPACITY);
    let (completion_tx, completion_rx) =
        oneshot::channel::<Result<AgentWrapperCompletion, AgentWrapperError>>();

    tokio::spawn(async move {
        let spawn_outcome = adapter.spawn(normalized).await;
        let spawned = match spawn_outcome {
            Ok(spawned) => spawned,
            Err(err) => {
                let message = adapter.redact_error(BackendHarnessErrorPhase::Spawn, &err);
                surface_spawn_failure(agent_kind, message, tx, completion_tx).await;
                return;
            }
        };

        start_backend_runtime(adapter, spawned, tx, completion_tx);
    });

    Ok(crate::run_handle_gate::build_gated_run_handle(
        rx,
        completion_rx,
    ))
}

#[cfg(test)]
mod tests;
