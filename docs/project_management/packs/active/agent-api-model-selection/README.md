# Universal model selection (`agent_api.config.model.v1`) - seam extraction

Source: `docs/adr/0020-universal-agent-api-model-selection.md`

This pack captures seam briefs, authoritative threading, pack-level review surfaces, seam-exit intent, and governance scaffolds. It defaults to seam-brief depth; seam-local decomposition exists only for the active and next seams under `threaded-seams/`.

- Start here: `scope_brief.md`
- Seam overview: `seam_map.md`
- Threading: `threading.md`
- Pack review surfaces: `review_surfaces.md`
- Governance: `governance/remediation-log.md`

Execution horizon (inferred):

- Active seam: `SEAM-3`
- Next seam: `SEAM-4`

Policy:

- only the active seam is eligible for authoritative downstream sub-slices by default
- the next seam may later receive seam-local review + slices, and only provisional candidate-subslice hints
- active and next seams must eventually terminate in a dedicated final `seam-exit-gate` slice once seam-local planning begins
- future seams remain seam briefs

Migration note:

- pre-v2.3 seam-local planning has been archived under `threaded-seams/_archive_pre_v2_3/`
- v2.3 seam-local planning exists only for the active and next seams

## Canonical authority + sync workflow

- `docs/specs/universal-agent-api/extensions-spec.md` is the canonical owner doc for `agent_api.config.model.v1` semantics.
- `docs/specs/universal-agent-api/capabilities-schema-spec.md` is the canonical registry entry for the same capability id, including its `agent_api.config.*` bucket placement and capability-advertising posture.
- `docs/adr/0020-universal-agent-api-model-selection.md` remains a **Draft** rationale/rollout ADR until implementation acceptance; it is contextual support for this pack, not the normative source of truth.
- When model-selection semantics or advertising rules change, edit the canonical specs first, then update ADR-0020 and this pack in the same change. Finish by running `make adr-fix ADR=docs/adr/0020-universal-agent-api-model-selection.md` so the ADR drift guard matches the synchronized text.
- Sync ownership stays with the ADR owner(s) named in ADR-0020; this pack should only restate what the canonical specs already pin.

SEAM-1 status: the canonical owner-spec semantics are already landed in
`docs/specs/universal-agent-api/extensions-spec.md`. Remaining SEAM-1 work in this pack is limited to
ADR-0020 sync, drift verification against `extensions-spec.md` plus
`docs/specs/universal-agent-api/capabilities-schema-spec.md`, and pack updates if a canonical-doc delta is opened.
SEAM-2 through SEAM-5 may begin once the SEAM-1 verification pass in
`seam-1-core-extension-contract.md` records `pass: no unresolved canonical-doc delta`; they are not waiting on a new
model-selection design decision.

- Verification pass owner: the ADR-0020 owner(s), or an explicitly delegated SEAM-1 assignee acting on their behalf.
- Verification scope: compare the canonical owner-doc section
  `docs/specs/universal-agent-api/extensions-spec.md` (`### agent_api.config.model.v1`), the canonical registry entry
  `docs/specs/universal-agent-api/capabilities-schema-spec.md` (`agent_api.config.model.v1`), ADR-0020 sections
  `Canonical authority + sync workflow`, `Decision (draft)`, `Validation and error model`, `Backend mapping`, and
  `Capability advertising`, plus this pack's SEAM-1/threading restatements.
- `no unresolved canonical-doc delta` means those sources agree on the capability id + bucket, trim/bounds semantics,
  absence behavior, exact InvalidRequest template, backend mapping boundaries, and advertising posture. Any mismatch
  reopens the gate until the canonical specs are updated first and the ADR + pack are synchronized in the same change.
- Recording rule: the passing run is recorded in `seam-1-core-extension-contract.md` under `## Verification record`,
  including the comparison scope, pass/fail result, and the synchronization reference downstream seams must cite before
  they start or merge work that depends on SEAM-1.
  Before the synchronized change set is committed or opened as a PR, that reference may be the recorded `git HEAD`
  plus an explicit working-tree delta note; once a commit or PR exists, the verification record MUST be updated to cite
  that commit/PR instead.

- Start here: `scope_brief.md`
- Seam overview: `seam_map.md`
- Threading (contracts + dependencies + workstreams): `threading.md`
 - Pack review surfaces (orientation only): `review_surfaces.md`

## Canonical contracts (source of truth)

- Model-selection owner doc: `docs/specs/universal-agent-api/extensions-spec.md`
- Capability registry entry for `agent_api.config.model.v1`: `docs/specs/universal-agent-api/capabilities-schema-spec.md`
- Generic inherited baselines used by this key:
  - `docs/specs/universal-agent-api/contract.md` for the crate-level `AgentWrapperError` / `AgentWrapperBackend`
    surface that model-selection failures flow through
  - `docs/specs/universal-agent-api/run-protocol-spec.md` for general capability-validation ordering and the
    post-spawn terminal `AgentWrapperEventKind::Error` rule
- Codex exec/resume model mapping + fork-rejection contract:
  - `docs/specs/codex-streaming-exec-contract.md`
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
- Claude Code model/session argv mapping:
  - `docs/specs/claude-code-session-mapping-contract.md`
- Prior promotion posture / pass-through boundaries: `docs/adr/0016-universal-agent-api-bounded-backend-config-pass-through.md`
