use std::ffi::OsString;

use crate::{
    ApplyDiffArtifacts, CodexClient, CodexError, ExecReviewCommandRequest, ReviewCommandRequest,
};

impl CodexClient {
    /// Runs `codex review [OPTIONS] [PROMPT]` and returns captured output.
    pub async fn review(
        &self,
        request: ReviewCommandRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        if matches!(request.prompt.as_deref(), Some(prompt) if prompt.trim().is_empty()) {
            return Err(CodexError::EmptyPrompt);
        }

        let mut args = vec![OsString::from("review")];
        if let Some(base) = request.base {
            if !base.trim().is_empty() {
                args.push(OsString::from("--base"));
                args.push(OsString::from(base));
            }
        }
        if let Some(commit) = request.commit {
            if !commit.trim().is_empty() {
                args.push(OsString::from("--commit"));
                args.push(OsString::from(commit));
            }
        }
        if let Some(title) = request.title {
            if !title.trim().is_empty() {
                args.push(OsString::from("--title"));
                args.push(OsString::from(title));
            }
        }
        if request.uncommitted {
            args.push(OsString::from("--uncommitted"));
        }
        if let Some(prompt) = request.prompt {
            if !prompt.trim().is_empty() {
                args.push(OsString::from(prompt));
            }
        }

        self.run_simple_command_with_overrides(args, request.overrides)
            .await
    }

    /// Runs `codex exec review [OPTIONS] [PROMPT]` and returns captured output.
    pub async fn exec_review(
        &self,
        request: ExecReviewCommandRequest,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        if matches!(request.prompt.as_deref(), Some(prompt) if prompt.trim().is_empty()) {
            return Err(CodexError::EmptyPrompt);
        }

        let mut args = vec![OsString::from("exec"), OsString::from("review")];
        if let Some(base) = request.base {
            if !base.trim().is_empty() {
                args.push(OsString::from("--base"));
                args.push(OsString::from(base));
            }
        }
        if let Some(commit) = request.commit {
            if !commit.trim().is_empty() {
                args.push(OsString::from("--commit"));
                args.push(OsString::from(commit));
            }
        }
        if request.json {
            args.push(OsString::from("--json"));
        }
        if request.skip_git_repo_check {
            args.push(OsString::from("--skip-git-repo-check"));
        }
        if let Some(title) = request.title {
            if !title.trim().is_empty() {
                args.push(OsString::from("--title"));
                args.push(OsString::from(title));
            }
        }
        if request.uncommitted {
            args.push(OsString::from("--uncommitted"));
        }
        if let Some(prompt) = request.prompt {
            if !prompt.trim().is_empty() {
                args.push(OsString::from(prompt));
            }
        }

        self.run_simple_command_with_overrides(args, request.overrides)
            .await
    }
}
