# Manual Testing Playbook — Universal Agent API

Status: Draft  
Date (UTC): 2026-02-16

This playbook is a non-gating manual validation path for maintainers who have access to real Codex
and Claude Code CLIs. The automated gates for this feature must remain fixture/sample-based and run
on GitHub-hosted runners.

## Preconditions (optional)

- Codex CLI available (either on `PATH` or via `agent_api::backends::codex::CodexBackendConfig.binary`).
- Claude Code CLI available (either on `PATH` or via `agent_api::backends::claude_code::ClaudeCodeBackendConfig.binary`).
- A scratch `CODEX_HOME`/work directory under a temp folder.

## Playbook

1. Build the workspace:
   - `cargo build --workspace --all-targets --all-features`
2. Run universal API unit tests:
   - `cargo test --workspace --all-targets --all-features`
3. If Codex CLI is available:
   - run the `agent_api` Codex backend integration example (if implemented)
   - confirm streaming events produce a mix of `Status`, `TextOutput`, and `ToolCall` where applicable
4. If Claude Code CLI is available:
   - run the `agent_api` Claude backend example (if implemented)
   - confirm buffered events are returned and capability gating behaves as specified
5. Record results in `session_log.md` under the appropriate integration task.
