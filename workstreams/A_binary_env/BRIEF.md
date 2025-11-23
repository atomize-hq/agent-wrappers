# Workstream A: Binary + Environment Isolation

Objective: Ensure the wrapper always runs against a pinned, bundled Codex binary and uses an app-scoped `CODEX_HOME` per invocation so user installations/configs remain untouched. Provide a unified env-prep layer for all Codex spawns.

Scope
- Binary selection: builder options to set explicit binary path (default to bundled) and optional `CODEX_BINARY` env override.
- CODEX_HOME isolation: compute and apply app-private home (e.g., `~/.myhub/codex`), ensure directories exist, expose path helpers.
- Env injection: centralize env setup for all subcommands (exec, login, mcp, app-server, etc.).
- No behavioral changes to higher-level flows beyond using the env-prep layer.

Constraints
- Rust 2021, pinned toolchain 1.78.
- All changes under `crates/codex` unless adding docs/examples elsewhere.
- Maintain backward compat for callers that omit new options.

Key references
- Current client/builder: `crates/codex/src/lib.rs`.
- Default binary env var: `CODEX_BINARY` in `default_binary_path()`.
- Known state locations (when CODEX_HOME is set): `config.toml`, `auth.json`, `.credentials.json`, `history.jsonl`, `conversations/*.jsonl`, `logs/codex-*.log`.
- Desired additions: helper to compute/apply CODEX_HOME, env map merge, builder fields for binary_path and codex_home.

Deliverables
- New builder API for binary + CODEX_HOME.
- Env-prep helper used by all process spawns.
- Tests covering env overrides and defaulting.
- Docs snippet in README/examples (can be coordinated with Workstream H).
- At task completion, agent must write the kickoff prompt for the next task in this workstream branch (not in a worktree).
- Kickoff prompts must follow `workstreams/KICKOFF_GUIDE.md` (includes required merge steps).

Assumptions
- Bundled binary path will be provided by the host app; crate just accepts a path.
- CODEX_HOME override should not leak into parent process; apply per-command env.
