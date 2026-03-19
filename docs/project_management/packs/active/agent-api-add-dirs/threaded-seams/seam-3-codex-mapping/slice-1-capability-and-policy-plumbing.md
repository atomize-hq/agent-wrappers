### S1 — Capability advertising and normalized policy plumbing

- **User/system value**: exposes a deterministic Codex support surface for
  `agent_api.exec.add_dirs.v1` and ensures every later Codex flow consumes one shared normalized
  directory list instead of re-parsing raw extension payloads.
- **Scope (in/out)**:
  - In:
    - Advertise `agent_api.exec.add_dirs.v1` from Codex capability ids and supported-extension
      allowlists.
    - Extend the Codex policy model to carry a normalized `Vec<PathBuf>` add-dir list.
    - Resolve the effective working directory during
      `CodexHarnessAdapter::validate_and_extract_policy(...)` using the pinned precedence:
      request `working_dir` -> backend `default_working_dir` -> captured `run_start_cwd`.
    - Call `backend_harness::normalize::normalize_add_dirs_v1(...)` with the raw extension payload
      and the selected effective working directory, and store only the helper output on the
      policy.
    - Preserve absence semantics: missing key yields `Vec::new()`.
  - Out:
    - Exec/resume argv emission (S2).
    - Fork rejection behavior (S3).
    - Shared helper implementation details and filesystem validation semantics (SEAM-2).
    - Pack-level regression coverage (SEAM-5).
- **Acceptance criteria**:
  - `backend.capabilities().ids` and `supported_extension_keys()` include
    `agent_api.exec.add_dirs.v1` when the Codex backend advertises the feature.
  - `CodexExecPolicy` carries a typed `Vec<PathBuf>` for normalized add-dir state.
  - `validate_and_extract_policy(...)` computes the effective working directory once, passes the
    raw extension value plus that directory to `normalize_add_dirs_v1(...)`, and stores only the
    returned list.
  - When the key is absent, the policy carries an empty list and later slices emit no `--add-dir`.
  - No exec/resume/fork path rereads `request.extensions["agent_api.exec.add_dirs.v1"]` after
    policy extraction.
- **Dependencies**:
  - `SEAM-1`: AD-C01, AD-C03, AD-C07
  - `SEAM-2`: AD-C02
- **Verification**:
  - `cargo test -p agent_api codex`
  - Direct policy-extraction and capability assertions live under the Codex backend test surface;
    exhaustive behavior pinning remains in SEAM-5.
- **Rollout/safety**:
  - This slice is safe to land first because it only exposes the feature once the backend can
    deterministically validate and carry the normalized list.

#### S1.T1 — Advertise `agent_api.exec.add_dirs.v1` from Codex capability surfaces

- **Outcome**: Codex exposes the add-dir capability id and allowlists the extension key wherever
  the backend declares supported run extensions.
- **Inputs/outputs**:
  - Input: AD-C01/AD-C07 from `threading.md` and
    `docs/specs/universal-agent-api/extensions-spec.md`.
  - Output:
    - `crates/agent_api/src/backends/codex/backend.rs`
    - `crates/agent_api/src/backends/codex/policy.rs`
    - any supporting constant exports in `crates/agent_api/src/backends/codex/mod.rs`
- **Implementation notes**:
  - Add the stable capability id `agent_api.exec.add_dirs.v1` to `capabilities().ids`.
  - Add the same extension key to the Codex supported-extension allowlist so R0 gating admits the
    request before policy extraction.
  - Keep the advertising unconditional once the seam lands, matching the scope brief.
- **Acceptance criteria**:
  - A supported Codex backend reports `agent_api.exec.add_dirs.v1` in both capability ids and
    supported extension keys.
  - Older behavior for unrelated extension keys is unchanged.
- **Test notes**:
  - Run `cargo test -p agent_api codex`.
  - Prefer updating the existing Codex capability tests rather than creating a parallel test
    surface.
- **Risk/rollback notes**:
  - Moderate risk if enabled before validation/plumbing exists, so land alongside `S1.T2` or keep
    the branch unmerged until both tasks are complete.

Checklist:
- Implement: wire `agent_api.exec.add_dirs.v1` into Codex capability ids and supported extension
  key lists.
- Test: `cargo test -p agent_api codex`.
- Validate: confirm `backend.capabilities().ids` and `supported_extension_keys()` both contain the
  key.
- Cleanup: avoid introducing backend-local aliases or duplicate key constants.

#### S1.T2 — Extract the normalized add-dir set during Codex policy validation

- **Outcome**: Codex policy extraction resolves the effective working directory, delegates add-dir
  normalization to SEAM-2’s shared helper, and carries the normalized `Vec<PathBuf>` forward for
  exec/resume/fork decisions.
- **Inputs/outputs**:
  - Input: AD-C02 from `threading.md`; effective working directory rules from
    `docs/specs/universal-agent-api/contract.md`.
  - Output:
    - `crates/agent_api/src/backends/codex/harness.rs`
    - `crates/agent_api/src/backends/codex/policy.rs`
- **Implementation notes**:
  - Extend `CodexExecPolicy` with `add_dirs: Vec<PathBuf>`.
  - In `CodexHarnessAdapter::validate_and_extract_policy(...)`, compute the effective working
    directory from the request, backend defaults, and `run_start_cwd` before calling the helper.
  - Call `backend_harness::normalize::normalize_add_dirs_v1(request.extensions.get("agent_api.exec.add_dirs.v1"), effective_working_dir)`.
  - Keep invalid-path and malformed-payload failures inside the shared helper so Codex does not
    invent backend-local `InvalidRequest` semantics.
- **Acceptance criteria**:
  - Policy extraction returns a normalized add-dir list for valid inputs.
  - Invalid payloads, bounds failures, missing paths, and non-directory paths fail before spawn as
    `InvalidRequest`, not as backend errors.
  - The extracted policy is the only source of add-dir data for downstream Codex flows.
- **Test notes**:
  - Run `cargo test -p agent_api codex`.
  - Favor direct validation of `validate_and_extract_policy(...)` for the typed `add_dirs` output.
- **Risk/rollback notes**:
  - Moderate risk around working-directory precedence; mitigate by keeping the resolution logic in
    `validate_and_extract_policy(...)` and not duplicating it downstream.

Checklist:
- Implement: add `Vec<PathBuf>` to `CodexExecPolicy` and populate it during
  `validate_and_extract_policy(...)`.
- Test: `cargo test -p agent_api codex`.
- Validate: verify no exec/resume/fork code path rereads the raw extension payload after policy
  extraction.
- Cleanup: keep working-directory resolution and helper invocation in one place only.
