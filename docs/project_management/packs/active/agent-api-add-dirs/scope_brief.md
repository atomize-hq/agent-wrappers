# Scope brief — Universal extra context roots (`agent_api.exec.add_dirs.v1`)

## Goal

Introduce one bounded, cross-backend run extension for extra context directories so callers can
request additional filesystem roots without depending on backend-specific flags.

## Why now

The Universal Agent API owner docs pin the v1 contract (`extensions-spec.md` for key semantics and
`contract.md` for effective working directory). ADR-0021 is rationale + an implementation plan; the
remaining work is threading one normalized add-dir set through `agent_api`, Codex, and Claude Code
without backend drift or session-flow gaps.

## Primary users + JTBD

- **Host integrators / orchestrators**: “Run a prompt against a primary working directory while
  intentionally granting the backend access to additional directories such as sibling repos, shared
  docs trees, or checked-out assets.”

## In-scope

- Implement `agent_api.exec.add_dirs.v1` as a supported run extension for built-in backends.
- Enforce the pinned v1 contract as defined by:
  - `docs/specs/universal-agent-api/extensions-spec.md` (normative owner doc for the key)
  - `docs/specs/universal-agent-api/contract.md` ("Working directory resolution (effective working directory)")
- ADR-0021 is non-normative rationale/plan; treat it as derived-from-spec checklist only. If any
  ADR text conflicts with the owner doc, resolve conflicts in favor of the owner doc.
- Add deterministic validation and normalization:
  - closed object schema,
  - `dirs` bounds,
  - trim + resolve + lexical normalize + dedup,
  - pre-spawn existence and directory checks,
  - stable safe `InvalidRequest` messages.
- Preserve deterministic session-flow behavior across:
  - new-session runs,
  - resume flows,
  - fork flows.
- Map the normalized directories into both built-in backends:
  - Codex exec/resume: repeated `--add-dir <DIR>` pairs after any `--model` pair
  - Claude Code: one variadic `--add-dir <DIR...>` group after any `--model` pair and before
    `--continue` / `--fork-session` / `--resume` and the final `--verbose` token
- Pin the backend-owned contract docs that make session behavior testable:
  - `docs/specs/codex-streaming-exec-contract.md`
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
  - `docs/specs/claude-code-session-mapping-contract.md`

## Out-of-scope

- Defining a host sandbox or security policy.
- Restricting directories to remain under the effective working directory.
- Supporting files instead of directories in v1.
- Adding a backend-specific raw path pass-through outside the core key.

## Capability inventory (implied)

- Core extension key:
  - `agent_api.exec.add_dirs.v1`
- Schema + bounds:
  - object with required `dirs: string[]`
  - `dirs.len()` in `1..=16`
  - each trimmed entry non-empty and `<= 1024` UTF-8 bytes
  - closed schema (`.v1`)
- Resolution + normalization:
  - relative paths resolve against the effective working directory
  - lexical normalization only
  - no `~` expansion
  - no env-var expansion
  - no canonicalization or symlink resolution requirement
  - dedup after normalization, preserving first occurrence order
- Pre-spawn validation:
  - resolved path exists
  - resolved path is a directory
  - invalid messages do not leak raw path values
- Backend mapping:
  - Codex repeated flag pairs
  - Claude Code single variadic flag group
- Session compatibility:
  - new/resume must honor the accepted directory set
  - Claude fork must honor the accepted directory set
  - current Codex fork behavior is the pinned safe backend rejection path from
    `docs/specs/codex-app-server-jsonrpc-contract.md`

## Required invariants (must not regress)

- **Fail-closed R0 gating**: unsupported key fails as `UnsupportedCapability` before value
  validation.
- **No synthetic defaults**: when absent, built-in backends do not emit `--add-dir`.
- **No containment rule**: valid directories outside the effective working directory remain legal.
- **Same normalization contract for both backends**: the wrapper decides the effective directory
  set before backend argv mapping.
- **Session parity**: accepted add-dir inputs are not silently dropped for resume or fork flows;
  the only allowed exception path is the pinned Codex fork rejection contract.
- **Safe errors**:
  - `InvalidRequest` messages MUST NOT echo raw path values and MUST use the pinned safe templates.
  - runtime backend errors MUST be safe/redacted and MUST NOT embed raw backend stdout/stderr.

## Success criteria

- A caller can send `extensions["agent_api.exec.add_dirs.v1"]` through `AgentWrapperRunRequest`
  and both built-in backends advertise deterministic behavior for the key.
- Relative and absolute directory inputs resolve deterministically from the effective working
  directory and backend defaults already in use.
- Duplicate directories collapse deterministically after normalization.
- Missing or non-directory paths fail before spawn.
- Claude resume/fork flows apply the accepted directory set with the pinned variadic argv
  placement.
- Codex fork rejects accepted add-dir inputs before app-server requests with the pinned safe
  backend message.
- Capability publication is owned by the implementation change that flips built-in advertising for
  `agent_api.exec.add_dirs.v1`:
  - the canonical capability id registry
    `docs/specs/universal-agent-api/capabilities-schema-spec.md` is expected to already contain the
    stable id, and
  - the same change that lands built-in advertising MUST regenerate
    `docs/specs/universal-agent-api/capability-matrix.md` via
    `cargo run -p xtask -- capability-matrix`, with the
    `agent_api.exec.add_dirs.v1` row showing ✅ for both `claude_code` and `codex`.
  Until that implementation change lands, omission from the generated matrix remains the current
  canonical truth for the built-in target profile: no built-in backend advertises
  `agent_api.exec.add_dirs.v1` yet.

## Ownership + verification

- Canonical capability id registry: `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  (owned by universal-agent-api spec maintainers).
- Generated overview: `docs/specs/universal-agent-api/capability-matrix.md` (regen required).
- Reviewers MUST verify that `agent_api.exec.add_dirs.v1` appears in the canonical registry.
- Reviewers MUST expect the regenerated matrix row only in the same change that actually lands
  built-in advertising for the key.

## Constraints

- Canonical semantics are owned by `docs/specs/universal-agent-api/extensions-spec.md`.
- Public API and policy extraction stay serde-friendly and backend-neutral at the `agent_api`
  boundary.
- Tests must stay deterministic and avoid depending on real external CLIs or network access.

## External systems / dependencies

- Upstream CLIs and wrapper surfaces:
  - `crates/codex/src/builder/mod.rs`
  - `crates/claude_code/src/commands/print.rs`
- Existing run harness + session flow infrastructure:
  - `crates/agent_api/src/backend_harness/**`
  - `crates/agent_api/src/backends/session_selectors.rs`
  - `crates/agent_api/src/backends/codex/**`
  - `crates/agent_api/src/backends/claude_code/**`
- Canonical backend mapping docs:
  - `docs/specs/codex-streaming-exec-contract.md`
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
  - `docs/specs/claude-code-session-mapping-contract.md`
  - `docs/specs/universal-agent-api/capability-matrix.md`

## Known unknowns / risks

- **Codex fork transport parity**: resolved for the current v1 contract as a pinned pre-handle
  backend rejection (`"add_dirs unsupported for codex fork"`) until the app-server schema exposes
  a dedicated add-dir field.
- **Effective working directory handoff**: add-dir normalization must use the same effective
  working directory a backend run will actually use, not a parallel approximation. For this pack,
  the concrete computation locus is backend policy extraction:
  `CodexHarnessAdapter::validate_and_extract_policy(...)` and
  `ClaudeHarnessAdapter::validate_and_extract_policy(...)` MUST resolve working-directory
  precedence before calling the shared normalizer, and Claude's backend run entrypoints MUST mirror
  Codex by capturing run-start `current_dir()` so the backend-internal fallback is explicit before
  add-dir validation.
- **No path leaks in errors**: filesystem validation is easy to implement incorrectly by echoing
  rejected path text in user-visible messages.

## Assumptions (explicit)

- The `extensions-spec.md` section for `agent_api.exec.add_dirs.v1` is the authoritative v1
  contract; this pack is for implementation decomposition, not semantic invention.
- The current wrapper crates already expose sufficient backend primitives for add-dir argv
  emission, so most implementation risk is in `agent_api` validation/plumbing, Codex fork
  rejection wiring, and keeping the backend contract docs aligned.

## Pinned implementation decisions

- Built-in backends MUST advertise `agent_api.exec.add_dirs.v1` unconditionally once the
  implementation is landed, independent of the per-run path contents.
- SEAM-2 owns a shared `backend_harness::normalize::normalize_add_dirs_v1(...)` helper that
  returns the backend-consumed `Vec<PathBuf>` normalized directory list.
- The shared helper MUST be called as
  `normalize_add_dirs_v1(request.extensions.get("agent_api.exec.add_dirs.v1"), effective_working_dir)`.
  When the key is absent, the helper MUST return `Ok(Vec::new())`, and backends MUST emit no
  `--add-dir` argv.
- Backend policy extraction, not backend spawn code, owns effective-working-directory resolution
  for add-dir validation. Codex uses request `working_dir` → backend `default_working_dir` →
  captured `run_start_cwd`; Claude uses request `working_dir` → backend `default_working_dir` →
  captured `run_start_cwd`.
- Backend policy structs carry the resulting `Vec<PathBuf>` forward. Downstream backend mapping and
  spawn layers MUST NOT reread `AgentWrapperRunRequest.extensions["agent_api.exec.add_dirs.v1"]`.
- `exists && is_dir` validation remains inside the shared helper; no backend-local pre-spawn
  filesystem validation step exists outside SEAM-2 for this key.
