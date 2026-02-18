#![cfg(any(feature = "codex", feature = "claude_code"))]

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use agent_api::{
    AgentWrapperCompletion, AgentWrapperEvent, AgentWrapperEventKind, AgentWrapperKind,
};
use futures_core::Stream;
use tokio::sync::{mpsc, oneshot};

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

fn noop_waker() -> Waker {
    unsafe fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    unsafe fn wake(_: *const ()) {}
    unsafe fn wake_by_ref(_: *const ()) {}
    unsafe fn drop(_: *const ()) {}

    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

fn block_on_ready<F: Future>(mut future: F) -> F::Output {
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut future = unsafe { Pin::new_unchecked(&mut future) };

    for _ in 0..64 {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::yield_now();
    }

    panic!("future did not resolve quickly (expected Ready)");
}

fn drain_to_none(mut stream: Pin<&mut (dyn Stream<Item = AgentWrapperEvent> + Send)>) {
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    loop {
        match stream.as_mut().poll_next(&mut context) {
            Poll::Ready(Some(_)) => continue,
            Poll::Ready(None) => break,
            Poll::Pending => {
                std::thread::yield_now();
            }
        }
    }
}

#[test]
fn completion_is_pending_until_events_stream_is_drained_to_none() {
    let (tx, rx) = mpsc::channel::<AgentWrapperEvent>(32);
    tx.try_send(AgentWrapperEvent {
        agent_kind: AgentWrapperKind::new("dummy").unwrap(),
        kind: AgentWrapperEventKind::Status,
        channel: None,
        text: None,
        message: Some("hello".to_string()),
        data: None,
    })
    .unwrap();
    drop(tx);

    let (completion_tx, completion_rx) =
        oneshot::channel::<Result<AgentWrapperCompletion, agent_api::AgentWrapperError>>();
    completion_tx
        .send(Ok(AgentWrapperCompletion {
            status: success_exit_status(),
            final_text: None,
            data: None,
        }))
        .unwrap();

    let mut handle = agent_api::__test_support::build_gated_run_handle(rx, completion_rx);

    {
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);
        assert!(matches!(
            handle.completion.as_mut().poll(&mut context),
            Poll::Pending
        ));
    }

    drain_to_none(handle.events.as_mut());

    let completion_result = block_on_ready(handle.completion);
    assert!(completion_result.is_ok());
}

#[test]
fn dropping_events_stream_unblocks_completion() {
    let (tx, rx) = mpsc::channel::<AgentWrapperEvent>(32);
    tx.try_send(AgentWrapperEvent {
        agent_kind: AgentWrapperKind::new("dummy").unwrap(),
        kind: AgentWrapperEventKind::Status,
        channel: None,
        text: None,
        message: Some("hello".to_string()),
        data: None,
    })
    .unwrap();
    drop(tx);

    let (completion_tx, completion_rx) =
        oneshot::channel::<Result<AgentWrapperCompletion, agent_api::AgentWrapperError>>();
    completion_tx
        .send(Ok(AgentWrapperCompletion {
            status: success_exit_status(),
            final_text: None,
            data: None,
        }))
        .unwrap();

    let handle = agent_api::__test_support::build_gated_run_handle(rx, completion_rx);
    let agent_api::AgentWrapperRunHandle { events, completion } = handle;

    drop(events);

    let completion_result = block_on_ready(completion);
    assert!(completion_result.is_ok());
}
