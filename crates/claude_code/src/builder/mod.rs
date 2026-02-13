use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use crate::client::ClaudeClient;

#[derive(Debug, Clone)]
pub struct ClaudeClientBuilder {
    pub(crate) binary: Option<PathBuf>,
    pub(crate) working_dir: Option<PathBuf>,
    pub(crate) env: BTreeMap<String, String>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) mirror_stdout: bool,
    pub(crate) mirror_stderr: bool,
}

impl Default for ClaudeClientBuilder {
    fn default() -> Self {
        Self {
            binary: None,
            working_dir: None,
            env: BTreeMap::new(),
            timeout: Some(Duration::from_secs(120)),
            mirror_stdout: false,
            mirror_stderr: false,
        }
    }
}

impl ClaudeClientBuilder {
    pub fn binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = Some(binary.into());
        self
    }

    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn mirror_stdout(mut self, enabled: bool) -> Self {
        self.mirror_stdout = enabled;
        self
    }

    pub fn mirror_stderr(mut self, enabled: bool) -> Self {
        self.mirror_stderr = enabled;
        self
    }

    pub fn build(mut self) -> ClaudeClient {
        // Avoid any updater side effects by default; callers may override explicitly.
        self.env
            .entry("DISABLE_AUTOUPDATER".to_string())
            .or_insert_with(|| "1".to_string());

        ClaudeClient {
            binary: self.binary,
            working_dir: self.working_dir,
            env: self.env,
            timeout: self.timeout,
            mirror_stdout: self.mirror_stdout,
            mirror_stderr: self.mirror_stderr,
        }
    }
}

