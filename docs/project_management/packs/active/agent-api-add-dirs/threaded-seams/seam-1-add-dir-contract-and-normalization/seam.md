### Seam Brief (Restated)

- **Seam ID**: SEAM-1
- **Name**: Add-dir contract + normalization semantics
- **Goal / value**: Publish one unambiguous, backend-neutral contract for `agent_api.exec.add_dirs.v1` so downstream implementation seams consume a single canonical truth for schema, normalization, safe errors, absence semantics, and session-flow behavior.
- **Type**: integration
- **Slicing strategy**: Contract-first. This seam blocks every other seam in the pack, so the first slice publishes the core contract that SEAM-2 must implement, and the second slice pins the session-flow and drift-conformance rules needed by SEAM-3/4/5.
- **Scope**
  - In:
    - Confirm the normative owner-doc text in `docs/specs/universal-agent-api/extensions-spec.md`.
    - Pin the closed schema, bounds, normalization rules, safe `InvalidRequest` posture, absence semantics, and session-flow compatibility language for `agent_api.exec.add_dirs.v1`.
    - Keep ADR-0021 aligned with the normative owner doc and maintain its drift guard.
  - Out:
    - Shared parser/normalizer code placement.
    - Backend capability advertising or argv wiring.
    - Backend contract-doc edits owned by SEAM-3/4/5.
- **Touch surface**:
  - `docs/specs/universal-agent-api/extensions-spec.md`
  - `docs/adr/0021-universal-agent-api-add-dirs.md`
- **Verification**:
  - Spec review against `docs/project_management/packs/active/agent-api-add-dirs/threading.md`
  - `make adr-check ADR=docs/adr/0021-universal-agent-api-add-dirs.md`
  - `make preflight` only after downstream implementation seams land
- **Threading constraints**
  - Upstream blockers:
    - None inside the pack
  - Downstream blocked seams:
    - `SEAM-2`
    - `SEAM-3`
    - `SEAM-4`
    - `SEAM-5`
  - Contracts produced (owned):
    - `AD-C01`
    - `AD-C03`
    - `AD-C04`
    - `AD-C07`
  - Contracts consumed:
    - None from sibling seams; this seam only references external normative baselines such as `docs/specs/universal-agent-api/contract.md`.

### Slice index

- `S1` → `slice-1-core-contract-publication.md`: publish the blocking v1 owner-doc truth for schema, normalization, safe errors, and absence semantics.
- `S2` → `slice-2-session-parity-and-drift-conformance.md`: pin session-flow parity, keep ADR-0021 aligned with the owner doc, and verify the seam’s contract handoff is drift-safe.

### Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `AD-C01`: closed `agent_api.exec.add_dirs.v1` schema, bounds, trim/resolve/lexical-normalize/dedup semantics, and owner-doc location in `docs/specs/universal-agent-api/extensions-spec.md`; produced by `S1`.
  - `AD-C03`: safe `InvalidRequest` message templates for the key, owned by `docs/specs/universal-agent-api/extensions-spec.md`; produced by `S1`.
  - `AD-C04`: session-flow parity contract, including orthogonality to `agent_api.session.resume.v1` and `agent_api.session.fork.v1`, Claude apply-on-fork behavior, and the pinned Codex fork rejection boundary; produced by `S2`.
  - `AD-C07`: absence semantics stating no backend synthesizes directories or emits `--add-dir` when the key is absent; produced by `S1`.
- **Contracts consumed**:
  - None from sibling seams.
  - External evidence only:
    - `docs/specs/universal-agent-api/contract.md`: effective working directory definition referenced by `S1`.
    - `docs/specs/codex-app-server-jsonrpc-contract.md`: evidence for the pinned Codex fork rejection boundary referenced by `S2`.
    - `docs/specs/claude-code-session-mapping-contract.md`: evidence for Claude session-flow parity referenced by `S2`.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-2`: `S1` publishes the exact contract that the shared normalizer must implement; SEAM-2 should not start with unresolved schema or error-shape ambiguity.
  - `SEAM-1 blocks SEAM-3` and `SEAM-1 blocks SEAM-4`: `S2` publishes the session-flow rules those backend seams must preserve instead of reinterpreting resume/fork behavior locally.
  - `SEAM-1 blocks SEAM-5`: both `S1` and `S2` provide the assertions and safe-message shapes that tests and capability verification must lock down.
- **Parallelization notes**:
  - What can proceed now:
    - Drafting `extensions-spec.md` and ADR-0021 updates within the WS-CONTRACT touch surface.
    - Evidence gathering from backend contract docs without editing them.
  - What must wait:
    - SEAM-2 implementation should wait for `S1` to land.
    - SEAM-3/4 session-flow implementation and SEAM-5 regression coverage should wait for `S2` to land.
