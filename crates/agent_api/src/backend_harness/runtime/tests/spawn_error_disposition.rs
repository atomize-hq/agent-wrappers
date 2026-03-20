use super::*;

#[tokio::test]
async fn direct_spawn_failures_return_backend_error_without_handle() {
    let adapter = std::sync::Arc::new(ToyAdapter {
        fail_spawn: true,
        spawn_error_disposition: contract::SpawnErrorDisposition::ReturnDirectly,
    });
    let request = crate::AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };

    let err = run_harnessed_backend(adapter, BackendDefaults::default(), request)
        .await
        .expect_err("classified spawn failure should fail directly");
    assert!(matches!(
        err,
        crate::AgentWrapperError::Backend { ref message }
        if message == "toy backend error (redacted): phase=spawn"
    ));
}

#[tokio::test]
async fn direct_spawn_failures_surface_via_control_handle_after_return() {
    let adapter = std::sync::Arc::new(ToyAdapter {
        fail_spawn: true,
        spawn_error_disposition: contract::SpawnErrorDisposition::ReturnDirectly,
    });
    let request = crate::AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };

    let crate::AgentWrapperRunControl {
        mut handle,
        cancel: _,
    } = run_harnessed_backend_control(adapter, BackendDefaults::default(), request, None)
        .await
        .expect("control entrypoint should return before startup resolves");

    let event = handle
        .events
        .next()
        .await
        .expect("spawn failure should surface as an error event");
    assert_eq!(event.kind, AgentWrapperEventKind::Error);
    assert_eq!(
        event.message.as_deref(),
        Some("toy backend error (redacted): phase=spawn")
    );

    assert!(
        handle.events.next().await.is_none(),
        "error event should be terminal for spawn failure"
    );

    let err = handle
        .completion
        .await
        .expect_err("completion should surface the spawn failure");
    assert!(matches!(
        err,
        crate::AgentWrapperError::Backend { ref message }
            if message == "toy backend error (redacted): phase=spawn"
    ));
}

#[tokio::test]
async fn unclassified_spawn_failures_still_surface_via_handle() {
    let adapter = std::sync::Arc::new(ToyAdapter {
        fail_spawn: true,
        spawn_error_disposition: contract::SpawnErrorDisposition::SurfaceViaHandle,
    });
    let request = crate::AgentWrapperRunRequest {
        prompt: "hello".to_string(),
        ..Default::default()
    };

    let handle = run_harnessed_backend(adapter, BackendDefaults::default(), request)
        .await
        .expect("unclassified spawn failure should still yield a handle");
    let mut events = handle.events;
    let mut seen = Vec::new();
    while let Some(event) = events.next().await {
        seen.push(event);
    }

    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].kind, AgentWrapperEventKind::Error);
    assert_eq!(
        seen[0].message.as_deref(),
        Some("toy backend error (redacted): phase=spawn")
    );

    let err = handle
        .completion
        .await
        .expect_err("completion should be backend error");
    assert!(matches!(
        err,
        crate::AgentWrapperError::Backend { ref message }
        if message == "toy backend error (redacted): phase=spawn"
    ));
}
