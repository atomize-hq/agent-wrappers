use std::sync::Arc;

use futures_util::StreamExt;
use tokio::sync::{mpsc, oneshot};

use super::normalize_request;
use super::{
    BackendDefaults, BackendHarnessAdapter, BackendHarnessErrorPhase, BackendSpawn,
    DynBackendEventStream,
};
use crate::{
    AgentWrapperCompletion, AgentWrapperError, AgentWrapperEvent, AgentWrapperEventKind,
    AgentWrapperRunHandle, AgentWrapperRunRequest,
};

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

pub(crate) fn run_harnessed_backend<A: BackendHarnessAdapter>(
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
        let spawned = match adapter.spawn(normalized).await {
            Ok(spawned) => spawned,
            Err(err) => {
                let message = adapter.redact_error(BackendHarnessErrorPhase::Spawn, &err);
                for bounded in crate::bounds::enforce_event_bounds(pump_error_event(
                    agent_kind,
                    message.clone(),
                )) {
                    let _ = tx.send(bounded).await;
                }

                // Finality signal: there is no stream to drain; drop sender immediately.
                drop(tx);

                let _ = completion_tx.send(Err(AgentWrapperError::Backend { message }));
                return;
            }
        };

        let BackendSpawn { events, completion } = spawned;

        tokio::spawn({
            let adapter = adapter.clone();
            async move {
                let completion_outcome = completion.await;
                let completion_outcome: Result<AgentWrapperCompletion, AgentWrapperError> =
                    match completion_outcome {
                        Ok(typed) => adapter.map_completion(typed),
                        Err(err) => Err(AgentWrapperError::Backend {
                            message: adapter
                                .redact_error(BackendHarnessErrorPhase::Completion, &err),
                        }),
                    }
                    .map(crate::bounds::enforce_completion_bounds);

                let _ = completion_tx.send(completion_outcome);
            }
        });

        tokio::spawn(pump_backend_events(adapter, events, tx));
    });

    Ok(crate::run_handle_gate::build_gated_run_handle(
        rx,
        completion_rx,
    ))
}

#[cfg(test)]
mod tests;
