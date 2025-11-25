# Codex CLI Surface (v0.61) — Commands, Flags, Config Keys

This is a static inventory from `codex --help` and subcommand help, plus known `config.toml` keys (per $CODEX_HOME/config.toml). CLI flags override env/config; `-c/--config` and `--enable/--disable` have highest precedence.

## Commands & Subcommands
- Top-level: `codex` (TUI default), `exec` (alias `e`), `login`, `logout`, `mcp`, `mcp-server`, `app-server`, `completion`, `sandbox` (alias `debug`), `apply` (alias `a`), `resume`, `cloud` (experimental), `features`, `help`.
- `exec` subcommands: `resume`.
- `mcp` subcommands: `list`, `get`, `add`, `remove`, `login`, `logout` (experimental; needs `experimental_use_rmcp_client = true`).
- `app-server` subcommands: `generate-ts`, `generate-json-schema`.
- `cloud` subcommands: `exec`.
- `features` subcommands: `list`.
- `sandbox` subcommands (platform): `macos`, `linux`, `windows` (not shown in `--help` but present in code/docs).

## Positional Args
- Top-level: `[PROMPT]` (interactive/TUI).
- `exec`: `[PROMPT] [COMMAND]` (COMMAND used for `resume`).
- `resume`: `[SESSION_ID] [PROMPT]` (UUID optional; `--last` to pick most recent).

## Shared Flags (most commands)
- `-c, --config <key=value>`: TOML override; dotted paths allowed; parsed as TOML else literal.
- `--enable <FEATURE>` / `--disable <FEATURE>`: feature toggles (equiv to `-c features.<name>=true/false`).
- `-m, --model <MODEL>`; `--oss`; `--local-provider <lmstudio|ollama>`.
- `-p, --profile <CONFIG_PROFILE>`.
- `-s, --sandbox <read-only|workspace-write|danger-full-access>`.
- `-a, --ask-for-approval <untrusted|on-failure|on-request|never>`.
- `--full-auto` (sets sandbox workspace-write + approval on-request).
- `--dangerously-bypass-approvals-and-sandbox`.
- `-C, --cd <DIR>`; `--add-dir <DIR>`.
- `-i, --image <FILE>...`; `--search`.
- `-h, --help`; `-V, --version`.

## Exec-Specific Flags
- `--skip-git-repo-check`.
- `--output-schema <FILE>`.
- `--color <always|never|auto>` (default auto).
- `--json`.
- `-o, --output-last-message <FILE>`.

## Resume Flags
- `--last`, `--all`, plus shared flags.

## MCP / App-Server / Cloud / Features / Sandbox
- `mcp`: subcommands list/get/add/remove/login/logout (experimental). Only shared config flags.
- `mcp-server`: stdio MCP server; only shared config flags.
- `app-server`: subcommands generate-ts / generate-json-schema; only shared config flags.
- `cloud`: subcommand exec; only shared config flags.
- `features`: subcommand list; only shared config flags.
- `sandbox`: platform subcommands macos/linux/windows (in code, not shown in `--help`).

## Config Keys (in $CODEX_HOME/config.toml)
- Model/behavior: `model`, `review_model`, `model_provider`, `model_context_window`, `model_max_output_tokens`, `model_auto_compact_token_limit`.
- Reasoning/verbosity: `model_reasoning_effort` (minimal|low|medium|high), `model_reasoning_summary` (auto|concise|detailed|none), `model_verbosity` (low|medium|high), `model_supports_reasoning_summaries`, `model_reasoning_summary_format` (none|experimental).
- Approval/sandbox: `approval_policy` (untrusted|on-failure|on-request|never), `sandbox_mode` (read-only|workspace-write|danger-full-access), `sandbox_workspace_write.writable_roots` (array), `sandbox_workspace_write.network_access` (bool), `sandbox_workspace_write.exclude_tmpdir_env_var` (bool), `sandbox_workspace_write.exclude_slash_tmp` (bool).
- Shell env policy: `[shell_environment_policy] inherit = core|all|none`, `exclude` (globs), `set` (map), `include_only` (whitelist).
- MCP servers: `[mcp_servers.<id>]` with `command`, `args`, `env`, `env_vars`, `cwd`, `enabled`, `startup_timeout_sec`, `tool_timeout_sec`, `enabled_tools`, `disabled_tools`; streamable_http transport adds `url`, `bearer_token_env_var`, `http_headers`, `env_http_headers`.
- Credential stores: `cli_auth_credentials_store` (File|Keyring|Auto); `mcp_oauth_credentials_store_mode` similar.
- History/logs: `history.persistence`; files under CODEX_HOME: `config.toml`, `auth.json`, `.credentials.json`, `history.jsonl`, `conversations/`, `logs/`; optional `notify` hook program on turn-complete.
- Features: `[features.<name>]` feature toggles (also set via CLI enable/disable).
- Profiles: `[profiles.<name>]` tables for presets.
- Misc: `model_providers` map entries; search toggles (if present); review toggles.

## Notes / Gaps vs Wrapper
- CLI parity flags/config are exposed on the builder and per-request patches: config overrides (including reasoning/verbosity), approval/sandbox/full-auto/danger-bypass, `cd`, `local-provider`, `search`, and resume selectors. GPT-5* reasoning defaults stay opt-out via overrides or `auto_reasoning_defaults(false)`.
- CODEX_HOME is now supported in the wrapper via builder (`codex_home`, `create_home_dirs`); env is applied per spawn with `CODEX_BINARY` and default `RUST_LOG`.
- Auth/session remains basic (login/status/logout only).
- Tests primarily live in inline unit tests (lib.rs, mcp.rs) and examples/doc-tests; no end-to-end coverage with a real CLI binary yet.
