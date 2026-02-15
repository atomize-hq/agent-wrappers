use std::{sync::Arc, time::Duration};

#[cfg(unix)]
use portable_pty::{native_pty_system, CommandBuilder, PtySize};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::{oneshot, Mutex},
    task, time,
};

use super::ClaudeClient;
use crate::{
    commands::setup_token::ClaudeSetupTokenRequest,
    process::{self, ConsoleTarget},
    ClaudeCodeError, CommandOutput,
};

pub struct ClaudeSetupTokenSession {
    url: String,
    url_rx: Option<oneshot::Receiver<String>>,
    process: Option<SetupTokenProcess>,
    stdout_buf: Arc<Mutex<Vec<u8>>>,
    stderr_buf: Arc<Mutex<Vec<u8>>>,
    stdout_task: Option<tokio::task::JoinHandle<Result<(), ClaudeCodeError>>>,
    stderr_task: Option<tokio::task::JoinHandle<Result<(), ClaudeCodeError>>>,
    timeout: Option<Duration>,
}

enum SetupTokenProcess {
    Pipes {
        child: tokio::process::Child,
        stdin: Option<tokio::process::ChildStdin>,
    },
    #[cfg(unix)]
    Pty {
        child: Box<dyn portable_pty::Child + Send + Sync>,
        writer: Option<Box<dyn std::io::Write + Send>>,
    },
}

impl std::fmt::Debug for ClaudeSetupTokenSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeSetupTokenSession")
            .field("url", &self.url)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

impl ClaudeSetupTokenSession {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub async fn wait_for_url(
        &mut self,
        timeout: Duration,
    ) -> Result<Option<&str>, ClaudeCodeError> {
        if !self.url.is_empty() {
            return Ok(Some(self.url.as_str()));
        }

        let deadline = time::Instant::now() + timeout;
        let poll_interval = Duration::from_millis(25);

        loop {
            if !self.url.is_empty() {
                return Ok(Some(self.url.as_str()));
            }

            // Parse from the full captured output buffer (more robust than relying only on
            // incremental chunk parsing, and avoids “TTY noise” issues).
            if let Some(url) = self.extract_url_from_captured_output().await {
                self.url = url;
                self.url_rx = None;
                return Ok(Some(self.url.as_str()));
            }

            if time::Instant::now() >= deadline {
                self.url_rx = None;
                return Ok(None);
            }

            let Some(rx) = self.url_rx.as_mut() else {
                time::sleep(poll_interval).await;
                continue;
            };

            let remaining = deadline.saturating_duration_since(time::Instant::now());
            let nap = std::cmp::min(poll_interval, remaining);

            match time::timeout(nap, rx).await {
                Ok(Ok(url)) => {
                    self.url = url;
                    self.url_rx = None;
                    return Ok(Some(self.url.as_str()));
                }
                Ok(Err(_closed)) => {
                    self.url_rx = None;
                    // Keep looping: URL might still be extractable from buffered output.
                }
                Err(_timeout) => {
                    // Keep looping; we'll re-check buffer.
                }
            }
        }
    }

    async fn extract_url_from_captured_output(&self) -> Option<String> {
        let mut combined = Vec::new();
        combined.extend_from_slice(&self.stdout_buf.lock().await);
        combined.extend_from_slice(&self.stderr_buf.lock().await);
        let text = String::from_utf8_lossy(&combined);
        extract_oauth_url(&text)
    }

    pub async fn submit_code(mut self, code: &str) -> Result<CommandOutput, ClaudeCodeError> {
        let process = self
            .process
            .as_mut()
            .expect("setup-token session process present");

        match process {
            SetupTokenProcess::Pipes { stdin, .. } => {
                if let Some(mut stdin) = stdin.take() {
                    let mut bytes = code.as_bytes().to_vec();
                    if !bytes.ends_with(b"\n") {
                        bytes.push(b'\n');
                    }
                    stdin
                        .write_all(&bytes)
                        .await
                        .map_err(ClaudeCodeError::StdinWrite)?;
                }
            }
            #[cfg(unix)]
            SetupTokenProcess::Pty { writer, .. } => {
                if let Some(mut writer) = writer.take() {
                    let mut bytes = code.as_bytes().to_vec();
                    if !bytes.ends_with(b"\n") {
                        bytes.push(b'\n');
                    }

                    tokio::task::spawn_blocking(move || {
                        writer.write_all(&bytes)?;
                        writer.flush()
                    })
                    .await
                    .map_err(|e| ClaudeCodeError::Join(e.to_string()))?
                    .map_err(ClaudeCodeError::StdinWrite)?;
                }
            }
        };
        self.wait().await
    }

    pub async fn wait(mut self) -> Result<CommandOutput, ClaudeCodeError> {
        let process = self
            .process
            .take()
            .expect("setup-token session process present");

        let timeout = self.timeout;
        let status = match process {
            SetupTokenProcess::Pipes { mut child, .. } => {
                let wait_fut = child.wait();
                if let Some(dur) = timeout {
                    time::timeout(dur, wait_fut)
                        .await
                        .map_err(|_| ClaudeCodeError::Timeout { timeout: dur })?
                        .map_err(ClaudeCodeError::Wait)?
                } else {
                    wait_fut.await.map_err(ClaudeCodeError::Wait)?
                }
            }
            #[cfg(unix)]
            SetupTokenProcess::Pty { mut child, .. } => {
                let poll_interval = Duration::from_millis(50);
                let status = if let Some(dur) = timeout {
                    let deadline = time::Instant::now() + dur;
                    loop {
                        match child.try_wait().map_err(ClaudeCodeError::Wait)? {
                            Some(status) => break status,
                            None => {
                                if time::Instant::now() >= deadline {
                                    let _ = child.kill();
                                    return Err(ClaudeCodeError::Timeout { timeout: dur });
                                }
                                time::sleep(poll_interval).await;
                            }
                        }
                    }
                } else {
                    loop {
                        match child.try_wait().map_err(ClaudeCodeError::Wait)? {
                            Some(status) => break status,
                            None => time::sleep(poll_interval).await,
                        }
                    }
                };

                portable_exit_status_to_std(status)
            }
        };

        if let Some(task) = self.stdout_task.take() {
            task.await
                .map_err(|e| ClaudeCodeError::Join(e.to_string()))??;
        }
        if let Some(task) = self.stderr_task.take() {
            task.await
                .map_err(|e| ClaudeCodeError::Join(e.to_string()))??;
        }

        let stdout = self.stdout_buf.lock().await.clone();
        let stderr = self.stderr_buf.lock().await.clone();

        Ok(CommandOutput {
            status,
            stdout,
            stderr,
        })
    }
}

impl Drop for ClaudeSetupTokenSession {
    fn drop(&mut self) {
        // Best-effort cleanup; if the session is dropped before completion, avoid leaving
        // an interactive `claude setup-token` process running.
        let Some(process) = self.process.as_mut() else {
            return;
        };

        match process {
            SetupTokenProcess::Pipes { child, .. } => {
                if child.id().is_some() {
                    let _ = child.start_kill();
                }
            }
            #[cfg(unix)]
            SetupTokenProcess::Pty { child, .. } => {
                let _ = child.kill();
            }
        }
    }
}

impl ClaudeClient {
    pub async fn setup_token_start(&self) -> Result<ClaudeSetupTokenSession, ClaudeCodeError> {
        self.setup_token_start_with(ClaudeSetupTokenRequest::new())
            .await
    }

    pub async fn setup_token_start_with(
        &self,
        request: ClaudeSetupTokenRequest,
    ) -> Result<ClaudeSetupTokenSession, ClaudeCodeError> {
        let requested_timeout = request.timeout;
        let binary = self.resolve_binary();
        let argv = request.into_command().argv();

        let stdout_buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let stderr_buf = Arc::new(Mutex::new(Vec::<u8>::new()));

        let (url_tx, url_rx) = oneshot::channel::<String>();
        let url_tx = Arc::new(Mutex::new(Some(url_tx)));
        let url_state = Arc::new(Mutex::new(UrlCapture::default()));

        #[cfg(unix)]
        if let Ok((process, stdout_task)) = spawn_setup_token_pty(
            &binary,
            &argv,
            self.working_dir.as_deref(),
            &self.env,
            self.mirror_stdout,
            self.mirror_stderr,
            stdout_buf.clone(),
            url_state.clone(),
            url_tx.clone(),
        ) {
            return Ok(ClaudeSetupTokenSession {
                url: String::new(),
                url_rx: Some(url_rx),
                process: Some(process),
                stdout_buf,
                stderr_buf,
                stdout_task: Some(stdout_task),
                stderr_task: None,
                timeout: requested_timeout,
            });
        }

        let mut cmd = Command::new(&binary);
        cmd.args(&argv);

        if let Some(dir) = self.working_dir.as_ref() {
            cmd.current_dir(dir);
        }

        process::apply_env(&mut cmd, &self.env);

        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = process::spawn_with_retry(&mut cmd, &binary)?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().ok_or(ClaudeCodeError::MissingStdout)?;
        let stderr = child.stderr.take().ok_or(ClaudeCodeError::MissingStderr)?;

        let stdout_task = spawn_capture_task(
            stdout,
            ConsoleTarget::Stdout,
            self.mirror_stdout,
            stdout_buf.clone(),
            url_state.clone(),
            url_tx.clone(),
        );
        let stderr_task = spawn_capture_task(
            stderr,
            ConsoleTarget::Stderr,
            self.mirror_stderr,
            stderr_buf.clone(),
            url_state.clone(),
            url_tx.clone(),
        );

        Ok(ClaudeSetupTokenSession {
            url: String::new(),
            url_rx: Some(url_rx),
            process: Some(SetupTokenProcess::Pipes { child, stdin }),
            stdout_buf,
            stderr_buf,
            stdout_task: Some(stdout_task),
            stderr_task: Some(stderr_task),
            // `setup-token` is inherently interactive and can require human/browser steps.
            // Do not apply the client's default timeout unless the caller explicitly requests one.
            timeout: requested_timeout,
        })
    }
}

#[derive(Debug, Default)]
struct UrlCapture {
    buffer: String,
    found: Option<String>,
}

impl UrlCapture {
    fn push_text(&mut self, chunk: &str) -> Option<String> {
        if self.found.is_some() {
            return None;
        }

        if self.buffer.len() > 64 * 1024 {
            // Avoid unbounded growth in the unlikely event the command is chatty before printing
            // the URL.
            self.buffer = self
                .buffer
                .chars()
                .rev()
                .take(16 * 1024)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();
        }

        self.buffer.push_str(chunk);
        if let Some(url) = extract_oauth_url(&self.buffer) {
            self.found = Some(url.clone());
            return Some(url);
        }
        None
    }
}

fn extract_oauth_url(text: &str) -> Option<String> {
    let cleaned = strip_ansi(text);
    let start = cleaned.find("https://claude.ai/oauth/authorize?")?;
    let tail = &cleaned[start..];

    let mut stop = tail.len();
    if let Some(idx) = tail.find("\n\n") {
        stop = stop.min(idx);
    }
    if let Some(idx) = tail.find("Paste code") {
        stop = stop.min(idx);
    }
    if let Some(idx) = tail.find("Paste") {
        stop = stop.min(idx);
    }

    let raw = &tail[..stop];
    let flattened: String = raw.split_whitespace().collect();
    flattened
        .starts_with("https://claude.ai/oauth/authorize?")
        .then_some(flattened)
}

fn strip_ansi(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != 0x1b {
            out.push(bytes[i]);
            i += 1;
            continue;
        }

        // ESC sequence.
        i += 1;
        if i >= bytes.len() {
            break;
        }

        match bytes[i] {
            b'[' => {
                // CSI: ESC [ ... <final>
                i += 1;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if (0x40..=0x7e).contains(&b) {
                        break;
                    }
                }
            }
            b']' => {
                // OSC: ESC ] ... BEL or ESC \
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == 0x07 {
                        i += 1;
                        break;
                    }
                    if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            b'P' | b'^' | b'_' => {
                // DCS / PM / APC: ESC <X> ... ESC \
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            _ => {
                // Other 2-byte escape; drop it.
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(unix)]
fn portable_exit_status_to_std(status: portable_pty::ExitStatus) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;

    let code = std::cmp::min(status.exit_code(), 255) as i32;
    // POSIX encodes the exit code in the high byte.
    std::process::ExitStatus::from_raw(code << 8)
}

#[cfg(unix)]
fn spawn_setup_token_pty(
    binary: &std::path::Path,
    argv: &[String],
    working_dir: Option<&std::path::Path>,
    env: &std::collections::BTreeMap<String, String>,
    mirror_stdout: bool,
    mirror_stderr: bool,
    out: Arc<Mutex<Vec<u8>>>,
    url_state: Arc<Mutex<UrlCapture>>,
    url_tx: Arc<Mutex<Option<oneshot::Sender<String>>>>,
) -> Result<
    (
        SetupTokenProcess,
        tokio::task::JoinHandle<Result<(), ClaudeCodeError>>,
    ),
    ClaudeCodeError,
> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| ClaudeCodeError::InvalidRequest(format!("failed to open PTY: {e}")))?;

    // Note: we don't attempt to alter termios/echo settings here. The underlying CLI
    // may toggle raw mode itself; this wrapper only needs a PTY to avoid Ink errors.

    let mut cmd = CommandBuilder::new(binary.to_string_lossy().to_string());
    for arg in argv {
        cmd.arg(arg);
    }
    if let Some(dir) = working_dir {
        cmd.cwd(dir);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }

    let child = pair.slave.spawn_command(cmd).map_err(|e| {
        ClaudeCodeError::InvalidRequest(format!("failed to spawn PTY command: {e}"))
    })?;
    drop(pair.slave);

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| ClaudeCodeError::InvalidRequest(format!("failed to clone PTY reader: {e}")))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| ClaudeCodeError::InvalidRequest(format!("failed to take PTY writer: {e}")))?;

    let mirror_console = mirror_stdout || mirror_stderr;
    let mirror_target = if mirror_stdout {
        ConsoleTarget::Stdout
    } else {
        ConsoleTarget::Stderr
    };

    let stdout_task = spawn_pty_capture_task(
        reader,
        mirror_target,
        mirror_console,
        out,
        url_state,
        url_tx,
    );

    Ok((
        SetupTokenProcess::Pty {
            child,
            writer: Some(writer),
        },
        stdout_task,
    ))
}

fn spawn_capture_task<R>(
    reader: R,
    target: ConsoleTarget,
    mirror_console: bool,
    out: Arc<Mutex<Vec<u8>>>,
    url_state: Arc<Mutex<UrlCapture>>,
    url_tx: Arc<Mutex<Option<oneshot::Sender<String>>>>,
) -> tokio::task::JoinHandle<Result<(), ClaudeCodeError>>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut reader = reader;
        let mut chunk = [0u8; 4096];
        loop {
            let n = reader.read(&mut chunk).await.map_err(|e| match target {
                ConsoleTarget::Stdout => ClaudeCodeError::StdoutRead(e),
                ConsoleTarget::Stderr => ClaudeCodeError::StderrRead(e),
            })?;
            if n == 0 {
                break;
            }

            if mirror_console {
                task::block_in_place(|| {
                    let mut w: Box<dyn std::io::Write> = match target {
                        ConsoleTarget::Stdout => Box::new(std::io::stdout()),
                        ConsoleTarget::Stderr => Box::new(std::io::stderr()),
                    };
                    w.write_all(&chunk[..n])?;
                    w.flush()
                })
                .map_err(|e| match target {
                    ConsoleTarget::Stdout => ClaudeCodeError::StdoutRead(e),
                    ConsoleTarget::Stderr => ClaudeCodeError::StderrRead(e),
                })?;
            }

            out.lock().await.extend_from_slice(&chunk[..n]);

            let text = String::from_utf8_lossy(&chunk[..n]);
            if let Some(url) = url_state.lock().await.push_text(&text) {
                if let Some(tx) = url_tx.lock().await.take() {
                    let _ = tx.send(url);
                }
            }
        }
        Ok(())
    })
}

#[cfg(unix)]
fn spawn_pty_capture_task(
    mut reader: Box<dyn std::io::Read + Send>,
    target: ConsoleTarget,
    mirror_console: bool,
    out: Arc<Mutex<Vec<u8>>>,
    url_state: Arc<Mutex<UrlCapture>>,
    url_tx: Arc<Mutex<Option<oneshot::Sender<String>>>>,
) -> tokio::task::JoinHandle<Result<(), ClaudeCodeError>> {
    tokio::spawn(async move {
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let read_task = tokio::task::spawn_blocking(move || -> Result<(), std::io::Error> {
            let mut chunk = [0u8; 4096];
            loop {
                let n = reader.read(&mut chunk)?;
                if n == 0 {
                    break;
                }
                if tx.send(chunk[..n].to_vec()).is_err() {
                    break;
                }
            }
            Ok(())
        });

        while let Some(bytes) = rx.recv().await {
            if mirror_console {
                task::block_in_place(|| {
                    let mut w: Box<dyn std::io::Write> = match target {
                        ConsoleTarget::Stdout => Box::new(std::io::stdout()),
                        ConsoleTarget::Stderr => Box::new(std::io::stderr()),
                    };
                    w.write_all(&bytes)?;
                    w.flush()
                })
                .map_err(|e| match target {
                    ConsoleTarget::Stdout => ClaudeCodeError::StdoutRead(e),
                    ConsoleTarget::Stderr => ClaudeCodeError::StderrRead(e),
                })?;
            }

            out.lock().await.extend_from_slice(&bytes);

            let text = String::from_utf8_lossy(&bytes);
            if let Some(url) = url_state.lock().await.push_text(&text) {
                if let Some(tx) = url_tx.lock().await.take() {
                    let _ = tx.send(url);
                }
            }
        }

        read_task
            .await
            .map_err(|e| ClaudeCodeError::Join(e.to_string()))?
            .map_err(|e| match target {
                ConsoleTarget::Stdout => ClaudeCodeError::StdoutRead(e),
                ConsoleTarget::Stderr => ClaudeCodeError::StderrRead(e),
            })?;

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::{extract_oauth_url, strip_ansi};

    #[test]
    fn extracts_wrapped_url_from_setup_token_output() {
        let text = r#"
Browser didn't open? Use the url below to sign in (c to copy)

https://claude.ai/oauth/authorize?code=true&client_id=abc&response_type=c
ode&redirect_uri=https%3A%2F%2Fplatform.claude.com%2Foauth%2Fcode%2Fcallback&scope=user%3Ainference

Paste code here if prompted >
"#;

        let url = extract_oauth_url(text).expect("url");
        assert!(url.starts_with("https://claude.ai/oauth/authorize?"));
        assert!(url.contains("client_id=abc"));
        assert!(url.contains("response_type=code"));
    }

    #[test]
    fn strip_ansi_removes_common_sequences() {
        assert_eq!(strip_ansi("a\x1b[2Jb"), "ab");
        assert_eq!(strip_ansi("a\x1b]0;title\x07b"), "ab");
        assert_eq!(strip_ansi("a\x1b]8;;https://x\x1b\\b"), "ab");
    }
}
