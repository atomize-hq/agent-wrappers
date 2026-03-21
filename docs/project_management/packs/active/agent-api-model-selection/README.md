# Universal model selection (`agent_api.config.model.v1`) — seam extraction (ADR-0020)

Source ADR: `docs/adr/0020-universal-agent-api-model-selection.md`  
Canonical owner spec: `docs/specs/universal-agent-api/extensions-spec.md`  
Backlog: `uaa-0002` (`bucket=agent_api.config`, `type=extension_key`)

This directory contains **seam** artifacts extracted to make the work owner-assignable and parallelizable without hiding coupling.
These files are planning aids; they are not normative contracts (authoritative contracts remain in `docs/specs/**`).

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
