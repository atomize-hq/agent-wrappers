# Workstream F: Versioning and Feature Detection

Objective: Detect Codex binary capabilities and versions to gate flags/features and surface update advisories.

Scope
- Probe binary: `codex --version`, parse version string; optionally cache per binary path.
- Detect features/flags: run `codex features list` and/or `codex --help` parsing; map to capability set used by wrapper to guard flags.
- Update advisory: detect newer releases (npm/Homebrew/GitHub) and expose hooks for host app to download/upgrade (actual download outside the crate).
  - `CodexLatestReleases` + `update_advisory_from_capabilities` compare the probed version to caller-provided latest releases (stable/beta/nightly) and return a `CodexUpdateAdvisory` with status/notes.
  - Hosts fetch latest versions themselves (e.g., `npm view @openai/codex version`, `brew info codex --json`, GitHub releases API) and populate the table; the crate stays offline by default.
- Failure handling: graceful degradation when commands absent or fail.
- Snapshot persistence: serialize/deserialize capability snapshots and overrides (JSON/TOML) so hosts can cache probe results on disk keyed by canonical binary path and fingerprint.

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

## Capability cache controls
- Inspect or reset cache entries via `capability_cache_entries`, `capability_cache_entry`, `clear_capability_cache_entry`, and `clear_capability_cache` (useful for hosts that want to mirror/evict probe results after deployments).
- Cache interaction is driven by `CapabilityCachePolicy` (builder: `capability_cache_policy`, convenience: `bypass_capability_cache`; per-call: `probe_capabilities_with_policy`). Default is `PreferCache`, which uses fingerprint-aware invalidation and skips cache reuse/writes when file metadata is missing.
- Use `CapabilityCachePolicy::Refresh` to force a fresh probe and overwrite the cache even when the fingerprint is unchanged (good for TTL/backoff windows or hot-swaps that reuse the same path). Use `CapabilityCachePolicy::Bypass` when you want a fresh snapshot without touching the cache.
- TTL/backoff helper: `capability_cache_ttl_decision` consumes `collected_at` and fingerprint presence to pick `Refresh` vs `Bypass`; start with a ~5 minute TTL when fingerprints exist and stretch the window toward 10-15 minutes on FUSE/overlay paths where metadata keeps missing.
- When metadata/fingerprints are unavailable (e.g., FUSE/overlay filesystems), probes bypass the cache automatically and avoid writing entries; callers should apply a backoff/TTL (start around 5 minutes and stretch toward 10-15) to avoid hammering the binary when repeated stats fail.
- For hot-swapped binaries, prefer clearing the per-binary cache entry or forcing a `Refresh` probe after deploys (the TTL helper does this automatically when the window is exceeded); otherwise rely on fingerprint invalidation plus a modest TTL so long-running hosts do not carry stale capability data.

## Post-workstream audit (F9)
- Workstream deliverables shipped: capability model + feature guards with cache policies, override + snapshot persistence helpers, and update advisory plumbing with semver-aware parsing/tests.
- Host integration notes:
  - Share capability snapshots across processes by persisting them via `write_capabilities_snapshot` / `read_capabilities_snapshot`, checking `capability_snapshot_matches_binary`, and optionally applying `CapabilityCachePolicy::Refresh` after deploys to refresh fingerprints.
  - The in-process cache is intentionally scoped to the current process; long-lived hosts should periodically refresh using `Refresh` or `Bypass` and can layer a TTL on `CodexCapabilities.collected_at`.
  - Update advisories stay offline; hosts must supply latest release tables (npm/Homebrew/GitHub) before calling `update_advisory` / `update_advisory_from_capabilities` and should surface advisory.notes to operators.
- Backlog/follow-ups to hand off:
  - Done: release/README notes describing capability detection, guard helpers, cache policies, overrides, and advisories (see `crates/codex/README.md` and crate docs).
  - Done: host-facing example showing disk snapshot reuse with fingerprint checks and `Refresh` vs `Bypass` guidance for TTL/backoff on hot-swap or FUSE-like paths (`crates/codex/examples/capability_snapshot.rs`).
  - Done: TTL/backoff helper (`capability_cache_ttl_decision`) that inspects `collected_at` and fingerprint presence to decide between `Refresh` and `Bypass` when metadata is missing or binaries are hot-swapped.
