# Codex Rust Wrapper

This crate shells out to the Codex CLI and adds async helpers for the MCP server and app-server.
This note focuses on the runtime and pool APIs and their non-destructive behavior.

## Runtime definitions and env prep
- `[mcp_servers]` and `[app_runtimes]` live in `config.toml`; `McpConfigManager` reads/writes them.
- `StdioServerConfig` should be built with the Workstream A env prep (binary path, `CODEX_HOME`, base env, timeouts). Runtime entries layer env/timeout overrides on top of those defaults, and `CODEX_HOME` is injected when `code_home` is set.
- Resolution through the runtime/app APIs is read-only: stored config and metadata are not mutated.

## MCP runtime API (read-only)
- `McpRuntimeApi::from_config(&manager, &defaults)` loads launch-ready stdio configs or HTTP connectors from stored runtimes.
- `available` returns `McpRuntimeSummary` entries (description/tags/tool hints + transport kind).
- `launcher`, `stdio_launcher`, and `http_connector` hand back launchers/connectors without side effects; HTTP connectors resolve bearer tokens from env without overwriting existing `Authorization` headers.
- `prepare` spawns stdio runtimes or hands back HTTP connectors with tool hints preserved; use `ManagedStdioRuntime::stop` to shut down processes (drop is best-effort kill).
- Use `McpRuntimeManager` directly when you already have launchers and only need spawn/connector plumbing.

## App runtime API (read-only)
- `AppRuntimeApi::from_config(&manager, &defaults)` merges stored `[app_runtimes]` entries with defaults (binary/path/env/timeout) while keeping metadata/resume hints intact.
- `available` lists stored runtimes and metadata; `prepare`/`stdio_config` return merged stdio configs without launching.
- `start` launches an app-server and returns `ManagedAppRuntime` (metadata + merged env + `CodexAppServer` handle). Calls leave stored definitions untouched and preserve metadata for future starts.

## Pooled app runtimes
- `AppRuntimePoolApi::from_config(&manager, &defaults)` (or `AppRuntimeApi::pool_api`) wraps the pool that reuses running runtimes by name.
- `available` lists stored entries; `running` lists active runtimes; `start` reuses an existing process if one is already running; `stop`/`stop_all` clean up without altering stored definitions or metadata/resume hints.
- Pool handles still expose stdio configs via `launcher`/`prepare` so callers can inspect launch parameters without starting a process.

## Examples and tests
- `examples/mcp_codex_flow.rs`: starts `codex mcp-server`, streams `codex/event`, supports `$ /cancelRequest` and follow-up `codex/codex-reply`; respects `CODEX_BINARY`/`CODEX_HOME` and does not touch stored `[mcp_servers]`.
- `examples/app_server_turns.rs`: starts/resumes `codex app-server` threads, streams items/task_complete, and can issue `turn/interrupt`; metadata/thread IDs come from server responses and are not persisted by the wrapper.
- `cargo test -p codex` exercises env merging and non-destructive behavior (`runtime_api_*`, `app_runtime_*`, `app_runtime_pool_*` cover listing/prepare/start/stop without writing config or altering metadata).
