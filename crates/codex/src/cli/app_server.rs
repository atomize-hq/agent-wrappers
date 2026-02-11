use crate::{CliOverridesPatch, ConfigOverride, FlagState};
use std::{path::PathBuf, process::ExitStatus};

/// Target for app-server code generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppServerCodegenTarget {
    /// Emits TypeScript bindings for the app-server protocol. Optionally formats the output with Prettier.
    TypeScript { prettier: Option<PathBuf> },
    /// Emits a JSON schema bundle for the app-server protocol.
    JsonSchema,
}

impl AppServerCodegenTarget {
    pub(crate) fn subcommand(&self) -> &'static str {
        match self {
            AppServerCodegenTarget::TypeScript { .. } => "generate-ts",
            AppServerCodegenTarget::JsonSchema => "generate-json-schema",
        }
    }

    pub(crate) fn prettier(&self) -> Option<&PathBuf> {
        match self {
            AppServerCodegenTarget::TypeScript { prettier } => prettier.as_ref(),
            AppServerCodegenTarget::JsonSchema => None,
        }
    }
}

/// Request for `codex app-server generate-ts` or `generate-json-schema`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppServerCodegenRequest {
    /// Codegen target and optional Prettier path (TypeScript only).
    pub target: AppServerCodegenTarget,
    /// Output directory passed to `--out`; created if missing.
    pub out_dir: PathBuf,
    /// Passes `--experimental` to the app-server codegen subcommand when enabled.
    pub experimental: bool,
    /// Per-call CLI overrides layered on top of the builder.
    pub overrides: CliOverridesPatch,
}

impl AppServerCodegenRequest {
    /// Generates TypeScript bindings into `out_dir`.
    pub fn typescript(out_dir: impl Into<PathBuf>) -> Self {
        Self {
            target: AppServerCodegenTarget::TypeScript { prettier: None },
            out_dir: out_dir.into(),
            experimental: false,
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Generates a JSON schema bundle into `out_dir`.
    pub fn json_schema(out_dir: impl Into<PathBuf>) -> Self {
        Self {
            target: AppServerCodegenTarget::JsonSchema,
            out_dir: out_dir.into(),
            experimental: false,
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Controls whether `--experimental` is passed to the codegen subcommand.
    pub fn experimental(mut self, enable: bool) -> Self {
        self.experimental = enable;
        self
    }

    /// Formats TypeScript output with the provided Prettier executable (no-op for JSON schema).
    pub fn prettier(mut self, prettier: impl Into<PathBuf>) -> Self {
        if let AppServerCodegenTarget::TypeScript { prettier: slot } = &mut self.target {
            *slot = Some(prettier.into());
        }
        self
    }

    /// Replaces the default CLI overrides for this request.
    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }

    /// Adds a `--config key=value` override for this request.
    pub fn config_override(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::new(key, value));
        self
    }

    /// Adds a raw `--config key=value` override without validation.
    pub fn config_override_raw(mut self, raw: impl Into<String>) -> Self {
        self.overrides
            .config_overrides
            .push(ConfigOverride::from_raw(raw));
        self
    }

    /// Sets the config profile (`--profile`) for this request.
    pub fn profile(mut self, profile: impl Into<String>) -> Self {
        let profile = profile.into();
        self.overrides.profile = (!profile.trim().is_empty()).then_some(profile);
        self
    }

    /// Requests the CLI `--oss` flag for this codegen call.
    pub fn oss(mut self, enable: bool) -> Self {
        self.overrides.oss = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }

    /// Adds a `--enable <feature>` toggle for this codegen call.
    pub fn enable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.enable.push(name.into());
        self
    }

    /// Adds a `--disable <feature>` toggle for this codegen call.
    pub fn disable_feature(mut self, name: impl Into<String>) -> Self {
        self.overrides.feature_toggles.disable.push(name.into());
        self
    }

    /// Controls whether `--search` is passed through to Codex.
    pub fn search(mut self, enable: bool) -> Self {
        self.overrides.search = if enable {
            FlagState::Enable
        } else {
            FlagState::Disable
        };
        self
    }
}

/// Captured output from app-server codegen commands.
#[derive(Clone, Debug)]
pub struct AppServerCodegenOutput {
    /// Exit status returned by the subcommand.
    pub status: ExitStatus,
    /// Captured stdout (mirrored to the console when `mirror_stdout` is true).
    pub stdout: String,
    /// Captured stderr (mirrored unless `quiet` is set).
    pub stderr: String,
    /// Output directory passed to `--out`.
    pub out_dir: PathBuf,
}
