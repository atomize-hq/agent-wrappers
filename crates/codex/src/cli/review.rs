use crate::CliOverridesPatch;

/// Request for `codex review [OPTIONS] [PROMPT]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewCommandRequest {
    pub prompt: Option<String>,
    pub base: Option<String>,
    pub commit: Option<String>,
    pub title: Option<String>,
    pub uncommitted: bool,
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl ReviewCommandRequest {
    pub fn new() -> Self {
        Self {
            prompt: None,
            base: None,
            commit: None,
            title: None,
            uncommitted: false,
            overrides: CliOverridesPatch::default(),
        }
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        let prompt = prompt.into();
        self.prompt = (!prompt.trim().is_empty()).then_some(prompt);
        self
    }

    pub fn base(mut self, branch: impl Into<String>) -> Self {
        let branch = branch.into();
        self.base = (!branch.trim().is_empty()).then_some(branch);
        self
    }

    pub fn commit(mut self, sha: impl Into<String>) -> Self {
        let sha = sha.into();
        self.commit = (!sha.trim().is_empty()).then_some(sha);
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        let title = title.into();
        self.title = (!title.trim().is_empty()).then_some(title);
        self
    }

    pub fn uncommitted(mut self, enable: bool) -> Self {
        self.uncommitted = enable;
        self
    }

    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }
}

impl Default for ReviewCommandRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Request for `codex exec review [OPTIONS] [PROMPT]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecReviewCommandRequest {
    pub prompt: Option<String>,
    pub base: Option<String>,
    pub commit: Option<String>,
    pub title: Option<String>,
    pub uncommitted: bool,
    pub json: bool,
    pub skip_git_repo_check: bool,
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl ExecReviewCommandRequest {
    pub fn new() -> Self {
        Self {
            prompt: None,
            base: None,
            commit: None,
            title: None,
            uncommitted: false,
            json: false,
            skip_git_repo_check: true,
            overrides: CliOverridesPatch::default(),
        }
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        let prompt = prompt.into();
        self.prompt = (!prompt.trim().is_empty()).then_some(prompt);
        self
    }

    pub fn base(mut self, branch: impl Into<String>) -> Self {
        let branch = branch.into();
        self.base = (!branch.trim().is_empty()).then_some(branch);
        self
    }

    pub fn commit(mut self, sha: impl Into<String>) -> Self {
        let sha = sha.into();
        self.commit = (!sha.trim().is_empty()).then_some(sha);
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        let title = title.into();
        self.title = (!title.trim().is_empty()).then_some(title);
        self
    }

    pub fn uncommitted(mut self, enable: bool) -> Self {
        self.uncommitted = enable;
        self
    }

    pub fn json(mut self, enable: bool) -> Self {
        self.json = enable;
        self
    }

    pub fn skip_git_repo_check(mut self, enable: bool) -> Self {
        self.skip_git_repo_check = enable;
        self
    }

    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }
}

impl Default for ExecReviewCommandRequest {
    fn default() -> Self {
        Self::new()
    }
}
