# MCP + app-server handoff (E15)

Brief release note
- Documented MCP runtime, app runtime, and pool APIs with their non-destructive guarantees and env merging expectations.
- Added MCP + app-server example coverage in `EXAMPLES.md` to mirror shipped flows without touching stored config or thread metadata.

Checklist
- [x] Runtime APIs stay read-only; listing/prepare leaves `config.toml` unchanged (tests: `runtime_api_lists_launchers_without_changing_config`, `runtime_api_prepare_http_is_non_destructive`).
- [x] App runtime APIs merge defaults and preserve metadata/resume hints (tests: `app_runtime_api_lists_and_merges_without_writes`, `app_runtime_lifecycle_starts_and_stops_without_mutation`).
- [x] Pool reuse/restart semantics covered without mutating stored definitions (tests: `app_runtime_pool_api_reuses_and_restarts_stdio`, `app_runtime_pool_api_stop_all_shuts_down_runtimes`).
- [x] Examples align with documented behavior and avoid writing config/thread metadata (`mcp_codex_flow`, `app_server_turns`).
- [x] Tests: `cargo test -p codex`.
