use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudeOutputFormat {
    Text,
    Json,
    StreamJson,
}

impl ClaudeOutputFormat {
    pub(crate) fn as_arg_value(&self) -> &'static str {
        match self {
            ClaudeOutputFormat::Text => "text",
            ClaudeOutputFormat::Json => "json",
            ClaudeOutputFormat::StreamJson => "stream-json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudeInputFormat {
    Text,
    StreamJson,
}

impl ClaudeInputFormat {
    pub(crate) fn as_arg_value(&self) -> &'static str {
        match self {
            ClaudeInputFormat::Text => "text",
            ClaudeInputFormat::StreamJson => "stream-json",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClaudePrintRequest {
    pub(crate) prompt: Option<String>,
    pub(crate) stdin: Option<Vec<u8>>,
    pub(crate) output_format: ClaudeOutputFormat,
    pub(crate) input_format: Option<ClaudeInputFormat>,
    pub(crate) json_schema: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) allowed_tools: Vec<String>,
    pub(crate) disallowed_tools: Vec<String>,
    pub(crate) permission_mode: Option<String>,
    pub(crate) dangerously_skip_permissions: bool,
    pub(crate) add_dirs: Vec<String>,
    pub(crate) mcp_config: Option<String>,
    pub(crate) strict_mcp_config: bool,
    pub(crate) timeout: Option<Duration>,
    pub(crate) extra_args: Vec<String>,
}

impl ClaudePrintRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: Some(prompt.into()),
            stdin: None,
            output_format: ClaudeOutputFormat::Text,
            input_format: None,
            json_schema: None,
            model: None,
            allowed_tools: Vec::new(),
            disallowed_tools: Vec::new(),
            permission_mode: None,
            dangerously_skip_permissions: false,
            add_dirs: Vec::new(),
            mcp_config: None,
            strict_mcp_config: false,
            timeout: None,
            extra_args: Vec::new(),
        }
    }

    pub fn output_format(mut self, format: ClaudeOutputFormat) -> Self {
        self.output_format = format;
        self
    }

    pub fn input_format(mut self, format: ClaudeInputFormat) -> Self {
        self.input_format = Some(format);
        self
    }

    pub fn json_schema(mut self, schema: impl Into<String>) -> Self {
        self.json_schema = Some(schema.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn allowed_tools(mut self, tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.allowed_tools = tools.into_iter().map(Into::into).collect();
        self
    }

    pub fn disallowed_tools(mut self, tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.disallowed_tools = tools.into_iter().map(Into::into).collect();
        self
    }

    pub fn permission_mode(mut self, mode: impl Into<String>) -> Self {
        self.permission_mode = Some(mode.into());
        self
    }

    pub fn dangerously_skip_permissions(mut self, enabled: bool) -> Self {
        self.dangerously_skip_permissions = enabled;
        self
    }

    pub fn add_dirs(mut self, dirs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.add_dirs = dirs.into_iter().map(Into::into).collect();
        self
    }

    pub fn mcp_config(mut self, config: impl Into<String>) -> Self {
        self.mcp_config = Some(config.into());
        self
    }

    pub fn strict_mcp_config(mut self, enabled: bool) -> Self {
        self.strict_mcp_config = enabled;
        self
    }

    pub fn stdin_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.stdin = Some(bytes);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn extra_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.extra_args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn argv(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();

        out.push("--print".to_string());
        out.push("--output-format".to_string());
        out.push(self.output_format.as_arg_value().to_string());

        if let Some(input_format) = self.input_format {
            out.push("--input-format".to_string());
            out.push(input_format.as_arg_value().to_string());
        }

        if let Some(schema) = self.json_schema.as_ref() {
            out.push("--json-schema".to_string());
            out.push(schema.clone());
        }

        if let Some(model) = self.model.as_ref() {
            out.push("--model".to_string());
            out.push(model.clone());
        }

        if !self.allowed_tools.is_empty() {
            out.push("--allowedTools".to_string());
            out.extend(self.allowed_tools.iter().cloned());
        }

        if !self.disallowed_tools.is_empty() {
            out.push("--disallowedTools".to_string());
            out.extend(self.disallowed_tools.iter().cloned());
        }

        if let Some(mode) = self.permission_mode.as_ref() {
            out.push("--permission-mode".to_string());
            out.push(mode.clone());
        }

        if self.dangerously_skip_permissions {
            out.push("--dangerously-skip-permissions".to_string());
        }

        if !self.add_dirs.is_empty() {
            out.push("--add-dir".to_string());
            out.extend(self.add_dirs.iter().cloned());
        }

        if let Some(config) = self.mcp_config.as_ref() {
            out.push("--mcp-config".to_string());
            out.push(config.clone());
        }

        if self.strict_mcp_config {
            out.push("--strict-mcp-config".to_string());
        }

        out.extend(self.extra_args.iter().cloned());

        if let Some(prompt) = self.prompt.as_ref() {
            out.push(prompt.clone());
        }

        out
    }
}
