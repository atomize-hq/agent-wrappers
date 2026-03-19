### S1 — Core contract publication

- **User/system value**: Downstream seams get one stable owner-doc contract for `agent_api.exec.add_dirs.v1` before any shared normalizer or backend mapping code is written.
- **Scope (in/out)**:
  - In:
    - Publish or tighten the normative `extensions-spec.md` text for the key’s schema, bounds, normalization rules, safe error posture, and absence semantics.
    - Align ADR-0021 anywhere it restates those same semantics.
  - Out:
    - Shared helper implementation in `crates/agent_api/src/backend_harness/normalize.rs`.
    - Codex or Claude argv mapping docs.
- **Acceptance criteria**:
  - `docs/specs/universal-agent-api/extensions-spec.md` explicitly covers `AD-C01`, `AD-C03`, and `AD-C07`.
  - The add-dir section is concrete enough that SEAM-2 can implement validation without inventing extra rules.
  - ADR-0021 no longer introduces weaker or conflicting language for the same semantics.
- **Dependencies**:
  - None inside the pack.
  - Evidence-only references:
    - `docs/specs/universal-agent-api/contract.md`
    - `docs/project_management/packs/active/agent-api-add-dirs/threading.md`
- **Verification**:
  - Review the `agent_api.exec.add_dirs.v1` section in `docs/specs/universal-agent-api/extensions-spec.md` against `AD-C01`, `AD-C03`, and `AD-C07`.
  - Review ADR-0021 for parity with the owner doc on schema, safe errors, and absence semantics.
- **Rollout/safety**:
  - Doc-only and additive.
  - Do not edit backend-owned docs or code in this slice.

#### S1.T1 — Publish the normative owner-doc contract for schema and normalization

- **Outcome**: `docs/specs/universal-agent-api/extensions-spec.md` states the full v1 meaning for the add-dir key without placeholders.
- **Inputs/outputs**:
  - Inputs:
    - `docs/project_management/packs/active/agent-api-add-dirs/threading.md`
    - `docs/project_management/packs/active/agent-api-add-dirs/seam-1-add-dir-contract-and-normalization.md`
    - `docs/specs/universal-agent-api/contract.md`
  - Outputs:
    - Updated `docs/specs/universal-agent-api/extensions-spec.md`
- **Implementation notes**:
  - Keep the schema closed.
  - State `dirs` bounds, per-entry trimming, relative-path resolution against the effective working directory, lexical normalization only, pre-spawn `exists && is_dir`, and order-preserving deduplication.
  - Preserve normative ownership in `extensions-spec.md`; do not push semantics down into the ADR.
- **Acceptance criteria**:
  - `AD-C01` is fully represented in one owner-doc section.
  - No required behavior is left as “per spec”, “etc.”, or implicit cross-reference shorthand.
- **Test notes**:
  - Manual spec review against `threading.md`.
- **Risk/rollback notes**:
  - Main risk is under-specifying path normalization or filesystem checks and forcing SEAM-2 to guess.

Checklist:
- Implement:
  - Update the `agent_api.exec.add_dirs.v1` owner-doc section in `docs/specs/universal-agent-api/extensions-spec.md`.
  - Tighten wording until the contract is directly implementable by SEAM-2.
- Test:
  - Compare the resulting text against `AD-C01` in `threading.md`.
- Validate:
  - Confirm no backend-specific argv details were added to this task beyond what the owner doc already normatively owns.
- Cleanup:
  - Remove any stale draft wording that suggests the contract is still undecided.

#### S1.T2 — Pin safe error posture and absence semantics

- **Outcome**: The owner doc and ADR align on safe `InvalidRequest` message shapes and on “no add-dir argv when absent”.
- **Inputs/outputs**:
  - Inputs:
    - `docs/project_management/packs/active/agent-api-add-dirs/threading.md`
    - `docs/specs/universal-agent-api/extensions-spec.md`
    - `docs/adr/0021-universal-agent-api-add-dirs.md`
  - Outputs:
    - Updated `docs/specs/universal-agent-api/extensions-spec.md`
    - Updated `docs/adr/0021-universal-agent-api-add-dirs.md`
- **Implementation notes**:
  - Reuse the exact safe templates from `AD-C03`.
  - Keep the ADR as rationale and derived plan only; any conflict resolves toward `extensions-spec.md`.
  - State absence semantics without adding backend-local fallback behavior.
- **Acceptance criteria**:
  - The only allowed `InvalidRequest` templates for this key are the three safe forms from `AD-C03`.
  - The absent-key path is explicitly documented as emitting no `--add-dir` behavior (`AD-C07`).
  - ADR text does not reintroduce raw-path echoing or synthetic defaults.
- **Test notes**:
  - Manual doc diff review.
- **Risk/rollback notes**:
  - Main risk is letting ADR language drift into a second source of truth.

Checklist:
- Implement:
  - Tighten the safe-error and absence-semantics text in `docs/specs/universal-agent-api/extensions-spec.md`.
  - Align ADR-0021 text that summarizes those rules.
- Test:
  - Verify the three safe-message templates match `threading.md` exactly.
- Validate:
  - Confirm the absent-key path is described as “no extra directories” and “no `--add-dir` emitted”.
- Cleanup:
  - Remove superseded draft phrasing from ADR-0021 if it implies optional or backend-specific defaults.
