# SEAM-3 — Codex backend support (threaded decomposition)

> Pack: `docs/project_management/packs/active/agent-api-add-dirs/`
> Seam brief: `seam-3-codex-mapping.md`
> Threading source of truth: `threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-3
- **Name**: Codex `agent_api.exec.add_dirs.v1` support
- **Goal / value**: let Codex advertise and consume the shared normalized add-dir set on
  exec/resume flows while keeping fork behavior deterministic under the pinned app-server
  limitation.
- **Type**: platform
- **Scope**
  - In:
    - Advertise `agent_api.exec.add_dirs.v1` from the Codex backend once the implementation is
      landed.
    - Add the key to Codex supported-extension allowlists.
    - Resolve the effective working directory during Codex policy extraction, call the shared
      `backend_harness::normalize::normalize_add_dirs_v1(...)` helper, and carry the resulting
      `Vec<PathBuf>` on the Codex policy.
    - Map the normalized directory list to repeated `--add-dir <DIR>` pairs on exec/resume flows,
      preserving order and keeping any accepted `--model` pair earlier in argv.
    - Reject accepted add-dir inputs on fork flows before any `thread/list`, `thread/fork`, or
      `turn/start` request with the pinned safe backend message.
    - Keep `docs/specs/codex-streaming-exec-contract.md` and
      `docs/specs/codex-app-server-jsonrpc-contract.md` aligned with the implemented truth.
  - Out:
    - Defining the universal add-dir schema, safe invalid-request templates, or absence semantics
      (SEAM-1).
    - Implementing the shared normalizer and filesystem validation helper (SEAM-2).
    - Claude Code behavior (SEAM-4).
    - Pack-level regression coverage and capability-matrix regeneration (SEAM-5).
- **Touch surface**:
  - `crates/agent_api/src/backends/codex/backend.rs`
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/codex/policy.rs`
  - `crates/agent_api/src/backends/codex/exec.rs`
  - `crates/agent_api/src/backends/codex/fork.rs`
  - `docs/specs/codex-streaming-exec-contract.md`
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
  - Existing wrapper integration surface: `crates/codex/src/builder/mod.rs`
- **Verification**:
  - Targeted Codex backend tests while landing the seam:
    `cargo test -p agent_api codex`
  - Pack-level end-to-end verification is owned by SEAM-5 once this behavior is in place.
- **Threading constraints**
  - Upstream blockers:
    - SEAM-1 for AD-C01, AD-C03, AD-C04, AD-C07
    - SEAM-2 for AD-C02
  - Downstream blocked seams:
    - SEAM-5
  - Contracts produced (owned):
    - AD-C05
    - AD-C08
  - Contracts consumed:
    - AD-C01
    - AD-C02
    - AD-C03
    - AD-C04
    - AD-C07

Implementation note: the Codex backend docs remain the canonical, Normative source of truth for
AD-C05 and AD-C08. The slices below are an implementation checklist that preserves the threading
constraints already fixed by the pack.

## Slice index

- `S1` → `slice-1-capability-and-policy-plumbing.md`: advertise the key and extract one normalized
  add-dir list into typed Codex policy state.
- `S2` → `slice-2-exec-resume-add-dir-mapping.md`: map the normalized list into repeated
  `--add-dir` argv pairs for exec/resume while preserving ordering guarantees.
- `S3` → `slice-3-fork-rejection-precedence.md`: enforce the pinned Codex fork rejection boundary
  after validation and before any app-server request.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `AD-C05`: Codex argv mapping contract. Codex exec/resume flows emit one repeated
    `--add-dir <DIR>` pair per normalized unique directory, in order, after any accepted
    `--model` pair. Canonical location:
    `docs/specs/codex-streaming-exec-contract.md`.
    - Produced by: `S2` with policy inputs prepared by `S1`.
  - `AD-C08`: Codex fork validation-vs-rejection precedence. Accepted add-dir inputs on fork flows
    fail as `AgentWrapperError::Backend { message: "add_dirs unsupported for codex fork" }` only
    after capability gating and pre-spawn validation succeed; malformed or invalid paths fail
    earlier as `InvalidRequest`. Canonical location:
    `docs/specs/codex-app-server-jsonrpc-contract.md`.
    - Produced by: `S3` with policy inputs prepared by `S1`.
- **Contracts consumed**:
  - `AD-C01`: Core add-dir extension key schema and bounds, owned by SEAM-1
    (`docs/specs/universal-agent-api/extensions-spec.md`).
    - Consumed by: `S1.T1` and `S1.T2` when advertising the key and attaching the shared helper.
  - `AD-C02`: Effective add-dir set algorithm, owned by SEAM-2.
    - Consumed by: `S1.T2` when Codex computes the effective working directory, calls
      `normalize_add_dirs_v1(...)`, and stores the resulting `Vec<PathBuf>`.
  - `AD-C03`: Safe error posture, owned by SEAM-1.
    - Consumed by: `S1.T2` for earlier `InvalidRequest` failures and `S3.T1` for the pinned safe
      backend rejection message.
  - `AD-C04`: Session-flow parity, owned by SEAM-1.
    - Consumed by: `S2.T1` to keep exec/resume behavior aligned and `S3.T1` to enforce the fork
      exception path without silently dropping accepted inputs.
  - `AD-C07`: Absence semantics, owned by SEAM-1.
    - Consumed by: `S1.T2` and `S2.T1` so missing keys yield `Vec::new()` and no emitted
      `--add-dir`.
- **Dependency edges honored**:
  - `SEAM-2 blocks SEAM-3`: this plan does not invent backend-local parsing; `S1` waits on the
    shared `normalize_add_dirs_v1(...)` contract and threads only its output.
  - `SEAM-3 blocks SEAM-5`: `S2` and `S3` establish the exact exec/resume mapping and fork
    rejection truth that SEAM-5 later pins in tests and regenerated artifacts.
- **Parallelization notes**:
  - What can proceed now: `S1` can land as soon as SEAM-2 exposes the shared helper; after that,
    `S2` and `S3` touch mostly separate files (`exec.rs` versus fork/harness/docs) and can be
    developed with low conflict.
  - What must wait: `S2` and `S3` both require `S1` to provide `policy.add_dirs`; SEAM-5 must wait
    for both produced contracts to stabilize before pinning coverage.
