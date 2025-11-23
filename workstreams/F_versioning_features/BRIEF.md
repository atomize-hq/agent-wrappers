# Workstream F: Versioning and Feature Detection

Objective: Detect Codex binary capabilities and versions to gate flags/features and surface update advisories.

Scope
- Probe binary: `codex --version`, parse version string; optionally cache per binary path.
- Detect features/flags: run `codex features list` and/or `codex --help` parsing; map to capability set used by wrapper to guard flags.
- Update advisory: detect newer releases (npm/Homebrew/GitHub) and expose hooks for host app to download/upgrade (actual download outside the crate).
- Failure handling: graceful degradation when commands absent or fail.

Constraints
- No network calls unless explicitly configured by host; default to local binary probing.
- Respect env isolation (Workstream A) when spawning codex.

Deliverables
- Capability model (struct of supported flags/features).
- Probing functions with caching keyed by binary path.
- Tests for parsing/version ordering.
- Docs on how host can react to upgrade availability.
