# Task H1: Documentation and Example Coverage Plan

Goal: blueprint the docs and runnable examples needed for the upcoming features: bundled binary + `CODEX_HOME`, JSON streaming API, MCP/app-server helpers, and feature detection/version gating.

## Current State
- No top-level README is present in this branch; `crates/codex/EXAMPLES.md` lists only basic exec examples (prompt, timeout, working dir, `--json`, color, model, images, env `CODEX_BINARY`).
- Examples directory (`crates/codex/examples/`) covers only baseline exec flows; nothing for streaming consumers, MCP/app-server, feature detection, or `CODEX_HOME`.

## Planned README Structure (for H3)
- **Overview & install**: describe the crate purpose, minimal `Cargo.toml` snippet, and default binary resolution (bundled path first, fall back to `CODEX_BINARY` env or explicit builder override).
- **CODEX_HOME/binary isolation**: section explaining how the env-prep layer sets `CODEX_HOME`, helper to compute app-scoped home, and how to override binary path; include quick example for setting both on a builder and note which files live under `CODEX_HOME`.
- **Exec APIs**:
  - Basic `send_prompt` usage and existing stdout behavior.
  - **Streaming JSON API**: how to opt into typed event stream, handling of `--output-last-message` and `--output-schema` paths, idle timeout behavior, and log tee options.
  - Output artifacts: where apply/diff/stdout/stderr surface in the new API.
- **MCP + app-server**: lifecycle overview (spawn, handle notifications, shut down). Show codex/codex-reply tool params, approval/cancel flows, and app-server thread/turn flows.
- **Feature detection & versioning**: how to probe `codex --version`/`features list`, what the capability struct looks like, how to gate flags, and how to react to upgrade advisories (host-provided download hook).
- **Examples index**: link to the EXAMPLES guide and list new examples by feature with commands (see below).

## Planned EXAMPLES Guide Updates (H3)
- Keep the wrapper vs. native CLI comparison table but split into sections:
  - **Basics** (existing rows).
  - **Binary/CODEX_HOME isolation** (new bundled binary + app-scoped home rows).
  - **Streaming/logging** (typed JSON stream consumer, log tee, last-message + schema handling).
  - **MCP/app-server** (codex + codex-reply tool flows, app-server thread/turn).
  - **Feature detection/versioning** (capability probe + gating example).
- For each new example, document:
  - Wrapper command (`cargo run -p codex --example ...`) including required env vars.
  - Equivalent native `codex`/`codex mcp-server`/`codex app-server` invocation(s).
  - Short note on what to expect (events surfaced, approval prompts, where artifacts land).

## New/Updated Examples to Add (H2)
- **Binary isolation**:
  - `bundled_binary.rs`: show builder selecting the bundled path (and honoring `CODEX_BINARY` override). Native: `CODEX_BINARY=/path/to/bundled codex exec ...`.
  - `codex_home.rs`: set `CODEX_HOME` to an app-scoped dir, emit the path used. Native: `CODEX_HOME=/tmp/my-app codex exec ...`.
- **Streaming/logging**:
  - `stream_events.rs`: consume the typed JSON stream; print turn/item events with minimal formatting; handle idle timeout error.
  - `stream_last_message.rs`: demonstrate `--output-last-message` + `--output-schema` handling, reading files from the API.
  - `stream_with_log.rs`: enable log tee to a file while mirroring stdout selectively.
- **MCP/app-server**:
  - `mcp_codex_tool.rs`: start `codex mcp-server`, issue a `codex` tool call with prompt/cwd/model/sandbox, stream notifications (approvals, task_complete).
  - `mcp_codex_reply.rs`: continue a session via `codex-reply` with a supplied conversation ID; show approval/cancel handling.
  - `app_server_thread_turn.rs`: start app-server, issue `thread/start` then `turn/start`, surface notifications and shutdown.
- **Feature detection/versioning**:
  - `feature_detection.rs`: probe version + feature list, print capability flags, and gate enabling streaming/logging features; include example of emitting an update advisory hook placeholder.

## File/Command Mapping
- README: add sections noted above plus links into EXAMPLES and new example file names.
- `crates/codex/EXAMPLES.md`: reorganize table per categories above; include new rows matching the example commands.
- `crates/codex/examples/*.rs`: add the files listed in “New/Updated Examples to Add”; ensure each example outputs a short explanation of what it is doing and required env vars (`CODEX_BINARY`, `CODEX_HOME`, mock `CONVERSATION_ID` for codex-reply, etc.).
- Cross-link streaming/event types and capability struct docs from README to any rustdoc/module docs added in Workstreams D/E/F.

## Coordination/Assumptions
- Depend on Workstream A for final binary/CODEX_HOME helpers; ensure docs mirror the actual default order (bundled path → env override → explicit builder path).
- Depend on Workstream D for event type names, log tee options, schema file handling; docs should match field names and error variants.
- Depend on Workstream E for MCP/app-server method names and payload shapes; examples should use the real JSON-RPC params once finalized.
- Depend on Workstream F for capability struct shape and advisory hook signature.
- README restoration/creation is needed in H3 since it is absent in this branch; plan assumes reintroducing a top-level README.md.
