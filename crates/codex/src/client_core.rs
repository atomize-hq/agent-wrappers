use std::{
    env,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::{
    apply_diff::ApplyDiffArtifacts,
    builder::{apply_cli_overrides, resolve_cli_overrides, CliOverridesPatch},
    process::{spawn_with_retry, tee_stream, CommandOutput, ConsoleTarget},
    CodexClient, CodexError,
};
use tempfile::TempDir;
use tokio::{process::Command, time};

impl CodexClient {
    pub(crate) fn directory_context(&self) -> Result<DirectoryContext, CodexError> {
        if let Some(dir) = &self.working_dir {
            return Ok(DirectoryContext::Fixed(dir.clone()));
        }

        let temp = tempfile::tempdir().map_err(CodexError::TempDir)?;
        Ok(DirectoryContext::Ephemeral(temp))
    }

    pub(crate) fn sandbox_working_dir(
        &self,
        request_dir: Option<PathBuf>,
    ) -> Result<PathBuf, CodexError> {
        if let Some(dir) = request_dir {
            return Ok(dir);
        }

        if let Some(dir) = &self.working_dir {
            return Ok(dir.clone());
        }

        env::current_dir().map_err(|source| CodexError::WorkingDirectory { source })
    }

    pub(crate) async fn run_simple_command_with_overrides(
        &self,
        args: Vec<OsString>,
        overrides: CliOverridesPatch,
    ) -> Result<ApplyDiffArtifacts, CodexError> {
        let dir_ctx = self.directory_context()?;
        let resolved_overrides =
            resolve_cli_overrides(&self.cli_overrides, &overrides, self.model.as_deref());

        let mut command = Command::new(self.command_env.binary_path());
        command
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(dir_ctx.path());

        apply_cli_overrides(&mut command, &resolved_overrides, true);

        self.command_env.apply(&mut command)?;

        let mut child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

        let stdout = child.stdout.take().ok_or(CodexError::StdoutUnavailable)?;
        let stderr = child.stderr.take().ok_or(CodexError::StderrUnavailable)?;

        let stdout_task = tokio::spawn(tee_stream(
            stdout,
            ConsoleTarget::Stdout,
            self.mirror_stdout,
        ));
        let stderr_task = tokio::spawn(tee_stream(stderr, ConsoleTarget::Stderr, !self.quiet));

        let timeout = self.timeout;
        let wait_task = async move {
            let _dir_ctx = dir_ctx;
            let status = child
                .wait()
                .await
                .map_err(|source| CodexError::Wait { source })?;
            let stdout_bytes = stdout_task
                .await
                .map_err(CodexError::Join)?
                .map_err(CodexError::CaptureIo)?;
            let stderr_bytes = stderr_task
                .await
                .map_err(CodexError::Join)?
                .map_err(CodexError::CaptureIo)?;
            Ok::<_, CodexError>((status, stdout_bytes, stderr_bytes))
        };

        let (status, stdout_bytes, stderr_bytes) = if timeout.is_zero() {
            wait_task.await?
        } else {
            match time::timeout(timeout, wait_task).await {
                Ok(result) => result?,
                Err(_) => {
                    return Err(CodexError::Timeout { timeout });
                }
            }
        };

        if !status.success() {
            return Err(CodexError::NonZeroExit {
                status,
                stderr: String::from_utf8(stderr_bytes)?,
            });
        }

        Ok(ApplyDiffArtifacts {
            status,
            stdout: String::from_utf8(stdout_bytes)?,
            stderr: String::from_utf8(stderr_bytes)?,
        })
    }

    pub(crate) async fn run_basic_command<S, I>(&self, args: I) -> Result<CommandOutput, CodexError>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = S>,
    {
        let mut command = Command::new(self.command_env.binary_path());
        command
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        self.command_env.apply(&mut command)?;

        let mut child = spawn_with_retry(&mut command, self.command_env.binary_path())?;

        let stdout = child.stdout.take().ok_or(CodexError::StdoutUnavailable)?;
        let stderr = child.stderr.take().ok_or(CodexError::StderrUnavailable)?;

        let stdout_task = tokio::spawn(tee_stream(stdout, ConsoleTarget::Stdout, false));
        let stderr_task = tokio::spawn(tee_stream(stderr, ConsoleTarget::Stderr, false));

        let wait_task = async move {
            let status = child
                .wait()
                .await
                .map_err(|source| CodexError::Wait { source })?;
            let stdout_bytes = stdout_task
                .await
                .map_err(CodexError::Join)?
                .map_err(CodexError::CaptureIo)?;
            let stderr_bytes = stderr_task
                .await
                .map_err(CodexError::Join)?
                .map_err(CodexError::CaptureIo)?;
            Ok::<_, CodexError>((status, stdout_bytes, stderr_bytes))
        };

        let (status, stdout_bytes, stderr_bytes) = if self.timeout.is_zero() {
            wait_task.await?
        } else {
            match time::timeout(self.timeout, wait_task).await {
                Ok(result) => result?,
                Err(_) => {
                    return Err(CodexError::Timeout {
                        timeout: self.timeout,
                    });
                }
            }
        };

        Ok(CommandOutput {
            status,
            stdout: stdout_bytes,
            stderr: stderr_bytes,
        })
    }
}

pub(crate) enum DirectoryContext {
    Fixed(PathBuf),
    Ephemeral(TempDir),
}

impl DirectoryContext {
    pub(crate) fn path(&self) -> &Path {
        match self {
            DirectoryContext::Fixed(path) => path.as_path(),
            DirectoryContext::Ephemeral(dir) => dir.path(),
        }
    }
}
