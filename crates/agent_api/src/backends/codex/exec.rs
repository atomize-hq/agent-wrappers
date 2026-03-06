use std::{
    collections::BTreeMap,
    path::PathBuf,
    process::ExitStatus,
    sync::{Arc, Mutex},
    time::Duration,
};

use codex::{CodexError, ExecStreamError, ExecStreamRequest, ThreadEvent};
use futures_util::future::poll_fn;
use tokio::sync::oneshot;

use crate::backend_harness::BackendSpawn;

pub(super) struct ExecFlowRequest {
    pub(super) config: super::CodexBackendConfig,
    pub(super) run_start_cwd: Option<PathBuf>,
    pub(super) termination:
        Option<Arc<super::super::termination::TerminationState<super::CodexTerminationHandle>>>,
    pub(super) non_interactive: bool,
    pub(super) external_sandbox: bool,
    pub(super) approval_policy: Option<super::CodexApprovalPolicy>,
    pub(super) sandbox_mode: super::CodexSandboxMode,
    pub(super) resume: Option<super::SessionSelectorV1>,
    pub(super) prompt: String,
    pub(super) working_dir: Option<PathBuf>,
    pub(super) effective_timeout: Option<Duration>,
    pub(super) env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default)]
struct CodexStreamState {
    saw_thread_id: bool,
    saw_stream_error: bool,
    last_transport_error_code: Option<String>,
    last_transport_error_message: Option<String>,
}

#[derive(Debug)]
enum CodexTailEvent {
    NonZeroExit { status: ExitStatus },
    TerminalError { message: String },
}

fn is_unknown_bypass_flag_stderr(stderr: &str) -> bool {
    let stderr = stderr.to_ascii_lowercase();
    if !stderr.contains("dangerously-bypass-approvals-and-sandbox") {
        return false;
    }

    const UNKNOWN_SIGNALS: &[&str] = &[
        "unknown",
        "unrecognized",
        "unexpected",
        "unknown option",
        "unknown flag",
        "unrecognized option",
        "unrecognized flag",
        "unexpected argument",
        "found argument",
        "invalid option",
        "invalid flag",
    ];

    UNKNOWN_SIGNALS.iter().any(|signal| stderr.contains(signal))
}

fn map_approval_policy(policy: &super::CodexApprovalPolicy) -> codex::ApprovalPolicy {
    match policy {
        super::CodexApprovalPolicy::Untrusted => codex::ApprovalPolicy::Untrusted,
        super::CodexApprovalPolicy::OnFailure => codex::ApprovalPolicy::OnFailure,
        super::CodexApprovalPolicy::OnRequest => codex::ApprovalPolicy::OnRequest,
        super::CodexApprovalPolicy::Never => codex::ApprovalPolicy::Never,
    }
}

fn map_sandbox_mode(mode: &super::CodexSandboxMode) -> codex::SandboxMode {
    match mode {
        super::CodexSandboxMode::ReadOnly => codex::SandboxMode::ReadOnly,
        super::CodexSandboxMode::WorkspaceWrite => codex::SandboxMode::WorkspaceWrite,
        super::CodexSandboxMode::DangerFullAccess => codex::SandboxMode::DangerFullAccess,
    }
}

pub(super) async fn spawn_exec_or_resume_flow(
    req: ExecFlowRequest,
) -> Result<
    BackendSpawn<super::CodexBackendEvent, super::CodexBackendCompletion, super::CodexBackendError>,
    super::CodexBackendError,
> {
    let ExecFlowRequest {
        config,
        run_start_cwd,
        termination,
        non_interactive,
        external_sandbox,
        approval_policy,
        sandbox_mode,
        resume,
        prompt,
        working_dir,
        effective_timeout,
        env,
    } = req;

    let mut builder = codex::CodexClient::builder()
        .json(true)
        .mirror_stdout(false)
        .quiet(true)
        .color_mode(codex::ColorMode::Never)
        .sandbox_mode(map_sandbox_mode(&sandbox_mode));

    if external_sandbox {
        builder = builder.dangerously_bypass_approvals_and_sandbox(true);
    }

    if non_interactive {
        builder = builder.approval_policy(codex::ApprovalPolicy::Never);
    } else if let Some(value) = approval_policy.as_ref() {
        builder = builder.approval_policy(map_approval_policy(value));
    }

    if let Some(binary) = config.binary.as_ref() {
        builder = builder.binary(binary.clone());
    }

    if let Some(codex_home) = config.codex_home.as_ref() {
        builder = builder.codex_home(codex_home.clone());
    }

    let working_dir = working_dir
        .or_else(|| config.default_working_dir.clone())
        .or(run_start_cwd)
        .ok_or(super::CodexBackendError::WorkingDirectoryUnresolved)?;
    builder = builder.working_dir(working_dir);

    // Codex wrapper treats `Duration::ZERO` as “no timeout”.
    builder = builder.timeout(effective_timeout.unwrap_or(Duration::ZERO));

    let client = builder.build();

    let resume_selector = resume.clone();
    let suppress_transport_errors = resume_selector.is_some();
    let stream_state = Arc::new(Mutex::new(CodexStreamState::default()));

    let handle = match resume {
        None => {
            client
                .stream_exec_with_env_overrides_control(
                    ExecStreamRequest {
                        prompt,
                        idle_timeout: None,
                        output_last_message: None,
                        output_schema: None,
                        json_event_log: None,
                    },
                    &env,
                )
                .await
        }
        Some(super::SessionSelectorV1::Last) => {
            client
                .stream_resume_with_env_overrides_control(
                    codex::ResumeRequest::last().prompt(prompt),
                    &env,
                )
                .await
        }
        Some(super::SessionSelectorV1::Id { id }) => {
            client
                .stream_resume_with_env_overrides_control(
                    codex::ResumeRequest::with_id(id).prompt(prompt),
                    &env,
                )
                .await
        }
    }
    .map_err(super::CodexBackendError::Exec)?;

    let codex::ExecStreamControl {
        events,
        completion,
        termination: termination_handle,
    } = handle;

    if let Some(state) = termination.as_ref() {
        state.set_handle(super::CodexTerminationHandle::Exec(termination_handle));
    }

    let (completion_tx, completion_rx) =
        oneshot::channel::<Result<super::CodexBackendCompletion, super::CodexBackendError>>();
    let (tail_tx, tail_rx) = oneshot::channel::<Option<CodexTailEvent>>();
    let (events_done_tx, events_done_rx) = oneshot::channel::<()>();
    let stream_state_for_completion = Arc::clone(&stream_state);

    tokio::spawn(async move {
        let outcome = completion.await;
        if resume_selector.is_some() {
            let _ = events_done_rx.await;
        }

        match outcome {
            Ok(exec_completion) => {
                let status = exec_completion.status;
                let completion = super::CodexBackendCompletion {
                    status,
                    final_text: exec_completion.last_message,
                    selection_failure_message: None,
                };
                let _ = completion_tx.send(Ok(completion));
                let _ = tail_tx.send(None);
            }
            Err(ExecStreamError::Codex(CodexError::NonZeroExit { status, stderr })) => {
                let bypass_flag_unsupported_message =
                    if external_sandbox && is_unknown_bypass_flag_stderr(&stderr) {
                        Some(super::PINNED_EXTERNAL_SANDBOX_FLAG_UNSUPPORTED.to_string())
                    } else {
                        None
                    };

                let selection_failure_message = bypass_flag_unsupported_message.or_else(|| {
                    resume_selector.as_ref().and_then(|selector| {
                        let snapshot = stream_state_for_completion.lock().ok()?;
                        if snapshot.saw_thread_id || snapshot.saw_stream_error {
                            return None;
                        }

                        let stderr_not_found = super::is_not_found_signal(&stderr);
                        let transport_message_not_found = snapshot
                            .last_transport_error_message
                            .as_deref()
                            .is_some_and(super::is_not_found_signal);
                        let transport_code_not_found = snapshot
                            .last_transport_error_code
                            .as_deref()
                            .is_some_and(super::is_not_found_signal);

                        if stderr_not_found
                            || transport_message_not_found
                            || transport_code_not_found
                        {
                            Some(super::pinned_selection_failure_message(selector).to_string())
                        } else {
                            None
                        }
                    })
                });

                let tail = if let Some(message) = selection_failure_message.clone() {
                    CodexTailEvent::TerminalError { message }
                } else {
                    CodexTailEvent::NonZeroExit { status }
                };

                let completion = super::CodexBackendCompletion {
                    status,
                    final_text: None,
                    selection_failure_message,
                };
                let _ = completion_tx.send(Ok(completion));
                let _ = tail_tx.send(Some(tail));
            }
            Err(err) => {
                let _ = completion_tx.send(Err(super::CodexBackendError::Exec(err)));
                let _ = tail_tx.send(None);
            }
        }
    });

    let events = Box::pin(futures_util::stream::unfold(
        (
            events,
            stream_state.clone(),
            Some(events_done_tx),
            Some(tail_rx),
            suppress_transport_errors,
            false,
        ),
        |(
            mut events,
            stream_state,
            mut events_done_tx,
            mut tail_rx,
            suppress_transport_errors,
            tail_emitted,
        )| async move {
            loop {
                let item = poll_fn(|cx| events.as_mut().poll_next(cx)).await;
                match item {
                    Some(Ok(thread_ev)) => {
                        if let Ok(mut snapshot) = stream_state.lock() {
                            if thread_ev.thread_id().is_some() {
                                snapshot.saw_thread_id = true;
                            }

                            if suppress_transport_errors
                                && matches!(thread_ev, ThreadEvent::Error(_))
                            {
                                if let ThreadEvent::Error(err) = &thread_ev {
                                    snapshot.last_transport_error_code = err.code.clone();
                                    snapshot.last_transport_error_message =
                                        Some(err.message.clone());
                                }
                            }
                        }

                        if suppress_transport_errors && matches!(thread_ev, ThreadEvent::Error(_)) {
                            continue;
                        }

                        return Some((
                            Ok(super::CodexBackendEvent::Thread(Box::new(thread_ev))),
                            (
                                events,
                                stream_state,
                                events_done_tx,
                                tail_rx,
                                suppress_transport_errors,
                                tail_emitted,
                            ),
                        ));
                    }
                    Some(Err(err)) => {
                        if let Ok(mut snapshot) = stream_state.lock() {
                            snapshot.saw_stream_error = true;
                        }

                        return Some((
                            Err(super::CodexBackendError::Exec(err)),
                            (
                                events,
                                stream_state,
                                events_done_tx,
                                tail_rx,
                                suppress_transport_errors,
                                tail_emitted,
                            ),
                        ));
                    }
                    None => {
                        if let Some(tx) = events_done_tx.take() {
                            let _ = tx.send(());
                        }

                        if tail_emitted {
                            return None;
                        }

                        let tail = match tail_rx.take() {
                            Some(rx) => rx.await.ok().flatten(),
                            None => None,
                        }?;

                        let event = match tail {
                            CodexTailEvent::NonZeroExit { status } => {
                                super::CodexBackendEvent::NonZeroExit { status }
                            }
                            CodexTailEvent::TerminalError { message } => {
                                super::CodexBackendEvent::TerminalError { message }
                            }
                        };

                        return Some((
                            Ok(event),
                            (
                                events,
                                stream_state,
                                events_done_tx,
                                tail_rx,
                                suppress_transport_errors,
                                true,
                            ),
                        ));
                    }
                }
            }
        },
    ));

    let completion = Box::pin(async move {
        completion_rx
            .await
            .unwrap_or(Err(super::CodexBackendError::CompletionTaskDropped))
    });

    Ok(BackendSpawn { events, completion })
}
