### Seam Brief (Restated)

- **Seam ID**: SEAM-1
- **Name**: Core extension key contract
- **Goal / value**: Keep `agent_api.config.model.v1` pinned to one verified universal contract so downstream seams can implement model selection against a single authoritative source of truth.
- **Type**: integration
- **Seam-local slicing strategy**: dependency-first. The normative design is already landed, so the smallest unblocker is a fresh verification/sync pass that either proves no canonical-doc delta remains or resolves that delta before downstream work proceeds.
- **Scope**
  - In:
    - verification that MS-C01 through MS-C04 remain correctly pinned across the canonical universal specs
    - canonical-doc clarification patches if the verification pass finds unresolved drift
    - ADR-0020 and pack synchronization after canonical truth is confirmed
    - publication of the latest verification record that downstream seams must cite
  - Out:
    - backend capability advertising or normalization implementation
    - Codex or Claude argv wiring
    - backend-specific runtime rejection logic beyond keeping the canonical docs aligned
- **Touch surface**:
  - `docs/specs/universal-agent-api/extensions-spec.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/run-protocol-spec.md`
  - `docs/adr/0020-universal-agent-api-model-selection.md`
  - `docs/project_management/packs/active/agent-api-model-selection/{README.md,scope_brief.md,threading.md,seam-1-core-extension-contract.md}`
- **Verification**:
  - compare the canonical owner section, registry entry, inherited error/run-lifecycle baselines, ADR sections, and pack restatements against MS-C01 through MS-C04
  - if drift is found, fix canonical docs first, then sync ADR/pack in the same change, rerun the comparison, and append the resulting pass/fail entry under `seam-1-core-extension-contract.md`
- **Threading constraints**
  - Upstream blockers: none
  - Downstream blocked seams: SEAM-2, SEAM-3, SEAM-4, SEAM-5
  - Contracts produced (owned): MS-C01, MS-C02, MS-C03, MS-C04
  - Contracts consumed: none from other seams; this seam uses the canonical universal docs as evidence inputs for its verification pass

### Slice Index

- `S1` -> `slice-1-canonical-drift-verification.md`: verify and, if needed, reconcile the canonical universal docs for MS-C01 through MS-C04.
- `S2` -> `slice-2-adr-pack-sync-and-gate-publication.md`: sync non-normative artifacts and publish the verification gate downstream seams must cite.

### Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `MS-C01`: universal extension-key definition for `agent_api.config.model.v1`; authoritative text lives in `docs/specs/universal-agent-api/extensions-spec.md` with registry anchoring in `docs/specs/universal-agent-api/capabilities-schema-spec.md`; S1 verifies and, if needed, reconciles the canonical wording.
  - `MS-C02`: absence semantics for the key; authoritative text lives in `docs/specs/universal-agent-api/extensions-spec.md`; S1 verifies that absence still preserves backend defaults everywhere the pack and ADR restate it.
  - `MS-C03`: pre-spawn validation schema and pinned `InvalidRequest` message; authoritative text lives in `docs/specs/universal-agent-api/extensions-spec.md` and inherited error taxonomy references in `docs/specs/universal-agent-api/contract.md`; S1 verifies the exact validation posture and S2 republishes it in synced planning docs.
  - `MS-C04`: backend-owned runtime rejection posture and terminal error-event rule; authoritative text lives across `docs/specs/universal-agent-api/extensions-spec.md`, `docs/specs/universal-agent-api/contract.md`, and `docs/specs/universal-agent-api/run-protocol-spec.md`; S1 verifies the cross-doc alignment and S2 records the gate that downstream seams depend on.
- **Contracts consumed**:
  - None from other seams. SEAM-1 is the producer seam for the contract set in this pack and uses canonical universal specs plus ADR/pack text only as evidence to verify or restate its own owned contracts.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-2`: S1 must finish with either reconciled canonical docs or a recorded pass before SEAM-2 can claim advertising/normalization work is unblocked.
  - `SEAM-1 blocks SEAM-3`: S2 publishes the synchronization reference that Codex mapping work must cite before merging.
  - `SEAM-1 blocks SEAM-4`: S2 publishes the synchronization reference that Claude mapping work must cite before merging.
  - `SEAM-1 blocks SEAM-5`: the tests seam may draft work earlier, but only the S2-published verification gate satisfies the blocker for implementation-adjacent assertions.
- **Parallelization notes**:
  - What can proceed now: S1.T1 can begin immediately because it has no upstream seam blockers; draft note-taking for S2 can happen in parallel as long as it does not claim the gate is satisfied.
  - What must wait: any final ADR/pack sync text, verification-record publication, or downstream seam unblock claims wait on S1 proving `pass: no unresolved canonical-doc delta`.
