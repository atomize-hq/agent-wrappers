# Codex Wrapper Backlog

High-priority items to make the Rust wrapper production-ready, cover Codex CLI surface area, and stay version-safe. Grouped into workstreams to enable concurrent progress.

## Workstream A: Binary + env isolation
1. Add builder opts for pinned binary path (default to bundled) and per-invocation `CODEX_HOME` override.
2. Helper to compute app-scoped CODEX_HOME (e.g., `~/.myhub/codex`) and mkdir/log path discovery (`config.toml`, `auth.json`, `.credentials.json`, `history.jsonl`, `conversations/*.jsonl`, `logs/codex-*.log`).
3. Expose composite env prep (CODEX_HOME + CODEX_BINARY) when spawning any subcommand.

## Workstream B: CLI surface coverage (exec/interactive)
1. `exec` flags: `--image/-i`, `--model/-m`, `--oss`, `--sandbox/-s`, `--profile/-p`, `--full-auto`, `--dangerously-bypass-approvals-and-sandbox/--yolo`, `--cd/-C`, `--add-dir`, `--skip-git-repo-check`, `--output-schema`, `--color`, `--json`, `--output-last-message/-o`.
2. Interactive (no subcommand): mirror exec options + optional session resume hints.
3. `resume`: resume by ID or `--last`.
4. `apply`: apply latest diff; capture stdout/stderr/exit.
5. `sandbox`: seatbelt/landlock/windows runners.
6. `features`: list/enable/disable.

## Workstream C: Auth + sessions
1. `login`/`logout`: ChatGPT OAuth, API key stdin (`--with-api-key`), device auth, issuer/client overrides.
2. Status parsing via `codex login status` (prefer `--json` if/when present).
3. Credential store modes (`File/Keyring/Auto`) for core auth and MCP OAuth.
4. Session reuse: pass session IDs, `--last`, load conversation files; history helpers for `history.jsonl` and `conversations/*.jsonl`.

## Workstream D: JSON streaming + logging
1. Typed event stream for `--json` output (thread/turn/item lifecycle; item types agent_message, reasoning, command_execution, file_change, mcp_tool_call, web_search, todo_list, errors).
2. Real-time streaming: flush per event; idle timeout surfaced as error; expose `--output-last-message` helper and schema path handling.
3. Log tee: opt-in mirroring to files; honor `RUST_LOG`; expose log path helper.
4. Apply/diff artifacts: return apply exit status/stdout/stderr.

## Workstream E: MCP + app-server
1. `mcp` management: `list/get/add/remove/login/logout` with JSON, env injection on add; support `[mcp_servers]` stdio + streamable_http definitions (headers, bearer env var, timeouts, enabled/disabled tools).
2. Launch helpers for `codex mcp-server` and `codex app-server` (stdio JSON-RPC); lifecycle management (spawn/kill, health check).
3. Tool params: `codex` (start session) and `codex-reply` (continue by conversationId) with prompt/cwd/model/sandbox/approval/config map; stream events to caller.
4. Approval elicitation plumbing: surface exec/apply approvals and cancellation handling.

## Workstream F: Versioning + feature detection
1. Binary version probe (`codex --version`) and `codex features list`; cache supported flags/features per binary.
2. Update flow: detect newer releases (npm/Homebrew/github), emit advisory; optional downloader hook (outside crate) but expose hooks to plug in.
3. Capability guards: gate new flags behind detection; graceful degradation for older binaries.

## Workstream G: Notifications + long-running tasks
1. Support external `notify` hook (run program with JSON payload on turn complete).
2. Ensure mcp-server/app-server event notifications bubble up to orchestration layer for wake-ups on task completion.
3. Long-running task resume: expose `codex-reply` helper for conversationId + prompt; cancellation support.

## Workstream H: Examples + docs
1. Add examples for MCP server/app-server usage, JSON streaming, resume/apply, feature toggles, CODEX_HOME override, and bundled binary selection.
2. API docs describing env resolution, event schema, approval/sandbox policies.

## Cross-cutting: Auth orchestration
- Add orchestration around `auth.json`/`.credentials.json` materialization when `CODEX_HOME` is set, so isolated homes can reuse or copy authenticated state without re-running `codex login`. Provide helpers to detect and copy existing auth files into a target `CODEX_HOME` safely.

## References: neighboring crates
- `codex-helper` (proxy/failover/logging; rewrites ~/.codex/config.toml to point Codex at local proxy; manages multi-upstream, quota-aware routing; sessions/usage diagnostics). Consider ideas for multi-provider routing and config backup/restore, not code.
- `llm-link` (multi-provider proxy with app presets for Codex CLI/Zed/Claude Code, supports OpenAI/Ollama/Anthropic etc., hot reload config, dynamic model discovery). Useful patterns for multi-protocol adapters, but likely out of scope for core Codex wrapper.
