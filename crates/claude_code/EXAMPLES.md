# Claude Code Wrapper Examples vs. Native CLI

Every example under `crates/claude_code/examples/` spawns a real `claude` CLI binary (no stubs). The examples are designed to be copy/paste friendly and to map 1:1 to a native CLI invocation.

## Common environment variables

- `CLAUDE_BINARY`: Path to the `claude` binary. If unset, examples fall back to a repo-local `./claude-<target>` when present, else `claude` from `PATH`.
- `CLAUDE_EXAMPLE_ISOLATED_HOME=1`: Runs examples with an isolated `HOME`/`XDG_*` under `target/` to avoid touching your real config.
- `CLAUDE_EXAMPLE_LIVE=1`: Enables examples that may require network/auth (e.g. `print_*`, `setup_token_flow`).
- `CLAUDE_EXAMPLE_ALLOW_MUTATION=1`: Enables examples that may mutate local state (e.g. `update`, `plugin_manage`, `mcp_manage`).
- `CLAUDE_SETUP_TOKEN_CODE`: Optional shortcut for `setup_token_flow` to submit the code without prompting.

## Examples

| Wrapper example | Native command | Notes |
| --- | --- | --- |
| `cargo run -p claude_code --example help_version` | `claude --help` and `claude --version` | Safe, non-auth, non-mutating. |
| `cargo run -p claude_code --example doctor` | `claude doctor` | Safe, non-auth, non-mutating. |
| `cargo run -p claude_code --example print_text -- "hello"` | `claude --print "hello"` | Requires `CLAUDE_EXAMPLE_LIVE=1` (auth/network). |
| `cargo run -p claude_code --example print_stream_json -- "hello"` | `claude --print --output-format stream-json "hello"` | Requires `CLAUDE_EXAMPLE_LIVE=1`; demonstrates parsing `stream-json`. |
| `cargo run -p claude_code --example setup_token_flow` | `claude setup-token` | Requires `CLAUDE_EXAMPLE_LIVE=1`; interactive auth flow; submits code if prompted. |
| `cargo run -p claude_code --example update` | `claude update` | Mutating; requires `CLAUDE_EXAMPLE_ALLOW_MUTATION=1`. |
| `cargo run -p claude_code --example mcp_list` | `claude mcp list` and `claude mcp reset-project-choices` | Safe-ish but can affect local MCP state; see source for behavior. |
| `cargo run -p claude_code --example mcp_manage -- <subcommand>` | `claude mcp ...` | Mutating; requires `CLAUDE_EXAMPLE_ALLOW_MUTATION=1`. Platform support may vary. |
| `cargo run -p claude_code --example plugin_manage -- <subcommand>` | `claude plugin ...` | Mutating; requires `CLAUDE_EXAMPLE_ALLOW_MUTATION=1`. Platform support may vary. |

## Coverage gate

- `crates/claude_code/examples/examples_manifest.json` maps wrapper command paths → example names.
- `crates/claude_code/tests/examples_manifest.rs` enforces that every `CoverageLevel::Explicit` command path (excluding the root path) has at least one example and that the referenced example file exists.

