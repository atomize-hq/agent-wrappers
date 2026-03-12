#[allow(dead_code)]
mod session_selectors;

#[cfg(any(feature = "codex", feature = "claude_code", test))]
mod termination;

#[cfg(any(feature = "codex", feature = "claude_code", test))]
mod spawn_path;

#[cfg(feature = "codex")]
pub mod codex;

#[cfg(feature = "claude_code")]
pub mod claude_code;
