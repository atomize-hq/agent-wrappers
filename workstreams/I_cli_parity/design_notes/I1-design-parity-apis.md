# Workstream I1 — CLI Parity API Design

Goal: extend the Rust wrapper so every missing Codex CLI flag/config override is reachable from the builder or per-request APIs without changing current defaults. Flags to cover: `--config`, `--ask-for-approval`, `--sandbox`, `--full-auto`, `--dangerously-bypass-approvals-and-sandbox`, `--cd`, `--local-provider`, `--search`, `--last`, `--all`, plus explicit setters for reasoning/verbosity config keys.

## Proposed surface
- **Common CLI patch (`CliOverrides`)** stored on the builder and merged per request. Fields:
  - `config_overrides: Vec<ConfigOverride>` for `--config key=value` (ordered; request entries applied after builder).
  - `reasoning: ReasoningTuning` (all optional) mapping to config keys: `model_reasoning_effort`, `model_reasoning_summary`, `model_verbosity`, `model_reasoning_summary_format`, `model_supports_reasoning_summaries`.
  - `approval_policy: Option<ApprovalPolicy>` → `--ask-for-approval <policy>`.
  - `sandbox_mode: Option<SandboxMode>` → `--sandbox <mode>`.
  - `safety_override: SafetyOverride` (`Inherit` | `FullAuto` | `DangerouslyBypass`). `DangerouslyBypass` wins over sandbox/approval, otherwise explicit sandbox/approval win; `FullAuto` only applies when neither explicit field is set.
  - `cd: Option<PathBuf>` → `--cd <dir>` (separate from `working_dir`, which still sets `Command::current_dir`).
  - `local_provider: Option<LocalProvider>` → `--local-provider <lmstudio|ollama|custom>`.
  - `search: FlagState` (`Inherit` | `Enable` | `Disable`). Only `Enable` emits `--search`; `Disable` suppresses a builder default.
- **Config override helpers**
  - `ConfigOverride { key: String, value: String }` plus builder/request helpers: `.config_override(key, value)`, `.config_overrides(iterable)`, and `.config_override_raw("key=value")` for preformatted values (no quoting/escaping).
  - Reasoning helpers on the builder: `.reasoning_effort(ReasoningEffort)`, `.reasoning_summary(ReasoningSummary)`, `.reasoning_verbosity(ModelVerbosity)`, `.reasoning_summary_format(ReasoningSummaryFormat)`, `.supports_reasoning_summaries(bool)`, `.auto_reasoning_defaults(bool)` (default `true`, preserving today’s auto-injection for GPT-5* models unless any reasoning override/config override for those keys is present).
- **Exec request shape**
  - New `ExecRequest { prompt: String, overrides: CliOverridesPatch }` where `CliOverridesPatch` is a partial overlay (all fields optional/`Inherit`). `send_prompt` remains; new `send_prompt_with(request)` (name TBD) builds the request and reuses `invoke_codex_exec`.
  - `ExecStreamRequest` embeds `ExecRequest` plus the existing streaming fields (`idle_timeout`, `output_last_message`, `output_schema`, `json_event_log`); current constructor stays as a convenience.
- **Resume**
  - Add `ResumeRequest { selector: ResumeSelector, prompt: Option<String>, overrides: CliOverridesPatch, stream: StreamArtifacts }`.
  - `ResumeSelector`: `Id(String)` (default), `Last` → `--last`, `All` → `--all` (mutually exclusive).
  - `stream_resume` mirrors `stream_exec` but shells out to `codex resume --json --skip-git-repo-check` and reuses the same CLI flag builder.
- **Command coverage**
  - `CliOverrides` applied to `exec`, `resume`, `apply`, and `diff` (shared flags: config overrides, safety, cd/local-provider). `--search` only emitted for `exec`/`resume`.
  - Existing fields (`model`, `color_mode`, `add_dirs`, `images`, `json`, `output_schema`, `quiet`, `mirror_stdout`, `capability_*`) remain intact and compose with the new overlays.

## Merging & precedence
- Start from builder `CliOverrides`; overlay per-request patch (`Some`/`Enable`/`Disable` wins, `Inherit` defers). Config overrides concatenate in order: auto reasoning defaults (when allowed) → builder → request; last writer for a key wins when emitting `--config`.
- Safety resolution: if final `safety_override` is `DangerouslyBypass`, emit only `--dangerously-bypass-approvals-and-sandbox`. Otherwise, emit explicit `sandbox_mode`/`approval_policy` when set; if neither is set and `FullAuto` is selected, emit `--full-auto`.
- Search: only `FlagState::Enable` produces `--search`. `Disable` suppresses a builder default so requests can turn search off.
- `cd` is additive; `working_dir` continues to set the child process cwd (temp dir by default). If both are set, the process cwd follows `working_dir` and Codex receives `--cd <dir>` as instructed.
- Reasoning defaults: kept for GPT-5* when `auto_reasoning_defaults` is true and no reasoning/config override touches those keys. Any explicit reasoning override or `config_override` for those keys suppresses the auto defaults.

## Wiring plan
- Add a helper (e.g., `apply_cli_overrides(&CliOverrides, &CliOverridesPatch, Command)`) that appends CLI args and returns the resolved config list for logging/tests. Reuse across `invoke_codex_exec`, `stream_exec`, `apply_or_diff`, and the new `stream_resume`.
- Continue to set `--skip-git-repo-check`, `--json`/`--output-schema`, color handling, and capability guards as today. New flags are only emitted when set so older binaries remain compatible unless callers opt in to unsupported flags.
