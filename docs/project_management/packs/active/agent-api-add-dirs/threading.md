# Threading — Universal extra context roots (`agent_api.exec.add_dirs.v1`)

This section makes coupling explicit: contracts/interfaces, dependency edges, and sequencing.

## Contract registry

Ownership note: within this pack, **Owner seam** refers to workstream ownership for implementing
and validating behavior. Normative ownership for `agent_api.*` extension-key semantics is always
`docs/specs/universal-agent-api/extensions-spec.md` (and foundational contract terms live in
`docs/specs/universal-agent-api/contract.md`). If any pack text conflicts with the normative specs,
the specs win.

- **AD-C01 — Core add-dir extension key**
  - **Type**: schema
  - **Owner seam (pack)**: SEAM-1
  - **Normative owner doc**: `docs/specs/universal-agent-api/extensions-spec.md`
  - **Consumers**: SEAM-2/3/4/5
  - **Definition**: `agent_api.exec.add_dirs.v1` is a closed object schema with required
    `dirs: string[]`, `dirs.len()` in `1..=16`, and per-entry trimmed byte bound `<= 1024`. Trimming
    is leading/trailing Unicode whitespace per the owner doc.

- **AD-C02 — Effective add-dir set algorithm**
  - **Type**: config
  - **Owner seam (pack)**: SEAM-2
  - **Consumers**: SEAM-3/4/5
  - **Definition**: the backend adapter layer computes one effective directory list by first
    resolving the effective working directory inside
    `crates/agent_api/src/backends/codex/harness.rs::CodexHarnessAdapter::validate_and_extract_policy(...)`
    or
    `crates/agent_api/src/backends/claude_code/harness.rs::ClaudeHarnessAdapter::validate_and_extract_policy(...)`,
    then passing
    `request.extensions.get("agent_api.exec.add_dirs.v1")`
    plus that already-selected directory into
    `backend_harness::normalize::normalize_add_dirs_v1(...)`.
    Request defaults, backend config defaults, and backend-internal fallbacks MUST be resolved
    before calling the helper. The helper MUST return `Ok(Vec::new())` when the key is absent;
    otherwise it trims leading/trailing Unicode whitespace (per the owner doc), resolves relatives
    against the run's effective working directory (per
    `docs/specs/universal-agent-api/contract.md` "Working directory resolution (effective working directory)"),
    lexically normalizes, verifies `exists && is_dir`, and deduplicates while preserving first
    occurrence order. This list is exported as `Vec<PathBuf>` and attached to the backend policy
    consumed by SEAM-3/4. No downstream code may reread the raw extension payload.

- **AD-C03 — Safe error posture**
  - **Type**: policy
  - **Owner seam (pack)**: SEAM-1
  - **Consumers**: SEAM-2/3/4/5
  - **Definition**: `InvalidRequest` messages for this key MUST use one of the exact safe
    templates `invalid agent_api.exec.add_dirs.v1`,
    `invalid agent_api.exec.add_dirs.v1.dirs`, or
    `invalid agent_api.exec.add_dirs.v1.dirs[<i>]`, where `<i>` is the zero-based failing entry
    index. Backends MUST NOT invent any other `InvalidRequest` message shape for this key.
    Runtime failures surface as safe/redacted backend errors (and MUST NOT embed raw backend
    stdout/stderr) per the owner doc.

- **AD-C04 — Session-flow parity**
  - **Type**: integration
  - **Owner seam (pack)**: SEAM-1
  - **Consumers**: SEAM-3/4/5
  - **Definition**: `agent_api.exec.add_dirs.v1` is orthogonal to the session selector keys
    `agent_api.session.resume.v1` and `agent_api.session.fork.v1`; session flows are selected via
    `AgentWrapperRunRequest.extensions` (not separate request fields). Backends MUST preserve the
    same accepted effective add-dir set across new-session, resume, and fork decision-making, and
    MUST NOT silently ignore accepted inputs. Claude applies the accepted set on fork flows. The
    current Codex fork contract uses the pinned pre-handle backend rejection path from
    `docs/specs/codex-app-server-jsonrpc-contract.md`. Pack-level verification obligations are:
    Claude resume selector `"last"`, Claude resume selector `"id"`, Claude fork selector `"last"`,
    and Claude fork selector `"id"` each get their own argv-placement assertion; Codex fork
    selector `"last"` and selector `"id"` each prove the same pre-request rejection boundary for
    accepted add-dir inputs. (Normative: see
    `docs/specs/universal-agent-api/extensions-spec.md` `agent_api.exec.add_dirs.v1`.)

- **AD-C05 — Codex argv mapping**
  - **Type**: integration
  - **Owner seam**: SEAM-3
  - **Consumers**: SEAM-5
  - **Definition**: Codex receives one repeated `--add-dir <DIR>` pair per normalized unique
    directory, in order, on exec/resume flows. Any accepted `--model` pair stays earlier in argv per
    `docs/specs/codex-streaming-exec-contract.md`.

- **AD-C06 — Claude Code argv mapping**
  - **Type**: integration
  - **Owner seam**: SEAM-4
  - **Consumers**: SEAM-5
  - **Definition**: Claude Code receives one variadic `--add-dir <DIR...>` group containing the
    normalized unique directories, in order, after any accepted `--model` pair and before
    `--continue` / `--fork-session` / `--resume` and the final `--verbose` token.

- **AD-C07 — Absence semantics**
  - **Type**: policy
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-2/3/4/5
  - **Definition**: when the key is absent, no backend synthesizes extra directories and no
    `--add-dir` argv is emitted.

- **AD-C08 — Codex fork validation-vs-rejection precedence**
  - **Type**: integration
  - **Owner seam**: SEAM-3
  - **Consumers**: SEAM-5
  - **Definition**: the Codex-specific backend rejection
    `AgentWrapperError::Backend { message: "add_dirs unsupported for codex fork" }` applies only
    after `agent_api.exec.add_dirs.v1` passes R0 capability gating and pre-spawn validation. If the
    add-dir payload is malformed, out of bounds, missing, or resolves to a missing/non-directory
    path, the run MUST fail earlier as `InvalidRequest`. Neither the invalid-input path nor the
    accepted-input fork rejection path may send `thread/list`, `thread/fork`, or `turn/start`.

## Dependency graph (text)

- `SEAM-1 blocks SEAM-2` because: the shared normalizer must implement the already-pinned v1
  schema, normalization, and safe-error rules.
- `SEAM-2 blocks SEAM-3` because: Codex support should consume the shared normalized directory set
  exported from `backend_harness::normalize::normalize_add_dirs_v1(...)` and the pinned Codex fork
  rejection contract instead of inventing backend-local path semantics.
- `SEAM-2 blocks SEAM-4` because: Claude Code support should consume the same shared normalized
  `Vec<PathBuf>` output from `backend_harness::normalize::normalize_add_dirs_v1(...)` instead of
  inventing backend-local path semantics.
- `SEAM-3 blocks SEAM-5` because: tests must pin Codex capability advertising, argv order, and
  session-flow behavior, including the fork rejection boundary and the invalid-input precedence
  rule.
- `SEAM-4 blocks SEAM-5` because: tests must pin Claude Code capability advertising, argv order,
  add-dir placement, selector-branch coverage, and runtime rejection parity.

## Critical path

`SEAM-1 (contract)` → `SEAM-2 (shared normalizer)` → `SEAM-3 (Codex mapping + fork rejection)` /
`SEAM-4 (Claude mapping + argv placement)` →
`SEAM-5 (tests + capability artifact + folded integration closeout)`

## Integration points

- **Run extension gate**: `backend_harness::normalize_request()` must fail closed on unsupported
  keys before any add-dir value parsing happens.
- **Effective working directory handoff**: the shared normalizer and each backend’s spawn path
  must agree on the same working directory source. Effective working directory is defined in
  `docs/specs/universal-agent-api/contract.md` ("Working directory resolution (effective working directory)"),
  and the concrete add-dir resolution locus is backend policy extraction, not spawn:
  `CodexHarnessAdapter::validate_and_extract_policy(...)` and
  `ClaudeHarnessAdapter::validate_and_extract_policy(...)` MUST compute the effective working
  directory, call `normalize_add_dirs_v1(...)`, and store the resulting `Vec<PathBuf>` on the
  backend policy before any spawn-specific argv/app-server mapping happens.
- **Session selectors**: resume/fork parsing stays orthogonal, but accepted add-dir inputs must
  survive into those flows. SEAM-4 and SEAM-5 MUST separately verify Claude resume selector
  `"last"`, Claude resume selector `"id"`, Claude fork selector `"last"`, and Claude fork selector
  `"id"` because the canonical argv subsequences differ by branch. The one pinned exception is
  Codex fork: for accepted add-dir inputs, selector `"last"` and selector `"id"` both reject before
  any `thread/list`, `thread/fork`, or `turn/start` request using the backend-owned safe message.
- **Fork rejection precedence**: Codex fork-specific backend rejection happens only after accepted
  add-dir inputs clear capability gating and pre-spawn validation. Invalid fork + add-dir
  combinations MUST fail earlier as `InvalidRequest`, so SEAM-3/5 must verify that invalid-input
  failures win before the fork-specific backend rejection path.
- **Runtime rejection parity**: handle-returning surfaces that later discover they cannot honor an
  accepted add-dir set MUST emit exactly one terminal `AgentWrapperEventKind::Error` event with the
  same safe/redacted message later surfaced through `AgentWrapperError::Backend { message }`.
  SEAM-4/5 own the explicit coverage for Claude fresh-run/resume/fork surfaces; Codex fork is
  excluded because its pinned contract rejects before a run handle is returned.
- **Runtime rejection fixtures**: deterministic post-handle add-dir rejection is owned only by
  `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` (Codex exec/resume) and
  `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` (Claude fresh/resume/fork).
  Dedicated `add_dirs_runtime_rejection_*` scenario ids MUST be added there and MUST NOT reuse the
  existing generic non-zero/session-selector fixtures, because the add-dir parity contract requires
  `AgentWrapperError::Backend { message }` plus an identical terminal `Error` event rather than the
  generic non-success completion path.
- **Wrapper crate parity**: `codex::CodexClientBuilder` and `claude_code::ClaudePrintRequest`
  already expose add-dir surfaces; backend seams wire the normalized list into them.
- **Shared normalizer anchor**: SEAM-2 owns `backend_harness::normalize::normalize_add_dirs_v1(...)`
  and the `Vec<PathBuf>` output consumed by both backend policy layers.
- **Canonical backend docs**: SEAM-3 and SEAM-4 are not done until
  `docs/specs/codex-streaming-exec-contract.md`,
  `docs/specs/codex-app-server-jsonrpc-contract.md` and
  `docs/specs/claude-code-session-mapping-contract.md` reflect the exact mapping/rejection truth.
- **Capability publication**: SEAM-5 must regenerate
  `docs/specs/universal-agent-api/capability-matrix.md` with
  `cargo run -p xtask -- capability-matrix` once the backend capability ids change.

## Parallelization notes / conflict-safe workstreams

- **WS-CONTRACT**: SEAM-1 (`extensions-spec.md` confirmation + pack contract).
- **WS-NORMALIZE**: SEAM-2 (shared normalizer + reusable validation/resolution helpers).
- **WS-CODEX**: SEAM-3 (Codex capability + policy + exec/resume/fork mapping).
- **WS-CLAUDE**: SEAM-4 (Claude capability + policy + print/resume/fork mapping).
- **WS-TESTS / WS-INT**: SEAM-5 (shared normalizer tests, backend capability/mapping/session
  tests, selector-branch verification, runtime rejection parity, capability-matrix regeneration,
  and final `make preflight` integration closeout).

## Pinned decisions / resolved threads

- **Trimming is leading/trailing Unicode whitespace** (per the owner doc).
- **Relative paths are allowed** and resolve against the effective working directory.
- **No containment rule** is imposed for v1.
- **Lexical normalization only**: no shell expansion, env expansion, canonicalization, or symlink
  resolution requirement.
- **Dedup is not an error**: duplicates collapse after normalization while preserving order.
