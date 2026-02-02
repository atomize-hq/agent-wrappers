use std::io::BufRead;
use std::path::Path;

use crate::{CodexError, ExecStreamError, ThreadEvent};

/// Parses Codex `--json` JSONL logs into typed [`ThreadEvent`] values.
///
/// This API is synchronous and line-oriented (v1 contract).
#[derive(Clone, Debug, Default)]
pub struct JsonlThreadEventParser {
    context: crate::StreamContext,
}

impl JsonlThreadEventParser {
    /// Constructs a new parser with no established context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears any thread/turn context and resets synthetic turn counters.
    pub fn reset(&mut self) {
        self.context = crate::StreamContext::default();
    }

    /// Parses a single logical JSONL line.
    ///
    /// - Returns `Ok(None)` for empty / whitespace-only lines.
    /// - Otherwise returns `Ok(Some(ThreadEvent))` on success.
    /// - Returns `Err(ExecStreamError)` on JSON parse / normalization / typed parse failures.
    pub fn parse_line(&mut self, line: &str) -> Result<Option<ThreadEvent>, ExecStreamError> {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.chars().all(|ch| ch.is_whitespace()) {
            return Ok(None);
        }

        crate::normalize_thread_event(line, &mut self.context).map(Some)
    }
}

#[derive(Debug)]
pub struct ThreadEventJsonlRecord {
    /// 1-based line number in the underlying source (file/reader).
    pub line_number: usize,
    /// The parse outcome for this line (success or failure).
    pub outcome: Result<ThreadEvent, ExecStreamError>,
}

impl Clone for ThreadEventJsonlRecord {
    fn clone(&self) -> Self {
        Self {
            line_number: self.line_number,
            outcome: match &self.outcome {
                Ok(event) => Ok(event.clone()),
                Err(err) => Err(clone_exec_stream_error(err)),
            },
        }
    }
}

fn clone_exec_stream_error(err: &ExecStreamError) -> ExecStreamError {
    match err {
        ExecStreamError::Codex(source) => ExecStreamError::Codex(clone_codex_error(source)),
        ExecStreamError::Parse { line, source } => ExecStreamError::Parse {
            line: line.clone(),
            source: <serde_json::Error as serde::de::Error>::custom(source.to_string()),
        },
        ExecStreamError::Normalize { line, message } => ExecStreamError::Normalize {
            line: line.clone(),
            message: message.clone(),
        },
        ExecStreamError::IdleTimeout { idle_for } => ExecStreamError::IdleTimeout {
            idle_for: *idle_for,
        },
        ExecStreamError::ChannelClosed => ExecStreamError::ChannelClosed,
    }
}

fn clone_codex_error(err: &CodexError) -> CodexError {
    match err {
        CodexError::Spawn { binary, source } => CodexError::Spawn {
            binary: binary.clone(),
            source: clone_io_error(source),
        },
        CodexError::Wait { source } => CodexError::Wait {
            source: clone_io_error(source),
        },
        CodexError::Timeout { timeout } => CodexError::Timeout { timeout: *timeout },
        CodexError::NonZeroExit { status, stderr } => CodexError::NonZeroExit {
            status: *status,
            stderr: stderr.clone(),
        },
        CodexError::InvalidUtf8(source) => {
            let io_err = std::io::Error::new(std::io::ErrorKind::InvalidData, source.to_string());
            CodexError::CaptureIo(io_err)
        }
        CodexError::JsonParse {
            context,
            stdout,
            source,
        } => CodexError::JsonParse {
            context,
            stdout: stdout.clone(),
            source: <serde_json::Error as serde::de::Error>::custom(source.to_string()),
        },
        CodexError::ExecPolicyParse { stdout, source } => CodexError::ExecPolicyParse {
            stdout: stdout.clone(),
            source: <serde_json::Error as serde::de::Error>::custom(source.to_string()),
        },
        CodexError::FeatureListParse { reason, stdout } => CodexError::FeatureListParse {
            reason: reason.clone(),
            stdout: stdout.clone(),
        },
        CodexError::ResponsesApiProxyInfoRead { path, source } => {
            CodexError::ResponsesApiProxyInfoRead {
                path: path.clone(),
                source: clone_io_error(source),
            }
        }
        CodexError::ResponsesApiProxyInfoParse { path, source } => {
            CodexError::ResponsesApiProxyInfoParse {
                path: path.clone(),
                source: <serde_json::Error as serde::de::Error>::custom(source.to_string()),
            }
        }
        CodexError::EmptyPrompt => CodexError::EmptyPrompt,
        CodexError::EmptySandboxCommand => CodexError::EmptySandboxCommand,
        CodexError::EmptyExecPolicyCommand => CodexError::EmptyExecPolicyCommand,
        CodexError::EmptyApiKey => CodexError::EmptyApiKey,
        CodexError::EmptyTaskId => CodexError::EmptyTaskId,
        CodexError::EmptyEnvId => CodexError::EmptyEnvId,
        CodexError::EmptyMcpServerName => CodexError::EmptyMcpServerName,
        CodexError::EmptyMcpCommand => CodexError::EmptyMcpCommand,
        CodexError::EmptyMcpUrl => CodexError::EmptyMcpUrl,
        CodexError::EmptySocketPath => CodexError::EmptySocketPath,
        CodexError::TempDir(source) => CodexError::TempDir(clone_io_error(source)),
        CodexError::WorkingDirectory { source } => CodexError::WorkingDirectory {
            source: clone_io_error(source),
        },
        CodexError::PrepareOutputDirectory { path, source } => CodexError::PrepareOutputDirectory {
            path: path.clone(),
            source: clone_io_error(source),
        },
        CodexError::PrepareCodexHome { path, source } => CodexError::PrepareCodexHome {
            path: path.clone(),
            source: clone_io_error(source),
        },
        CodexError::StdoutUnavailable => CodexError::StdoutUnavailable,
        CodexError::StderrUnavailable => CodexError::StderrUnavailable,
        CodexError::StdinUnavailable => CodexError::StdinUnavailable,
        CodexError::CaptureIo(source) => CodexError::CaptureIo(clone_io_error(source)),
        CodexError::StdinWrite(source) => CodexError::StdinWrite(clone_io_error(source)),
        CodexError::Join(source) => {
            let io_err = std::io::Error::other(source.to_string());
            CodexError::CaptureIo(io_err)
        }
    }
}

fn clone_io_error(err: &std::io::Error) -> std::io::Error {
    std::io::Error::new(err.kind(), err.to_string())
}

pub struct ThreadEventJsonlReader<R: BufRead> {
    reader: R,
    parser: JsonlThreadEventParser,
    line_number: usize,
    buffer: String,
    done: bool,
}

impl<R: BufRead> ThreadEventJsonlReader<R> {
    /// Creates a reader-backed iterator with a fresh parser.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            parser: JsonlThreadEventParser::new(),
            line_number: 0,
            buffer: String::new(),
            done: false,
        }
    }

    /// Consumes the iterator and returns the wrapped reader.
    pub fn into_inner(self) -> R {
        self.reader
    }
}

impl<R: BufRead> Iterator for ThreadEventJsonlReader<R> {
    type Item = ThreadEventJsonlRecord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        loop {
            self.buffer.clear();
            let line_number = self.line_number.saturating_add(1);

            match self.reader.read_line(&mut self.buffer) {
                Ok(0) => {
                    self.done = true;
                    return None;
                }
                Ok(_) => {
                    self.line_number = line_number;
                    if self.buffer.ends_with('\n') {
                        self.buffer.pop();
                    }

                    match self.parser.parse_line(&self.buffer) {
                        Ok(None) => continue,
                        Ok(Some(event)) => {
                            return Some(ThreadEventJsonlRecord {
                                line_number,
                                outcome: Ok(event),
                            });
                        }
                        Err(err) => {
                            return Some(ThreadEventJsonlRecord {
                                line_number,
                                outcome: Err(err),
                            });
                        }
                    }
                }
                Err(err) => {
                    self.done = true;
                    self.line_number = line_number;
                    return Some(ThreadEventJsonlRecord {
                        line_number,
                        outcome: Err(ExecStreamError::from(CodexError::CaptureIo(err))),
                    });
                }
            }
        }
    }
}

pub type ThreadEventJsonlFileReader = ThreadEventJsonlReader<std::io::BufReader<std::fs::File>>;

/// Convenience constructor for reader-backed parsing.
pub fn thread_event_jsonl_reader<R: BufRead>(reader: R) -> ThreadEventJsonlReader<R> {
    ThreadEventJsonlReader::new(reader)
}

/// Convenience constructor for file-backed parsing.
pub fn thread_event_jsonl_file(
    path: impl AsRef<Path>,
) -> Result<ThreadEventJsonlFileReader, ExecStreamError> {
    let file = std::fs::File::open(path.as_ref())
        .map_err(|err| ExecStreamError::from(CodexError::CaptureIo(err)))?;
    Ok(ThreadEventJsonlReader::new(std::io::BufReader::new(file)))
}
