# Codex Wrapper Backlog

High-priority items to make the Rust wrapper production-ready and cover Codex CLI surface area.

## CLI Surface Coverage
- `exec`: support all flags (`--image/-i`, `--model/-m`, `--oss`, `--sandbox/-s`, `--profile/-p`, `--full-auto`, `--dangerously-bypass-approvals-and-sandbox/--yolo`, `--cd/-C`, `--skip-git-repo-check`, `--add-dir`, `--output-schema`, `--color`, `--json`, `--output-last-message/-o`); expose structured builder fields.
- Interactive (no subcommand): mirror `exec` options plus session resume behavior.
- `login`/`logout`: cover ChatGPT OAuth, API key input (`--with-api-key`), device auth (`--device-auth`), issuer/client overrides.
- `resume`: resume by ID or `--last`.
- `apply`: apply latest diff; surface stdout/stderr and exit codes.
- `sandbox`: seatbelt/landlock/windows wrappers.
- `features`: list/enable/disable feature flags.
- `mcp`: `list/get/add/remove/login/logout` with JSON output and `--env` on add.
- `mcp-server` and `app-server`: support launch helpers (stdio MCP server mode).

## Configuration and State
- Respect config precedence: CLI flags > env vars > `$CODEX_HOME/config.toml`.
- Manage `CODEX_HOME` (default `~/.codex`); expose helpers to locate paths:
  - `config.toml`, `auth.json`, `.credentials.json`, `history.jsonl`, `conversations/*.jsonl`, `logs/codex-*.log`.
- Read/write `config.toml` fragments for MCP servers (`[mcp_servers]` with stdio/streamable_http transports, timeouts, enabled/disabled tools).
- Expose profile selection (`[profiles.<name>]`).
- Support credential store mode (`File/Keyring/Auto`) for both core auth and MCP OAuth tokens.

## Sessions and Logging
- Session reuse: allow passing session IDs, `--last`, and loading conversation files.
- History handling: read/append `history.jsonl` and per-session JSONL rolls.
- Log handling: opt-in tee to log files, honor `RUST_LOG`, expose log path helper.
- Apply/diff artifacts: capture and return apply exit status/stdout/stderr.

## JSON Streaming
- Provide typed event stream for `--json` output (ThreadEvent, item types: agent_message, reasoning, command_execution, file_change, mcp_tool_call, web_search, todo_list, errors).
- Write `--output-last-message` helper; accept output schema path.
- Handle tool call lifecycle (begin/end), errors, and final messages.

## MCP Support
- `mcp add`: stdio server command/args/env/cwd; streamable HTTP fields (url, headers, bearer env var).
- `mcp login/logout`: when `experimental_use_rmcp_client = true`.
- Allow enabling/disabling tools per server; tune `startup_timeout_sec` and `tool_timeout_sec`.
- Run Codex as MCP server (`codex mcp-server`) with configurable stdin/stdout plumbing.

## Sandbox and Approval Policies
- Surface approval policies (`UnlessTrusted`, `OnFailure`, `OnRequest`, `Never`) and sandbox modes (`ReadOnly`, `WorkspaceWrite`, `DangerFullAccess`).
- Map convenience flags (`--full-auto`, `--yolo`) to their composite settings.
- Allow extra writable dirs (`--add-dir`) and working dir overrides (`--cd`).

## Auth
- ChatGPT OAuth login spawn helper (existing).
- API key login via stdin flag (`--with-api-key`); improve status parsing by reading `codex login status --json` when available.
- MCP OAuth credential storage and logout support.

## Ergonomics and Reliability
- Timeouts with clearer error types; support zero timeout = no limit.
- Mirror/quiet controls for stdout/stderr.
- Color mode controls; defaults to deterministic.
- Expose binary discovery via `CODEX_BINARY` env var override.
- Add examples for MCP, exec JSON streaming, resume, apply, and feature toggles.
