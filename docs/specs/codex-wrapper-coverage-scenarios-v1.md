# Codex Wrapper Coverage Scenario Catalog (v1)

Status: **Normative** (paired with the generator contract)  
Scope: required scenario set for `wrapper_coverage_manifest()` derivation

## Normative language

This document uses RFC 2119-style requirement keywords (`MUST`, `MUST NOT`).

This document enumerates the **complete v1 scenario set** the wrapper coverage generator must use to derive `cli_manifests/codex/wrapper_coverage.json` from `crates/codex` implementation signals.

The intent is to remove ambiguity about:
- which wrapper APIs MUST be reflected,
- which command paths MUST appear,
- which flags/args MUST be claimed (and at what level),
- which invocations MUST be considered global.

If the wrapper adds a new public API that spawns `codex`, this catalog MUST be updated (as part of the same change) to include a new scenario or extend an existing one.

## Conventions

- **Command path** is shown as tokens, e.g. `["features","list"]`.
- **Flag key** is the canonical string emitted in argv, e.g. `--profile`.
- **Arg name** is the upstream help-derived positional name, e.g. `PROMPT`. The generator MUST emit positional args only for arg names listed in this catalog.
- Global override flags are recorded on the `path=[]` entry and reflect wrapper support for upstream-global surfaces.

## Exactness (v1, normative)

- For every command path `P` listed in this catalog, the generator MUST emit exactly one `coverage[]` entry with `level: "explicit"` for `P`.
- Multiple scenarios reference the same command path `P` (e.g., Scenario 1 and Scenario 2 both reference `["exec"]`). For each `P`, the emitted flags/args MUST equal the union of all flags/args listed across every scenario section that references `P`.
- Global flags are emitted only under `path=[]` (Scenario 0). They MUST NOT be duplicated under other command paths.
- For each command path `P`, the generator MUST omit any flag key or arg name not listed for `P` by this catalog.

## Scenario 0: Wrapper-global CLI overrides (root entry)

The generator MUST emit a `coverage[]` entry for `path=[]` (root) with `level: "explicit"` containing the root/global flags supported by the wrapper (both override flags and probe flags).

### Required root flags

The generator MUST include the following flags under `path=[]`:

- `--help` (level: `explicit`)
- `--version` (level: `explicit`)
- `--model` (level: `explicit`)
- `--image` (level: `explicit`)
- `--add-dir` (level: `explicit`, note: `capability-guarded`)
- `--config` (level: `passthrough`)  
  Rationale: wrapper forwards `key=value` overrides but does not type individual upstream config keys.
- `--enable` (level: `passthrough`)
- `--disable` (level: `passthrough`)
- `--profile` (level: `explicit`)
- `--cd` (level: `explicit`)
- `--ask-for-approval` (level: `explicit`)
- `--sandbox` (level: `explicit`)
- `--full-auto` (level: `explicit`)
- `--dangerously-bypass-approvals-and-sandbox` (level: `explicit`)
- `--local-provider` (level: `explicit`)
- `--oss` (level: `explicit`)
- `--search` (level: `explicit`)

Notes:
- `passthrough` is reserved in v1 for stringly/generic forwarding (currently: `--config`, `--enable`, `--disable`).
- Always-on wrapper defaults (e.g., `--skip-git-repo-check`) MUST be recorded in the specific command scenario where they are emitted.

## Scenario 1: `codex exec` (single-response)

Wrapper API family:
- `CodexClient::send_prompt` / `CodexClient::send_prompt_with`

### Command entry

- Path: `["exec"]` (level: `explicit`)

### Required command-specific flags

- `--color` (level: `explicit`)  
  The wrapper always passes `--color <MODE>` for `exec` invocations.
- `--skip-git-repo-check` (level: `explicit`)  
  The wrapper always passes this flag for `exec` invocations.
- `--output-schema` (level: `explicit`, note: `capability-guarded`)  
  The wrapper supports this flag but emits it only if runtime capability probes indicate support.

### Required positional args

- Arg: `PROMPT` (level: `explicit`)  
  The wrapper requires a prompt string and forwards it either as a positional argument or via stdin depending on JSON mode; v1 claims parity for the `PROMPT` unit.

## Scenario 2: `codex exec --json` (streaming)

Wrapper API family:
- `CodexClient::stream_exec` / `CodexClient::stream_exec_with_overrides`

### Command entry

- Path: `["exec"]` (level: `explicit`)  
  This scenario merges with Scenario 1 for the same path; the strongest level wins.

### Required command-specific flags (additive)

- `--json` (level: `explicit`)
- `--output-last-message` (level: `explicit`)
- `--output-schema` (level: `explicit`, note: `capability-guarded`)  
  Streaming uses the valued form `--output-schema <PATH>`.

### Required positional args

- Arg: `PROMPT` (level: `explicit`)

## Scenario 3: `codex exec --json resume` (streaming resume)

Wrapper API family:
- `CodexClient::stream_resume`

### Command entry

- Path: `["exec","resume"]` (level: `explicit`)

### Required command-specific flags

- `--json` (level: `explicit`)  
  The wrapper requests JSONL output by passing `--json` to the parent `codex exec` invocation.
- `--skip-git-repo-check` (level: `explicit`)
- `--last` (level: `explicit`)
- `--all` (level: `explicit`)
Notes:
- `--color`, `--output-last-message`, and `--output-schema` are recorded under `path=["exec"]` (Scenario 2) because the wrapper supplies them as part of the parent `codex exec --json` invocation.
- `--model` and `--add-dir` are recorded at `path=[]` as global flags (Scenario 0).

### Required positional args

- Arg: `PROMPT` (level: `explicit`)  
  This positional arg is represented in wrapper request types and is forwarded when present.
- Arg: `SESSION_ID` (level: `explicit`)  
  Emitted only for `ResumeSelector::Id(...)`.

## Scenario 4: `codex apply <TASK_ID>` and `codex cloud diff <TASK_ID>`

Wrapper API family:
- `CodexClient::apply`
- `CodexClient::apply_task`
- `CodexClient::diff`
- `CodexClient::cloud_diff_task`

### Command entries

- Path: `["apply"]` (level: `explicit`)
- Path: `["cloud","diff"]` (level: `explicit`)

Notes:
- `CodexClient::apply` and `CodexClient::diff` may read `CODEX_TASK_ID` as a convenience when callers do not supply a task id explicitly.

### Required positional args

- For `path=["apply"]`: `TASK_ID` (level: `explicit`)
- For `path=["cloud","diff"]`: `TASK_ID` (level: `explicit`)

## Scenario 5: `codex login`, `codex login status`, `codex logout`

Wrapper API family:
- `CodexClient::spawn_login_process` (login interactive)
- `CodexClient::spawn_mcp_login_process` (login with MCP integration)
- `CodexClient::login_with_api_key`
- `CodexClient::login_status`
- `CodexClient::logout`

### Command entries

- Path: `["login"]` (level: `explicit`)
- Path: `["login","status"]` (level: `explicit`)
- Path: `["logout"]` (level: `explicit`)

### Flags/args

The generator MUST emit the following flags under `path=["login"]`:

- `--mcp` (level: `explicit`, note: `capability-guarded`)
- `--api-key` (level: `explicit`)

The generator MUST NOT emit any flags or args under:
- `path=["login","status"]`
- `path=["logout"]`

## Scenario 6: `codex features list`

Wrapper API family:
- `CodexClient::list_features`

### Command entry

- Path: `["features","list"]` (level: `explicit`)

### Required command-specific flags

- `--json` (level: `explicit`)

## Scenario 7: `codex app-server generate-ts` / `generate-json-schema`

Wrapper API family:
- `CodexClient::generate_app_server_bindings`

### Command entries

- Path: `["app-server","generate-ts"]` (level: `explicit`)
- Path: `["app-server","generate-json-schema"]` (level: `explicit`)

### Required command-specific flags

- `--out` (level: `explicit`)
- `--prettier` (level: `explicit`) only under `path=["app-server","generate-ts"]`

## Scenario 8: `codex responses-api-proxy`

Wrapper API family:
- `CodexClient::start_responses_api_proxy`

### Command entry

- Path: `["responses-api-proxy"]` (level: `explicit`)

### Flags/args

The generator MUST emit the following flags under `path=["responses-api-proxy"]`:

- `--port` (level: `explicit`)
- `--server-info` (level: `explicit`)
- `--http-shutdown` (level: `explicit`)
- `--upstream-url` (level: `explicit`)

The generator MUST NOT emit any positional args for `path=["responses-api-proxy"]`. The API key is supplied via stdin and is not represented as a help-surface positional arg in v1.

## Scenario 9: `codex stdio-to-uds`

Wrapper API family:
- `CodexClient::stdio_to_uds`

### Command entry

- Path: `["stdio-to-uds"]` (level: `explicit`)

### Required positional args

- Arg: `SOCKET_PATH` (level: `explicit`)  
  This is a wrapper-chosen identity for a wrapper-only surface in v1.

## Scenario 10: `codex sandbox <platform>`

Wrapper API family:
- `CodexClient::run_sandbox`

### Command entries

- Path: `["sandbox","macos"]` (level: `explicit`)
- Path: `["sandbox","linux"]` (level: `explicit`)
- Path: `["sandbox","windows"]` (level: `explicit`)

### Required command-specific flags

The generator MUST emit the following sandbox-specific flag:

- `--log-denials` (level: `explicit`) only under `path=["sandbox","macos"]`

### Required positional args

- Arg: `COMMAND` (level: `explicit`)  
  Represents the trailing command vector (passed after `--`).

## Scenario 11: `codex execpolicy check`

Wrapper API family:
- `CodexClient::check_execpolicy`

### Command entry

- Path: `["execpolicy","check"]` (level: `explicit`)

### Required command-specific flags

- `--policy` (level: `explicit`)
- `--pretty` (level: `explicit`)

### Required positional args

- Arg: `COMMAND` (level: `explicit`)  
  Represents the trailing command vector (passed after `--`).

## Scenario 12: `codex mcp-server` and server-mode `codex app-server`

Wrapper API family:
- `codex::mcp` server spawns (stdio JSON-RPC transports)

### Command entries

- Path: `["mcp-server"]` (level: `explicit`)
- Path: `["app-server"]` (level: `explicit`)

Notes:
- If upstream snapshots do not include these paths for a given version, reports will include them as `wrapper_only_commands`.
- If upstream snapshots include these paths for a given version, report comparison will align by identity automatically.
