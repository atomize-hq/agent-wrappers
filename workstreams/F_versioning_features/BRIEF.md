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

## Consuming capability guards
- Probe once per binary (`probe_capabilities`) and consult guards before enabling optional flags like `--output-schema`, `--add-dir`, or `login --mcp`. The wrapper skips requested flags when support is unknown to stay compatible with older releases.
- Example:
```rust
let capabilities = client.probe_capabilities().await;
let output_schema = capabilities.guard_output_schema();
let add_dir = capabilities.guard_add_dir();

let client = CodexClient::builder()
    .binary("/path/to/codex")
    .output_schema(output_schema.is_supported())
    .add_dir("/workspace")
    .build();

if let Some(child) = client.spawn_mcp_login_process().await? {
    // handle MCP login child or drop it to kill the helper
}
```
- When guards are `Unknown`, skip the optional flags and surface the guard notes to operators rather than attempting the flag blindly.

## Capability overrides
- Hosts can skip probes for pinned/bundled binaries by supplying a manual snapshot via `CodexClientBuilder::capability_snapshot(...)`. The snapshot is cached against the canonical binary path and current fingerprint; when the on-disk binary changes the cache entry is invalidated and the snapshot is re-applied.
- Version/feature overrides (`capability_version_override`, `capability_feature_overrides`, or the `capability_feature_hints` helper) apply after cache hits or probes run. Manual snapshots take precedence over cached/probed data, then version overrides, then feature overrides.
- Feature overrides let vendors opt into bespoke flags (`capability_feature_overrides` to force true/false; `capability_feature_hints` to only force-enable). Guard helpers and probes consume the merged snapshot so callers can combine manual hints with probed data.
