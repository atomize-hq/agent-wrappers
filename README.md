# Codex Rust Wrapper

Async helper around the OpenAI Codex CLI for programmatic prompting, streaming, apply/diff helpers, and server flows. The crate shells out to `codex`, applies safe defaults (temp working dirs, timeouts, quiet stderr mirroring), supports bundled binaries, and offers capability-aware streaming plus typed MCP/app-server helpers.

## Getting Started
- Add the dependency:  
  ```toml
  [dependencies]
  codex = { path = "crates/codex" }
  ```
- Minimal prompt:
  ```rust
  use codex::CodexClient;

  # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
  let client = CodexClient::builder().build();
  let reply = client.send_prompt("List rustfmt defaults").await?;
  println!("{reply}");
  # Ok(()) }
  ```
- Default binary resolution: `CODEX_BINARY` if set, otherwise `codex` on `PATH`. Embedded apps should resolve `<bundle_root>/<platform>/<version>/codex` via `resolve_bundled_binary(...)` and pass the returned path to `.binary(...)` (see `crates/codex/examples/bundled_binary.rs`).

## Bundled Binary & `CODEX_HOME`
- Defaults stay unchanged: `CODEX_BINARY` override, otherwise `codex` on `PATH`. For embedded apps, call `resolve_bundled_binary(BundledBinarySpec { bundle_root, version, platform: None })` to locate a pinned `<bundle_root>/<platform>/<version>/codex` without falling back to the user install, then pass it to `.binary(...)`. Hosts own bundle downloads/version pins.
- Derive a per-project `CODEX_HOME` (e.g. `~/.myapp/codex-homes/<project-slug>`) and set it via `.codex_home(...)` with `.create_home_dirs(true)`. `CodexHomeLayout` surfaces config/auth/history/conversation/log paths so each workspace stays isolated.
- To reuse credentials safely, copy only `auth.json` and `.credentials.json` from a trusted seed home into the project `CODEX_HOME` before spawning Codex; avoid copying history/logs. `AuthSessionHelper` runs login/status helpers under the isolated home without mutating the parent env.
- Quick bundled flow (values shown inline for clarity; wire them to your app config/env):
  ```rust,no_run
  use codex::{resolve_bundled_binary, AuthSessionHelper, BundledBinarySpec, CodexClient, CodexHomeLayout};

  # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
  let bundled = resolve_bundled_binary(BundledBinarySpec {
      bundle_root: "/apps/myapp/codex-bin".as_ref(),
      version: "1.2.3",
      platform: None, // defaults to the current target
  })?;

  let home = CodexHomeLayout::new("/apps/myapp/codex-homes/demo");
  home.materialize(true)?;

  let client = CodexClient::builder()
      .binary(&bundled.binary_path)
      .codex_home(home.root())
      .create_home_dirs(true)
      .build();

  let _ = AuthSessionHelper::with_client(client.clone()).status().await?;
  let reply = client.send_prompt("Health check").await?;
  println!("{reply}");
  # Ok(()) }
  ```
  See `crates/codex/examples/bundled_binary_home.rs` for a runnable flow with optional auth seeding and `AuthSessionHelper` login.

## Exec API & Safety Defaults
- `send_prompt` shells out to `codex exec --skip-git-repo-check` with:
  - temp working directory per call unless `working_dir` is set
  - 120s timeout (use `.timeout(Duration::ZERO)` to disable)
  - ANSI colors off by default (`ColorMode::Never`)
  - mirrors stdout by default; set `.mirror_stdout(false)` when parsing JSON
  - `RUST_LOG=error` if unset to keep the console quiet
  - model-specific reasoning config for `gpt-5*`/`gpt-5.1*` defaults to **medium** effort to avoid unsupported “minimal” errors on current models
- Other builder flags: `.model("gpt-5-codex")`, `.image("/path/mock.png")`, `.json(true)` (pipes prompt via stdin), `.quiet(true)`.
- `ExecStreamRequest` supports `idle_timeout` (fails fast on silent streams), `output_last_message`/`output_schema` for artifacts, and `json_event_log` to tee raw JSONL before parsing.
- Example `crates/codex/examples/send_prompt.rs` covers the baseline; `working_dir(_json).rs`, `timeout*.rs`, `image_json.rs`, `color_always.rs`, `quiet.rs`, and `no_stdout_mirror.rs` expand on inputs and output handling.

## CLI Parity Overrides
- Builder methods mirror CLI flags and config overrides: `.config_override(_raw|s)`, `.reasoning_*`, `.approval_policy(...)`, `.sandbox_mode(...)`, `.full_auto(true)`, `.dangerously_bypass_approvals_and_sandbox(true)`, `.profile(...)`, `.cd(...)`, `.local_provider(...)`, `.oss(true)`, `.enable_feature(...)`, `.disable_feature(...)`, `.search(...)`, `.auto_reasoning_defaults(false)`. Config overrides and feature toggles carry across exec/resume/apply/diff; per-request patches win on conflict (including `--oss`).
- Per-call overlays use `ExecRequest`/`ResumeRequest`: add config overrides, toggle search/oss/feature flags, swap `cd`/`profile`, or change safety policy for a single run. Resume supports `.last()`/`.all()` selectors matching `--last`/`--all`.
- GPT-5* reasoning defaults stay enabled unless you set reasoning/config overrides or flip `auto_reasoning_defaults(false)` on the builder or request.

```rust,no_run
use codex::{ApprovalPolicy, CodexClient, ExecRequest, LocalProvider, SandboxMode};

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let client = CodexClient::builder()
    .approval_policy(ApprovalPolicy::OnRequest)
    .sandbox_mode(SandboxMode::WorkspaceWrite)
    .profile("staging")
    .local_provider(LocalProvider::Ollama)
    .config_override("model_verbosity", "high")
    .search(true)
    .build();

let request = ExecRequest::new("Draft release notes")
    .config_override("model_reasoning_effort", "low")
    .search(false);
let reply = client.send_prompt_with(request).await?;
println!("{reply}");
# Ok(()) }
```
- See `crates/codex/examples/cli_overrides.rs` for a runnable parity example.

## Streaming Output & Artifacts
- Event schema: JSONL lines carry `type` plus thread/turn IDs and status. Expect `thread.started` (or `thread.resumed` when continuing a run), `turn.started/turn.completed/turn.failed`, and `item.created/item.updated` where `item.type` can be `agent_message`, `reasoning`, `command_execution`, `file_change`, `mcp_tool_call`, `web_search`, or `todo_list` with optional `status`/`content`/`input`. Errors surface as `{"type":"error","message":...}`. Examples ship `--sample` payloads so you can inspect shapes without a binary.
- Sample streaming/resume/apply payloads live under `crates/codex/examples/fixtures/*` and power the `--sample` flags in examples; refresh them whenever the CLI JSON surface changes so docs stay aligned.
- Enable JSONL streaming with `.json(true)` or by invoking the CLI directly. The crate returns captured output; use the examples to consume the stream yourself:
  - `crates/codex/examples/stream_events.rs`: typed consumer for `thread/turn/item` events (success + failure), uses `--timeout 0` to keep streaming, includes idle timeout handling, and a `--sample` replay path.
  - `crates/codex/examples/stream_last_message.rs`: runs `--output-last-message` + `--output-schema`, reads the emitted files, and ships sample payloads if the binary is missing.
  - `crates/codex/examples/stream_with_log.rs`: mirrors JSON events to stdout and tees them to `CODEX_LOG_PATH` (default `codex-stream.log`); also supports `--sample` and can defer to the binary's built-in log tee feature when advertised via `codex features list`.
  - `crates/codex/examples/json_stream.rs`: simplest `--json` usage when you just want the raw stream buffered.
- Artifacts: Codex can persist the final assistant message and the output schema alongside streaming output; point them at writable locations per the `stream_last_message` example. Apply/diff flows also surface stdout/stderr/exit (see below) so you can log or mirror them alongside the JSON stream.
- Apply/diff output also arrives inside `file_change` events and any configured `json_event_log` tee when you stream.

## Resume, Diff, and Apply
- Resume an existing conversation with `codex resume --json --skip-git-repo-check --last` (or `--id <conversationId>`/`CODEX_CONVERSATION_ID`). Expect `thread.resumed` followed by the usual `thread/turn/item` stream plus `turn.failed` on idle timeouts; reuse the streaming examples to consume it.
- Preview a patch before applying it: `codex diff --json --skip-git-repo-check` emits the staged diff while preserving JSON-safe output. Pair it with `codex apply --json` to capture stdout, stderr, and the exit code for the apply step (`{"type":"apply.result","exit_code":0,"stdout":"...","stderr":""}`).
- Approvals and cancellations surface as events in MCP/app-server flows; see the server examples for approval-required hooks around apply.
- Example `crates/codex/examples/resume_apply.rs` covers the full flow with `--sample` payloads (resume stream, diff preview, apply result) and lets you skip the apply step with `--no-apply`.

## Sandbox Command
- `run_sandbox` wraps `codex sandbox <macos|linux|windows>` and returns stdout/stderr + the inner command status (non-zero statuses are not converted into errors). `mirror_stdout`/`quiet` from the builder control console mirroring.
- Flags: `full_auto(true)` maps to `--full-auto`, `log_denials(true)` maps to the macOS-only `--log-denials`, and request `config_overrides`/`feature_toggles` become `--config/--enable/--disable`. Other CLI overrides (approval/search/profile/sandbox) are intentionally not forwarded on this subcommand.
- Working dir precedence: request `.working_dir(...)` → builder `.working_dir(...)` → current process dir (no temp dirs).
- Platform notes: macOS is the only platform that emits denial logs; Linux relies on the bundled `codex-linux-sandbox` helper; Windows sandboxing is experimental and requires the upstream helper (the wrapper does not gate support—non-zero exits surface in `SandboxRun::status`). There is no built-in post-run hook; run any follow-up script after awaiting `run_sandbox`.
- Example:
  ```rust,no_run
  use codex::{CodexClient, SandboxCommandRequest, SandboxPlatform};

  # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
  let client = CodexClient::builder().mirror_stdout(false).quiet(true).build();
  let run = client
      .run_sandbox(
          SandboxCommandRequest::new(
              SandboxPlatform::Linux,
              ["bash", "-lc", "ls -la /tmp"],
          )
          .full_auto(true)
          .config_override("features.experimental_sandbox", "true"),
      )
      .await?;
  println!("exit: {:?}", run.status.code());
  println!("stdout:\n{}", run.stdout);
  # Ok(()) }
  ```
  See `crates/codex/examples/run_sandbox.rs` for a runnable wrapper that selects the platform, forwards `--full-auto`/`--log-denials`, and prints captured stdout/stderr/exit.

## Execpolicy Checks
- `check_execpolicy` wraps `codex execpolicy check --policy <PATH>... [--pretty] -- <COMMAND...>` and returns captured stdout/stderr/status plus parsed JSON (`match` with `decision`/`rules` or `noMatch`).
- Decisions map to `allow`/`prompt`/`forbidden` (forbidden > prompt > allow); rule-level decisions default to `allow` when omitted.
- Request helpers: `.policy(...)`/`.policies(...)` push repeatable `--policy` flags, `.pretty(true)` forwards `--pretty`, and `.config_override/_raw` + `.profile` + `.search` layer request overrides on top of builder config/profile/approval/sandbox/local-provider/cd settings.
- Empty command argv returns `EmptyExecPolicyCommand`; non-zero CLI exits surface as `CodexError::NonZeroExit` with stderr attached.
- Example:
  ```rust,no_run
  use codex::{CodexClient, ExecPolicyCheckRequest, ExecPolicyDecision};

  # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
  let client = CodexClient::builder().mirror_stdout(false).quiet(true).build();
  let check = client
      .check_execpolicy(
          ExecPolicyCheckRequest::new(["bash", "-lc", "rm -rf /tmp/scratch"])
              .policies(["./policies/default.codexpolicy"])
              .pretty(true),
      )
      .await?;

  match check.decision() {
      Some(ExecPolicyDecision::Forbidden) => eprintln!("blocked by policy"),
      Some(ExecPolicyDecision::Prompt) => eprintln!("requires approval"),
      Some(ExecPolicyDecision::Allow) => println!("allowed"),
      None => println!("no policy matched"),
  }
  # Ok(()) }
  ```

## Responses API Proxy
- `start_responses_api_proxy` wraps `codex responses-api-proxy`, writes the API key to stdin, and forwards `--port`/`--server-info`/`--http-shutdown`/`--upstream-url` as requested.
- The returned handle exposes the child process (kill-on-drop) plus any `--server-info` path, and `read_server_info` parses `{port,pid}` when the file is present. Stdout/stderr remain piped; drain them if you want to tail logs.
- Example:
  ```rust,no_run
  use codex::{CodexClient, ResponsesApiProxyRequest};

  # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
  let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "sk-placeholder".to_string());
  let server_info = "/tmp/responses-proxy.json";
  let mut proxy = CodexClient::builder()
      .mirror_stdout(false)
      .quiet(true)
      .build()
      .start_responses_api_proxy(
          ResponsesApiProxyRequest::new(api_key)
              .port(8081)
              .http_shutdown(true)
              .server_info(server_info),
      )
      .await?;

  if let Some(info) = proxy.read_server_info().await? {
      println!("proxy listening on http://127.0.0.1:{}", info.port);
  }
  let _ = proxy.child.start_kill();
  let _ = proxy.child.wait().await?;
  # Ok(()) }
  ```
  See `crates/codex/examples/responses_api_proxy.rs` for a runnable smoke check.

## Stdio-to-UDS Bridge
- `stdio_to_uds` wraps `codex stdio-to-uds <SOCKET_PATH>` with piped stdin/stdout/stderr so you can bridge JSON-RPC streams over a Unix domain socket. The working dir comes from the request override, then the builder, then the current process dir.
- Keep stdout/stderr drained to avoid backpressure if the relay emits logs.
- Example (assumes a listening UDS at `socket_path`):
  ```rust,no_run
  use codex::{CodexClient, StdioToUdsRequest};
  use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

  # async fn demo(socket_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
  let mut bridge = CodexClient::builder()
      .mirror_stdout(false)
      .quiet(true)
      .build()
      .stdio_to_uds(StdioToUdsRequest::new(socket_path))?;

  let mut stdin = bridge.stdin.take().unwrap();
  let mut stdout = BufReader::new(bridge.stdout.take().unwrap());

  stdin.write_all(b"ping\n").await?;
  stdin.shutdown().await?;

  let mut line = String::new();
  stdout.read_line(&mut line).await?;
  println!("echoed: {line}");

  let _ = bridge.wait().await?;
  # Ok(()) }
  ```
  See `crates/codex/examples/stdio_to_uds.rs` for a Unix smoke relay against a local echo server.

## App-Server Codegen
- `generate_app_server_bindings` wraps `codex app-server generate-ts` (optional `--prettier`) and `generate-json-schema`, creates the output directory when missing, and returns captured stdout/stderr plus the exit status; non-zero exits surface as `CodexError::NonZeroExit` with stderr attached. Shared config/profile/search/approval flags flow through via builder/request overrides.
- Example:
  ```rust,no_run
  use codex::{AppServerCodegenRequest, CodexClient};

  # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
  let client = CodexClient::builder()
      .mirror_stdout(false)
      .quiet(true)
      .profile("dev")
      .build();
  let codegen = client
      .generate_app_server_bindings(
          AppServerCodegenRequest::typescript("./gen/app")
              .prettier("./node_modules/.bin/prettier")
              .config_override("features.codegen", "true"),
      )
      .await?;
  println!("app-server exit: {:?}", codegen.status.code());
  println!("bindings dir: {}", codegen.out_dir.display());
  # Ok(()) }
  ```

## Features List
- `list_features` wraps `codex features list` with optional `--json` (opt-in via the request), falling back to parsing the text table when JSON is unavailable. Returns parsed entries (name, stage, enabled) alongside captured stdout/stderr/status and indicates which format was parsed.
- Shared config/profile/search/approval overrides flow through; the enabled column reflects the effective config/profile. Non-zero exits surface as `CodexError::NonZeroExit`, and unparsable output surfaces as `CodexError::FeatureListParse`.
- Example:
  ```rust,no_run
  use codex::{CodexClient, FeaturesListRequest};

  # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
  let client = CodexClient::builder()
      .mirror_stdout(false)
      .quiet(true)
      .build();
  let features = client
      .list_features(
          FeaturesListRequest::new()
              .json(true)
              .profile("beta")
              .config_override("features.experimental_sandbox", "true"),
      )
      .await?;
  println!("features parsed from {:?}", features.format);
  for feature in features.features {
      println!("{} ({:?}) enabled={}", feature.name, feature.stage, feature.enabled);
  }
  # Ok(()) }
  ```

## MCP + App-Server Flows
- The CLI ships stdio servers for Model Context Protocol and the app-server APIs. Examples cover the JSON-RPC wiring, approvals, and shutdown:
- `crates/codex/examples/mcp_codex_flow.rs`: start `codex mcp-server --stdio`, call `tools/codex`, and follow up with `codex/codex-reply` when a conversation ID is returned (supports `--sample`).
- `crates/codex/examples/mcp_codex_tool.rs`: start `codex mcp-server`, call `tools/codex` with prompt/cwd/model/sandbox, and watch `approval_required`/`task_complete` notifications (includes `turn_id`/`sandbox` and supports `--sample`).
- `crates/codex/examples/mcp_codex_reply.rs`: resume a session via `tools/codex-reply`, taking `CODEX_CONVERSATION_ID` or a CLI arg; supports `--sample`.
- `crates/codex/examples/app_server_turns.rs`: start/resume threads, stream items/task_complete, and optionally send `turn/interrupt`.
- `crates/codex/examples/app_server_thread_turn.rs`: launch `codex app-server`, send `thread/start` then `turn/start`, and stream task notifications (thread/turn IDs echoed; `--sample` supported).
- Pass `CODEX_HOME` for isolated server state and `CODEX_BINARY` (or `.binary(...)`) to pin the binary version used by the servers.

## Feature Detection & Version Hooks
- `crates/codex/examples/feature_detection.rs` shows how to:
  - parse `codex --version`
  - list features via `codex features list` (if supported) and cache them per binary path so repeated probes avoid extra processes
  - gate optional knobs like JSON streaming, log tee, MCP/app-server endpoints, resume/apply/diff flags, and artifact flags
  - emit an upgrade advisory hook when required capabilities are missing
- Use this when deciding whether to enable `--json`, log tee paths, resume/apply helpers, or app-server endpoints in your app UI. Always gate new feature names against `codex features list` so drift in the binary's output is handled gracefully.

## Upgrade Advisories & Gaps
- Sample streams, resume/apply payloads, and feature names reflect the current CLI surface but are not validated against a live binary here; gate risky flags behind capability checks and prefer `--sample` payloads while developing.
- The crate still buffers stdout/stderr from streaming/apply flows instead of exposing a typed stream API; use the examples to consume JSONL incrementally until a typed interface lands.
- Apply/diff flows depend on Codex emitting JSON-friendly stdout/stderr; handle non-JSON output defensively in host apps.
- Capability detection caches are keyed to a binary path/version pairing; refresh them whenever the Codex binary path, mtime, or `--version` output changes instead of reusing stale results across upgrades. Treat `codex features list` output as best-effort hints that may drift across releases and fall back to the fixtures above when probing fails.
- Top-level `--oss` and `--enable/--disable` toggles now flow through builder/request helpers; feature toggles are additive across builder/request, and request overrides can disable a builder-supplied `--oss`.
- The wrapper still leaves `codex cloud exec` and the shell-completion helper to the CLI because they are experimental/setup-time utilities. Invoke them directly when needed (e.g., `codex cloud exec -- <cmd>` or `codex completion bash/zsh/fish` in your shell profile).

## Examples Index
- The full wrapper vs. native CLI matrix lives in `crates/codex/EXAMPLES.md`.
- Run any example via `cargo run -p codex --example <name> -- <args>`; most support `--sample` so you can read shapes without a binary present.
