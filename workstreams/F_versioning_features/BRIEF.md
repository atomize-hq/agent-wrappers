# Workstream F: Versioning and Feature Detection

Objective: Detect Codex binary capabilities and versions to gate flags/features and surface update advisories.

Scope
- Probe binary: `codex --version`, parse version string; optionally cache per binary path.
- Detect features/flags: run `codex features list` and/or `codex --help` parsing; map to capability set used by wrapper to guard flags.
- Update advisory: detect newer releases (npm/Homebrew/GitHub) and expose hooks for host app to download/upgrade (actual download outside the crate).
  - `CodexLatestReleases` + `update_advisory_from_capabilities` compare the probed version to caller-provided latest releases (stable/beta/nightly) and return a `CodexUpdateAdvisory` with status/notes.
  - Hosts fetch latest versions themselves (e.g., `npm view @openai/codex version`, `brew info codex --json`, GitHub releases API) and populate the table; the crate stays offline by default.
- Failure handling: graceful degradation when commands absent or fail.

Constraints
- No network calls unless explicitly configured by host; default to local binary probing.
- Respect env isolation (Workstream A) when spawning codex.

Deliverables
- Capability model (struct of supported flags/features).
- Probing functions with caching keyed by binary path.
- Tests for parsing/version ordering.
- Docs on how host can react to upgrade availability.
- At task completion, agent must write the kickoff prompt for the next task in this workstream branch (not in a worktree).
