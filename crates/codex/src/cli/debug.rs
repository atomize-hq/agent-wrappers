use crate::CliOverridesPatch;

/// Request for `codex debug`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugCommandRequest {
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl DebugCommandRequest {
    pub fn new() -> Self {
        Self {
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Replaces the default CLI overrides for this request.
    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }
}

impl Default for DebugCommandRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Request for `codex debug help [COMMAND]...`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugHelpRequest {
    /// Optional command tokens passed after `help` (variadic).
    pub command: Vec<String>,
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl DebugHelpRequest {
    pub fn new() -> Self {
        Self {
            command: Vec::new(),
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Sets the optional `COMMAND` tokens.
    pub fn command(mut self, command: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.command = command.into_iter().map(Into::into).collect();
        self
    }

    /// Replaces the default CLI overrides for this request.
    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }
}

impl Default for DebugHelpRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Request for `codex debug app-server`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugAppServerRequest {
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl DebugAppServerRequest {
    pub fn new() -> Self {
        Self {
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Replaces the default CLI overrides for this request.
    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }
}

impl Default for DebugAppServerRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Request for `codex debug app-server help [COMMAND]...`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugAppServerHelpRequest {
    /// Optional command tokens passed after `help` (variadic).
    pub command: Vec<String>,
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl DebugAppServerHelpRequest {
    pub fn new() -> Self {
        Self {
            command: Vec::new(),
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Sets the optional `COMMAND` tokens.
    pub fn command(mut self, command: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.command = command.into_iter().map(Into::into).collect();
        self
    }

    /// Replaces the default CLI overrides for this request.
    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }
}

impl Default for DebugAppServerHelpRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Request for `codex debug app-server send-message-v2 <USER_MESSAGE>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DebugAppServerSendMessageV2Request {
    /// Message payload sent to the app-server debug shim.
    pub user_message: String,
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl DebugAppServerSendMessageV2Request {
    pub fn new(user_message: impl Into<String>) -> Self {
        Self {
            user_message: user_message.into(),
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Replaces the default CLI overrides for this request.
    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }
}
