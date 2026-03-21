#[allow(dead_code)]
mod session_selectors;

#[cfg(any(feature = "codex", feature = "claude_code", test))]
mod termination;

#[cfg(any(feature = "codex", feature = "claude_code", test))]
pub(crate) mod spawn_path;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(feature = "codex")]
pub mod codex;

#[cfg(feature = "claude_code")]
pub mod claude_code;
