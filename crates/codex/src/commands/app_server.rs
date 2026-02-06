use std::fs as std_fs;

use tokio::{process::Command, time};

use crate::{
    builder::{apply_cli_overrides, resolve_cli_overrides},
    process::{spawn_with_retry, tee_stream, ConsoleTarget},
    AppServerCodegenOutput, AppServerCodegenRequest, CodexClient, CodexError,
};

impl CodexClient {
    /// Generates app-server bindings via `codex app-server generate-ts` or `generate-json-schema`.
    ///
    /// Ensures the output directory exists, mirrors stdout/stderr according to the builder
    /// (`mirror_stdout` / `quiet`), and returns captured output plus the exit status. Non-zero
    /// exits bubble up as [`CodexError::NonZeroExit`] with stderr attached. Use
    /// [`AppServerCodegenRequest::prettier`] to format TypeScript output with a specific
    /// Prettier binary and request-level overrides for config/profile toggles.
    pub async fn generate_app_server_bindings(
        &self,
        request: AppServerCodegenRequest,
    ) -> Result<AppServerCodegenOutput, CodexError> {
        let AppServerCodegenRequest {
            target,
            out_dir,
            overrides,
        } = request;

        std_fs::create_dir_all(&out_dir).map_err(|source| CodexError::PrepareOutputDirectory {
            path: out_dir.clone(),
            source,
        })?;

        let dir_ctx = self.directory_context()?;
        let resolved_overrides =
            resolve_cli_overrides(&self.cli_overrides, &overrides, self.model.as_deref());

        let mut command = Command::new(self.command_env.binary_path());
        command
            .arg("app-server")
            .arg(target.subcommand())
            .arg("--out")
            .arg(&out_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .current_dir(dir_ctx.path());

        apply_cli_overrides(&mut command, &resolved_overrides, true);

        if let Some(prettier) = target.prettier() {
            command.arg("--prettier").arg(prettier);
        }

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

        if !status.success() {
            return Err(CodexError::NonZeroExit {
                status,
                stderr: String::from_utf8(stderr_bytes)?,
            });
        }

        Ok(AppServerCodegenOutput {
            status,
            stdout: String::from_utf8(stdout_bytes)?,
            stderr: String::from_utf8(stderr_bytes)?,
            out_dir,
        })
    }
}
