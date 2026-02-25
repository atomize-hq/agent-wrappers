use super::*;

use super::support::ControlledEndStream;

#[tokio::test]
async fn control_entrypoint_exposes_cancel_handle_wired_to_driver() {
    struct ControlAdapter {
        events_finish_rx: Mutex<Option<oneshot::Receiver<()>>>,
        backend_done_rx: Mutex<Option<oneshot::Receiver<()>>>,
    }

    impl BackendHarnessAdapter for ControlAdapter {
        fn kind(&self) -> crate::AgentWrapperKind {
            toy_kind()
        }

        fn supported_extension_keys(&self) -> &'static [&'static str] {
            &[]
        }

        type Policy = ();

        fn validate_and_extract_policy(
            &self,
            _request: &crate::AgentWrapperRunRequest,
        ) -> Result<Self::Policy, crate::AgentWrapperError> {
            Ok(())
        }

        type BackendEvent = String;
        type BackendCompletion = ();
        type BackendError = ();

        fn spawn(
            &self,
            _req: contract::NormalizedRequest<Self::Policy>,
        ) -> Pin<
            Box<
                dyn Future<
                        Output = Result<
                            contract::BackendSpawn<
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
            let finish_rx = self
                .events_finish_rx
                .lock()
                .unwrap()
                .take()
                .expect("events gate available once");
            let backend_done_rx = self
                .backend_done_rx
                .lock()
                .unwrap()
                .take()
                .expect("backend gate available once");

            let events = ControlledEndStream::<String, ()> {
                first: Some(Ok::<String, ()>("one".to_string())),
                finish_rx,
            };
            let completion: DynBackendCompletionFuture<(), ()> = Box::pin(async move {
                backend_done_rx.await.expect("backend completion released");
                Ok::<(), ()>(())
            });

            Box::pin(async move {
                Ok(BackendSpawn {
                    events: Box::pin(events),
                    completion,
                })
            })
        }

        fn map_event(&self, event: Self::BackendEvent) -> Vec<crate::AgentWrapperEvent> {
            vec![crate::AgentWrapperEvent {
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
        ) -> Result<crate::AgentWrapperCompletion, crate::AgentWrapperError> {
            Ok(crate::AgentWrapperCompletion {
                status: success_exit_status(),
                final_text: None,
                data: None,
            })
        }

        fn redact_error(
            &self,
            _phase: BackendHarnessErrorPhase,
            _err: &Self::BackendError,
        ) -> String {
            "unused".to_string()
        }
    }

    let (events_finish_tx, events_finish_rx) = oneshot::channel::<()>();
    let (backend_done_tx, backend_done_rx) = oneshot::channel::<()>();

    let adapter = std::sync::Arc::new(ControlAdapter {
        events_finish_rx: Mutex::new(Some(events_finish_rx)),
        backend_done_rx: Mutex::new(Some(backend_done_rx)),
    });

    let request = crate::AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };

    let crate::AgentWrapperRunControl { mut handle, cancel } =
        run_harnessed_backend_control(adapter, BackendDefaults::default(), request, None)
            .expect("control entrypoint succeeds");

    let first = handle.events.next().await.expect("first event forwarded");
    assert_eq!(first.kind, AgentWrapperEventKind::TextOutput);

    cancel.cancel();
    tokio::task::yield_now().await;

    assert!(
        tokio::time::timeout(Duration::from_secs(1), handle.events.next())
            .await
            .expect("events do not hang")
            .is_none(),
        "events stream must be closed after cancellation"
    );

    {
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(matches!(
            handle.completion.as_mut().poll(&mut cx),
            Poll::Pending
        ));
    }

    let _ = backend_done_tx.send(());
    let outcome = handle.completion.await;
    assert!(
        matches!(
            outcome,
            Err(AgentWrapperError::Backend { ref message }) if message == "cancelled"
        ),
        "completion must resolve to the pinned cancelled error after backend exit"
    );

    let _ = events_finish_tx.send(());
}
