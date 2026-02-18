use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use tokio::sync::{mpsc, oneshot};

use crate::{
    AgentWrapperCompletion, AgentWrapperError, AgentWrapperEvent, AgentWrapperRunHandle,
    DynAgentWrapperCompletion, DynAgentWrapperEventStream,
};

pub(crate) fn build_gated_run_handle(
    rx: mpsc::Receiver<AgentWrapperEvent>,
    completion_rx: oneshot::Receiver<Result<AgentWrapperCompletion, AgentWrapperError>>,
) -> AgentWrapperRunHandle {
    let (events_done_tx, events_done_rx) = oneshot::channel::<()>();

    let events: DynAgentWrapperEventStream = Box::pin(FinalityEventStream {
        rx,
        events_done_tx: Some(events_done_tx),
    });

    let completion: DynAgentWrapperCompletion = Box::pin(async move {
        let result = completion_rx.await.unwrap_or_else(|_| {
            Err(AgentWrapperError::Backend {
                message: "completion channel dropped".to_string(),
            })
        });

        let _ = events_done_rx.await;
        result
    });

    AgentWrapperRunHandle { events, completion }
}

struct FinalityEventStream {
    rx: mpsc::Receiver<AgentWrapperEvent>,
    events_done_tx: Option<oneshot::Sender<()>>,
}

impl FinalityEventStream {
    fn signal_done(&mut self) {
        if let Some(tx) = self.events_done_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Stream for FinalityEventStream {
    type Item = AgentWrapperEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let poll = Pin::new(&mut self.rx).poll_recv(cx);
        if let Poll::Ready(None) = poll {
            self.signal_done();
        }
        poll
    }
}

impl Drop for FinalityEventStream {
    fn drop(&mut self) {
        self.signal_done();
    }
}
