# Codex Rust Wrapper

## Capability + versioning release notes (Workstream F)
- Capability probes now capture `codex --version`, `codex features list` (`--json` when available), and `--help` hints, storing results as `CodexCapabilities` snapshots with `collected_at` timestamps and `BinaryFingerprint` metadata keyed by canonical binary path.
- Guard helpers (`guard_output_schema`, `guard_add_dir`, `guard_mcp_login`, `guard_features_list`) keep optional flags off when support is unknown; surface `CapabilityGuard.notes` to operators instead of passing flags blindly.
- Cache controls: configure `CapabilityCachePolicy::{PreferCache, Refresh, Bypass}` via `capability_cache_policy` or `bypass_capability_cache`. Use `Refresh` for TTL/backoff windows or hot-swaps that reuse the same path; use `Bypass` when metadata is missing (FUSE/overlay filesystems) or when you need an isolated probe that skips cache reads/writes.
- Overrides + persistence: `capability_snapshot` / `capability_overrides` accept manual snapshots and feature/version hints; `write_capabilities_snapshot`, `read_capabilities_snapshot`, and `capability_snapshot_matches_binary` let hosts reuse snapshots across processes while avoiding stale data when fingerprints diverge.
- Update advisories stay offline: supply `CodexLatestReleases` and call `update_advisory_from_capabilities` to prompt upgrades without this crate performing network I/O.

## Snapshot reuse + cache policy quickstart
Run the snapshot example to see disk reuse with fingerprint checks plus cache policy guidance:

```
cargo run -p codex --example capability_snapshot -- ./codex ./codex-capabilities.json auto
```

- The example loads a prior snapshot when the fingerprint matches, falls back to `CapabilityCachePolicy::Refresh` after a TTL or hot-swap, and drops to `CapabilityCachePolicy::Bypass` when metadata is missing (typical on some FUSE/overlay mounts) to avoid persisting snapshots that cannot be validated.
- Refresh vs. Bypass: use `Refresh` to re-probe while still writing back to the cache (good for TTL/backoff windows or deployments that reuse the same path); use `Bypass` for one-off probes that should not read or write cache entries when metadata is unreliable.

See `crates/codex/examples/capability_snapshot.rs` for the full flow, including fingerprint validation and snapshot persistence helpers.
