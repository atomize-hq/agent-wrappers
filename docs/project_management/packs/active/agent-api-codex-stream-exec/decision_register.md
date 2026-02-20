# Decision Register — Agent API Codex `stream_exec` parity

Status: Draft  
Date (UTC): 2026-02-20  
Feature directory: `docs/project_management/packs/active/agent-api-codex-stream-exec/`

This register records the non-trivial architectural decisions required to make ADR-0011 execution-ready.
Each decision is exactly two options (A/B) with explicit tradeoffs and one selection.

Inputs:
- ADR: `docs/adr/0011-agent-api-codex-stream-exec.md`
- Spec manifest: `docs/project_management/packs/active/agent-api-codex-stream-exec/spec_manifest.md`
- Impact map: `docs/project_management/packs/active/agent-api-codex-stream-exec/impact_map.md`
- Baselines:
  - `docs/project_management/next/universal-agent-api/contract.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`

## DR-0002 — Per-run environment override strategy (Codex wrapper → spawned process)

**A) Rely on global process env mutation (`std::env::set_var`) to “override per run”**
- Pros: minimal API work in `crates/codex`.
- Cons: unsafe (cross-run leakage), not concurrency-safe, violates “per-run” semantics, and is hostile to multi-agent hosting.

**B) Add explicit per-request env overrides that are applied to the `Command` for that run only (Selected)**
- Pros: correct isolation; deterministic precedence; works cross-platform; no parent-process mutation.
- Cons: requires additive API work in `crates/codex` (e.g., add `env_overrides` to `ExecStreamRequest` / resume request, or a parallel `stream_exec_with_env(...)` surface).

**Selected:** B

Pinned precedence rule (normative for this feature’s contract docs):
1) `AgentWrapperRunRequest.env` wins for keys it sets.
2) Then backend config env (`CodexBackendConfig.env`).
3) Then wrapper defaults applied by `crates/codex` (e.g., `CODEX_HOME`, default `RUST_LOG`).

Notes:
- Request env MUST be able to override `CODEX_HOME` and `RUST_LOG` for the spawned process (matches the current `agent_api` Codex backend behavior).
- “Unset/removal” of an env var is out of scope for v1 (this feature only requires set/override semantics).

## DR-0003 — Redaction strategy for `ExecStreamError` (no raw JSONL line leakage)

**A) Forward upstream error display text (`ExecStreamError::to_string()`) into universal errors/events**
- Pros: easiest; maximum debug detail.
- Cons: forbidden by the universal v1 safety posture: `ExecStreamError::{Parse,Normalize}` include the raw JSONL line in `Display`; `CodexError` variants may include raw stdout/stderr strings.

**B) Map upstream errors to redacted, stable summaries before emitting universal errors/events (Selected)**
- Pros: preserves safety posture; deterministic message shapes; works across platforms; compatible with envelope bounds (`message <= 4096` bytes).
- Cons: less debug detail; requires explicitly pinning the mapping (and tests later).

**Selected:** B

Pinned mapping rules (normative for this feature; authoritative wording lives in the adapter-protocol spec):
- MUST NOT include raw JSONL lines or raw stderr/stdout bytes.
- MUST NOT use `ExecStreamError::Display` / `to_string()` for emitted messages.
- MUST implement the canonical, deterministic redaction mapping from:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md`
  - including `line_bytes={n}` for parse/normalize errors and stable `{kind}` categories for `CodexError`.

## DR-0004 — `AgentWrapperCompletion.final_text` policy for Codex runs

**A) Always set `final_text = None` for Codex**
- Pros: preserves existing `agent_api` Codex backend behavior; avoids introducing a new “final answer extraction” guarantee without cross-backend parity signaling; simplest v1 contract.
- Cons: leaves potentially useful wrapper output (`ExecCompletion.last_message`) out of the universal completion surface.

**B) Populate `final_text` from `ExecCompletion.last_message` when available (Selected)**
- Pros: aligns with universal contract’s “may populate when deterministic”; provides a stable convenience for consumers; uses wrapper-provided output rather than heuristics.
- Cons: requires pinning a strict rule (when set vs `None`) and enforcing bounds; may differ from other backends until parity decisions are made.

**Selected:** B

Pinned rule (normative for this feature’s contract docs):
- `final_text = Some(s)` iff the upstream completion returns `Ok(ExecCompletion { last_message: Some(s), .. })`.
- Otherwise `final_text = None`.
- Bounds: if `final_text` exceeds `65536` bytes UTF-8, it MUST be truncated UTF-8-safely and suffixed with `…(truncated)`.

## DR-0005 — Non-zero exit behavior (Codex wrapper completion → universal completion)

**A) Treat non-zero exit as a universal error (`completion = Err(AgentWrapperError::Backend)`)**
- Pros: makes failures obvious; matches the “errors are errors” mental model.
- Cons: diverges from the universal run protocol’s meaning of “success”: success means the backend
  successfully ran and observed an `ExitStatus` (which may be non-zero). This forces more `Err`
  handling by consumers and can create cross-backend divergence (Claude currently reports status
  via `Ok(AgentWrapperCompletion { status, .. })`).

**B) Preserve current (pre-refactor) completion semantics: return `Ok(AgentWrapperCompletion { status: <non-zero>, ... })` and emit a redacted error event (Selected)**
- Pros: aligns with the universal run protocol (“on success, completion contains `ExitStatus`”),
  keeps cross-backend parity (Claude reports `ExitStatus` via `Ok(AgentWrapperCompletion { .. })`),
  and keeps a clean separation:
  - transport/backend failures → `Err(AgentWrapperError::Backend)`
  - process outcome (including non-zero) → `Ok(AgentWrapperCompletion { status, .. })`
- Cons: requires explicitly pinning the policy and ensuring stderr remains redacted.

**Selected:** B

Pinned rule (normative for this feature’s protocol + contract docs):
- On non-zero exit, downstream completion MUST be `Ok(AgentWrapperCompletion { status: <non-zero>, final_text: None, data: None })`.
- The adapter MUST emit a best-effort `AgentWrapperEventKind::Error` message:
  - `message = "codex exited non-zero: {status:?} (stderr redacted)"`

## DR-0006 — C0 Codex wrapper per-invocation env override API (exact public surface)

**A) Add fields to `ExecStreamRequest` / `ResumeRequest` to carry env overrides**
- Pros: keeps all inputs in a single request struct.
- Cons: breaking change for downstream callers constructing the public structs; high churn across
  the repo and consumers; violates “additive, no callsite breakage” requirement for C0.

**B) Add an additive `CodexClient` method that accepts a per-call env map (Selected)**
- Pros: fully additive; does not break existing `stream_exec(ExecStreamRequest { .. })` call sites;
  isolates per-call env injection as required by the universal backend.
- Cons: adds one additional method to maintain; requires parallel `resume` support later if needed.

**Selected:** B

Pinned contract surface (normative; must match `C0-spec.md` and be used by `agent_api`):
- `codex::CodexClient::stream_exec_with_env_overrides(exec_request, &env_overrides)`
  - `env_overrides: &BTreeMap<String, String>`
  - env overrides MUST be applied to the spawned `Command` after wrapper injection and MUST NOT
    mutate parent process env.

Explicit v1 scope boundary:
- This feature does not require a parallel env override API for `codex resume`.

## DR-0007 — Cross-platform CI evidence for this feature

**A) Rely on local/manual execution for macOS/Windows evidence**
- Pros: no CI changes.
- Cons: violates the platform-parity spec’s requirement for GitHub-hosted runner evidence; drifts
  easily; not deterministic for reviewers.

**B) Add a dedicated feature smoke workflow that runs the feature-local smoke scripts (Selected)**
- Pros: matches existing repo patterns (`claude-code-live-stream-json-smoke.yml`,
  `universal-agent-api-smoke.yml`); produces deterministic cross-platform evidence on GitHub-hosted
  runners; does not slow down baseline `ci.yml` for all PRs.
- Cons: adds another workflow file to maintain.

**Selected:** B

Pinned requirement:
- Add `.github/workflows/agent-api-codex-stream-exec-smoke.yml` that runs:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/linux-smoke.sh`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/macos-smoke.sh`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/windows-smoke.ps1`
  - and includes the existing public API guard used by other smoke workflows.

## DR-0008 — Post-merge CI gating for `agent_api` backend feature flags

**A) Only run `agent_api` backend feature tests in feature-branch smoke workflows**
- Pros: keeps baseline CI fast.
- Cons: once merged, future changes can silently break `agent_api --features codex/claude_code`
  without any PR CI signal; contradicts the “mechanical onboarding” goal.

**B) Add a baseline CI job that always runs `cargo test -p agent_api --all-features` (Selected)**
- Pros: ensures ongoing coverage for optional backends after merge; aligns with the repo’s existing
  “workspace tests” baseline by adding a narrow, targeted job.
- Cons: increases CI time on PRs (ubuntu-only, but still additional).

**Selected:** B

Pinned requirement:
- Edit `.github/workflows/ci.yml` to include a job on `ubuntu-latest` that runs:
  - `cargo test -p agent_api --all-features`

## DR-0009 — Codex non-interactive + sandbox policy (explicit flags vs `--full-auto`)

**A) Use Codex CLI `--full-auto` as the non-interactive mechanism**
- Pros: single flag; matches common Codex CLI usage patterns.
- Cons: ambiguous as a contract surface (it is a safety override, not an explicit policy); harder to
  reason about in a library context because it is only applied when other flags are absent; makes
  it difficult to offer a clean “lever” for hosts to select sandbox/approvals deterministically.

**B) Pin explicit policies by default: `--ask-for-approval never` + `--sandbox workspace-write`, and expose a per-run lever to override (Selected)**
- Pros: fully deterministic contract; avoids relying on “override” semantics; supports greenfield
  ergonomics by making the universal backend automation-safe by default while still allowing hosts
  (like Substrate) to opt into `danger-full-access` explicitly per run.
- Cons: requires defining/validating a small extension surface and mapping it per backend.

**Selected:** B

Pinned requirements (normative for `contract.md` + adapter protocol):
- The Codex backend MUST NOT use dangerously-bypass/yolo modes.
- Default behavior (when extensions are absent):
  - MUST pass `--ask-for-approval never` (non-interactive).
  - MUST pass `--sandbox workspace-write`.
- The backend MUST support the extension keys pinned in DR-0010 to override these policies per run.

## DR-0010 — Exec policy extension surface (non-interactive + sandbox/approvals)

**A) No exec-policy extensions in v1; hard-code a single policy per backend**
- Pros: minimal surface area; simplest validation.
- Cons: contradicts the “lever in the API” requirement; forces forks or config mutation for hosts
  that need different execution modes (e.g., Substrate wanting `danger-full-access`).

**B) Add a minimal, orthogonal extension surface with explicit defaults (Selected)**
- Pros: aligns with universal capability gating model; scales to additional CLIs; keeps the
  universal API orthogonal by separating “run protocol” from “execution policy inputs”.
- Cons: must be explicitly documented to avoid drift.

**Selected:** B

Pinned extension keys (normative; authoritative text also appears in `contract.md`):
- Core:
  - `agent_api.exec.non_interactive` (core key; schema + defaults are owned by):
    - `docs/project_management/next/universal-agent-api/extensions-spec.md`
- Codex-specific:
  - `backend.codex.exec.sandbox_mode`: string enum:
    - `read-only` | `workspace-write` | `danger-full-access`
    - default: `workspace-write`
  - `backend.codex.exec.approval_policy`: string enum:
    - `untrusted` | `on-failure` | `on-request` | `never`
    - default: absent (but when `agent_api.exec.non_interactive=true`, the backend MUST force `never`)

Pinned validation rules (normative):
- Unknown extension keys MUST fail-closed as `AgentWrapperError::UnsupportedCapability` before spawn.
- Value types MUST be validated before spawn:
  - `agent_api.exec.non_interactive` MUST be boolean (per `extensions-spec.md`).
  - the Codex strings MUST match one of the allowed values.
- Contradiction rule:
  - if `agent_api.exec.non_interactive=true`, then `backend.codex.exec.approval_policy` MUST be
    absent or `"never"`; otherwise fail with `AgentWrapperError::InvalidRequest`.
