# Claude Code Wrapper Examples vs. Native CLI

Every example under `crates/claude_code/examples/` spawns a real `claude` CLI binary (no stubs). The examples are designed to be copy/paste friendly and to map 1:1 to a native CLI invocation.

## Common environment variables

- `CLAUDE_BINARY`: Path to the `claude` binary. If unset, examples fall back to a repo-local `./claude-<target>` when present, else `claude` from `PATH`.
- `CLAUDE_EXAMPLE_ISOLATED_HOME=1`: Runs examples with an isolated `HOME`/`XDG_*` under `target/` to avoid touching your real config.
- `CLAUDE_EXAMPLE_LIVE=1`: Enables examples that may require network/auth (e.g. `print_*`, `setup_token_flow`).
- `CLAUDE_EXAMPLE_ALLOW_MUTATION=1`: Enables examples that may mutate local state (e.g. `update`, `plugin_manage`, `mcp_manage`).
- `CLAUDE_SETUP_TOKEN_CODE`: Optional shortcut for `setup_token_flow` to submit the code without prompting.

## Basics

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p claude_code --example help_version` | `claude --help` and `claude --version` | Safe, non-auth, non-mutating. |
| `cargo run -p claude_code --example doctor` | `claude doctor` | Safe, non-auth, non-mutating. |
| `cargo run -p claude_code --example print_text -- "hello"` | `claude --print "hello"` | Requires `CLAUDE_EXAMPLE_LIVE=1` (auth/network). |
| `cargo run -p claude_code --example print_json -- "hello"` | `claude --print --output-format json "hello"` | Requires `CLAUDE_EXAMPLE_LIVE=1`; prints prettified JSON. |
| `cargo run -p claude_code --example print_stream_json -- "hello"` | `claude --print --output-format stream-json "hello"` | Requires `CLAUDE_EXAMPLE_LIVE=1`; demonstrates parsing `stream-json`. |

## Stream-JSON

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p claude_code --example print_session_id -- "hello"` | `claude --print --output-format stream-json "hello"` | Requires `CLAUDE_EXAMPLE_LIVE=1`; prints the discovered `session_id`. |
| `cargo run -p claude_code --example print_include_partial_messages -- "hello"` | `claude --print --output-format stream-json --include-partial-messages "hello"` | Requires `CLAUDE_EXAMPLE_LIVE=1`; prints a type-count summary. |

## Multi-turn & sessions

These examples are intentionally run inside a temp working directory so session persistence doesn’t touch your repo checkout.

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p claude_code --example multi_turn_resume` | `claude --print --session-id <uuid> ...` then `claude --print --resume <uuid> ...` | Requires `CLAUDE_EXAMPLE_LIVE=1`; demonstrates 2 turns via explicit session ID then resume. |
| `cargo run -p claude_code --example multi_turn_fork` | `claude --print --resume <uuid> --fork-session ...` | Requires `CLAUDE_EXAMPLE_LIVE=1`; best-effort check that a new session is created. |
| `cargo run -p claude_code --example multi_turn_continue` | `claude --print ...` then `claude --print --continue ...` | Requires `CLAUDE_EXAMPLE_LIVE=1`; continues most recent session in the working dir. |

## Auth & setup-token

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p claude_code --example setup_token_flow` | `claude setup-token` | Requires `CLAUDE_EXAMPLE_LIVE=1`; interactive auth flow; submits code if prompted. |

## MCP / plugins / update (mutation-gated)

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p claude_code --example update` | `claude update` | Mutating; requires `CLAUDE_EXAMPLE_ALLOW_MUTATION=1`. |
| `cargo run -p claude_code --example mcp_list` | `claude mcp list` and `claude mcp reset-project-choices` | Safe-ish but can affect local MCP state; see source for behavior. |
| `cargo run -p claude_code --example mcp_manage -- <subcommand>` | `claude mcp ...` | Mutating; requires `CLAUDE_EXAMPLE_ALLOW_MUTATION=1`. Platform support may vary. |
| `cargo run -p claude_code --example plugin_manage -- <subcommand>` | `claude plugin ...` | Mutating; requires `CLAUDE_EXAMPLE_ALLOW_MUTATION=1`. Platform support may vary. |

## Drift prevention (coverage gates)

- Command coverage: `crates/claude_code/examples/examples_manifest.json` and `crates/claude_code/tests/examples_manifest.rs`.
  - Ensures every `CoverageLevel::Explicit` command path (excluding root) has at least one example.
- Print-flow coverage: `crates/claude_code/examples/print_flows_manifest.json` and `crates/claude_code/tests/print_flows_manifest.rs`.
  - Ensures multi-turn/session + stream-json flows keep examples as wrapper capabilities evolve.
