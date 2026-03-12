# Universal model selection (`agent_api.config.model.v1`) — seam extraction (ADR-0020)

Source ADR: `docs/adr/0020-universal-agent-api-model-selection.md`  
Canonical owner spec: `docs/specs/universal-agent-api/extensions-spec.md`  
Backlog: `uaa-0002` (`bucket=agent_api.config`, `type=extension_key`)

This directory contains **seam** artifacts extracted to make the work owner-assignable and parallelizable without hiding coupling.
These files are planning aids; they are not normative contracts (authoritative contracts remain in `docs/specs/**`).

Note: ADR-0020 is currently **Draft**, but the model-selection semantics are already pinned in the universal extensions spec.
This pack treats the spec + ADR pair as the implementation target; any contract drift should be resolved in the canonical specs first,
then reflected here.

SEAM-1 status: the canonical owner-spec semantics are already landed in
`docs/specs/universal-agent-api/extensions-spec.md`. Remaining SEAM-1 work in this pack is limited to
ADR-0020 sync, drift verification against related universal specs, and pack updates if a canonical-doc delta is
opened. SEAM-2 through SEAM-5 may begin once that verification pass records "no unresolved canonical-doc delta";
they are not waiting on a new model-selection design decision.

- Start here: `scope_brief.md`
- Seam overview: `seam_map.md`
- Threading (contracts + dependencies + workstreams): `threading.md`

## Canonical contracts (source of truth)

- Model-selection key semantics: `docs/specs/universal-agent-api/extensions-spec.md`
- Capability gating + error taxonomy: `docs/specs/universal-agent-api/contract.md`
- Run/event semantics for backend failures: `docs/specs/universal-agent-api/run-protocol-spec.md`
- Codex exec/resume model mapping + fork-rejection contract:
  - `docs/specs/codex-streaming-exec-contract.md`
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
- Claude Code model/session argv mapping:
  - `docs/specs/claude-code-session-mapping-contract.md`
- Capability naming + schema posture: `docs/specs/universal-agent-api/capabilities-schema-spec.md`
- Prior promotion posture / pass-through boundaries: `docs/adr/0016-universal-agent-api-bounded-backend-config-pass-through.md`
