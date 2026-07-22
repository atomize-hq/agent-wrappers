use std::ffi::OsString;

use crate::CliOverridesPatch;

/// An upstream non-TUI command family maintained through the 0.144.6 packet.
///
/// The enum deliberately omits `completion`: shell-script generation remains a
/// separately tracked architectural gap rather than an accidental raw-command
/// escape hatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NonTuiCommand {
    AppServer,
    AppServerDaemon,
    AppServerDaemonBootstrap,
    AppServerDaemonDisableRemoteControl,
    AppServerDaemonEnableRemoteControl,
    AppServerDaemonHelp,
    AppServerDaemonRestart,
    AppServerDaemonStart,
    AppServerDaemonStop,
    AppServerDaemonVersion,
    Archive,
    Delete,
    Doctor,
    ExecResume,
    ExecReview,
    ExecServer,
    McpAdd,
    PluginAdd,
    PluginList,
    PluginMarketplaceAdd,
    PluginMarketplaceList,
    PluginMarketplaceRemove,
    PluginMarketplaceUpgrade,
    PluginRemove,
    RemoteControl,
    RemoteControlHelp,
    RemoteControlPair,
    RemoteControlStart,
    RemoteControlStop,
    Sandbox,
    Unarchive,
}

impl NonTuiCommand {
    /// Returns every packet-owned command path. This is useful for exhaustive
    /// compatibility tests and intentionally excludes deferred surfaces.
    pub const fn all() -> &'static [Self] {
        &[
            Self::AppServer,
            Self::AppServerDaemon,
            Self::AppServerDaemonBootstrap,
            Self::AppServerDaemonDisableRemoteControl,
            Self::AppServerDaemonEnableRemoteControl,
            Self::AppServerDaemonHelp,
            Self::AppServerDaemonRestart,
            Self::AppServerDaemonStart,
            Self::AppServerDaemonStop,
            Self::AppServerDaemonVersion,
            Self::Archive,
            Self::Delete,
            Self::Doctor,
            Self::ExecResume,
            Self::ExecReview,
            Self::ExecServer,
            Self::McpAdd,
            Self::PluginAdd,
            Self::PluginList,
            Self::PluginMarketplaceAdd,
            Self::PluginMarketplaceList,
            Self::PluginMarketplaceRemove,
            Self::PluginMarketplaceUpgrade,
            Self::PluginRemove,
            Self::RemoteControl,
            Self::RemoteControlHelp,
            Self::RemoteControlPair,
            Self::RemoteControlStart,
            Self::RemoteControlStop,
            Self::Sandbox,
            Self::Unarchive,
        ]
    }

    pub(crate) const fn path(self) -> &'static [&'static str] {
        match self {
            Self::AppServer => &["app-server"],
            Self::AppServerDaemon => &["app-server", "daemon"],
            Self::AppServerDaemonBootstrap => &["app-server", "daemon", "bootstrap"],
            Self::AppServerDaemonDisableRemoteControl => {
                &["app-server", "daemon", "disable-remote-control"]
            }
            Self::AppServerDaemonEnableRemoteControl => {
                &["app-server", "daemon", "enable-remote-control"]
            }
            Self::AppServerDaemonHelp => &["app-server", "daemon", "help"],
            Self::AppServerDaemonRestart => &["app-server", "daemon", "restart"],
            Self::AppServerDaemonStart => &["app-server", "daemon", "start"],
            Self::AppServerDaemonStop => &["app-server", "daemon", "stop"],
            Self::AppServerDaemonVersion => &["app-server", "daemon", "version"],
            Self::Archive => &["archive"],
            Self::Delete => &["delete"],
            Self::Doctor => &["doctor"],
            Self::ExecResume => &["exec", "resume"],
            Self::ExecReview => &["exec", "review"],
            Self::ExecServer => &["exec-server"],
            Self::McpAdd => &["mcp", "add"],
            Self::PluginAdd => &["plugin", "add"],
            Self::PluginList => &["plugin", "list"],
            Self::PluginMarketplaceAdd => &["plugin", "marketplace", "add"],
            Self::PluginMarketplaceList => &["plugin", "marketplace", "list"],
            Self::PluginMarketplaceRemove => &["plugin", "marketplace", "remove"],
            Self::PluginMarketplaceUpgrade => &["plugin", "marketplace", "upgrade"],
            Self::PluginRemove => &["plugin", "remove"],
            Self::RemoteControl => &["remote-control"],
            Self::RemoteControlHelp => &["remote-control", "help"],
            Self::RemoteControlPair => &["remote-control", "pair"],
            Self::RemoteControlStart => &["remote-control", "start"],
            Self::RemoteControlStop => &["remote-control", "stop"],
            Self::Sandbox => &["sandbox"],
            Self::Unarchive => &["unarchive"],
        }
    }
}

/// A bounded pass-through request for a packet-maintained non-TUI command.
///
/// `arguments` are sent verbatim after the selected command path so callers
/// can use upstream-maintained flags without waiting for a crate release. The
/// command itself is an enum, preventing access to deferred shell completion
/// support through this compatibility surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NonTuiCommandRequest {
    pub command: NonTuiCommand,
    pub arguments: Vec<OsString>,
    pub dangerously_bypass_hook_trust: bool,
    pub strict_config: bool,
    pub overrides: CliOverridesPatch,
}

impl NonTuiCommandRequest {
    pub fn new(command: NonTuiCommand) -> Self {
        Self {
            command,
            arguments: Vec::new(),
            dangerously_bypass_hook_trust: false,
            strict_config: false,
            overrides: CliOverridesPatch::default(),
        }
    }

    /// Appends one command-specific flag or positional argument verbatim.
    pub fn arg(mut self, argument: impl Into<OsString>) -> Self {
        self.arguments.push(argument.into());
        self
    }

    /// Appends command-specific arguments verbatim.
    pub fn args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.arguments.extend(arguments.into_iter().map(Into::into));
        self
    }

    /// Passes the root `--dangerously-bypass-hook-trust` flag when enabled.
    pub fn dangerously_bypass_hook_trust(mut self, enable: bool) -> Self {
        self.dangerously_bypass_hook_trust = enable;
        self
    }

    /// Passes the root `--strict-config` flag when enabled.
    pub fn strict_config(mut self, enable: bool) -> Self {
        self.strict_config = enable;
        self
    }

    /// Replaces the default CLI overrides for this request.
    pub fn with_overrides(mut self, overrides: CliOverridesPatch) -> Self {
        self.overrides = overrides;
        self
    }
}
