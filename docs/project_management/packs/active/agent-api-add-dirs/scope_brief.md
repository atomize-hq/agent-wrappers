# Scope brief — Universal extra context roots (`agent_api.exec.add_dirs.v1`)

## Goal

Introduce one bounded, cross-backend run extension for extra context directories so callers can
request additional filesystem roots without depending on backend-specific flags.

## Why now

ADR-0021 pins the contract, but the implementation still has to thread the same normalized add-dir
set through `agent_api`, Codex, and Claude Code without backend drift or session-flow gaps.

## Primary users + JTBD

- **Host integrators / orchestrators**: “Run a prompt against a primary working directory while
  intentionally granting the backend access to additional directories such as sibling repos, shared
  docs trees, or checked-out assets.”

## In-scope

- Implement `agent_api.exec.add_dirs.v1` as a supported run extension for built-in backends.
- Enforce the pinned v1 contract from:
  - `docs/adr/0021-universal-agent-api-add-dirs.md`
  - `docs/specs/universal-agent-api/extensions-spec.md`
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
  - Codex: repeated `--add-dir <DIR>`
  - Claude Code: one variadic `--add-dir <DIR...>` group
- Pin the backend-owned contract docs that make session behavior testable:
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
- **Safe errors**: `InvalidRequest` and runtime backend errors do not echo raw path values.

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
- The capability inventory change is published by regenerating
  `docs/specs/universal-agent-api/capability-matrix.md` via
  `cargo run -p xtask -- capability-matrix` in the same change.

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
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
  - `docs/specs/claude-code-session-mapping-contract.md`
  - `docs/specs/universal-agent-api/capability-matrix.md`

## Known unknowns / risks

- **Codex fork transport parity**: resolved for the current v1 contract as a pinned pre-handle
  backend rejection (`"add_dirs unsupported for codex fork"`) until the app-server schema exposes
  a dedicated add-dir field.
- **Effective working directory handoff**: add-dir normalization must use the same effective
  working directory a backend run will actually use, not a parallel approximation.
- **No path leaks in errors**: filesystem validation is easy to implement incorrectly by echoing
  rejected path text in user-visible messages.

## Assumptions (explicit)

- The `extensions-spec.md` section for `agent_api.exec.add_dirs.v1` is the authoritative v1
  contract; this pack is for implementation decomposition, not semantic invention.
- The current wrapper crates already expose sufficient backend primitives for add-dir argv
  emission, so most implementation risk is in `agent_api` validation/plumbing, Codex fork
  rejection wiring, and keeping the backend contract docs aligned.
- Built-in backends should advertise `agent_api.exec.add_dirs.v1` unconditionally once the
  implementation is landed, independent of the per-run path contents.
