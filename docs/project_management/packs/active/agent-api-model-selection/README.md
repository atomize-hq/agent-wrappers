# Universal model selection (`agent_api.config.model.v1`) — seam extraction (ADR-0020)

Source ADR: `docs/adr/0020-universal-agent-api-model-selection.md`  
Canonical owner spec: `docs/specs/universal-agent-api/extensions-spec.md`  
Backlog: `uaa-0002` (`bucket=agent_api.config`, `type=extension_key`)

This directory contains **seam** artifacts extracted to make the work owner-assignable and parallelizable without hiding coupling.
These files are planning aids; they are not normative contracts (authoritative contracts remain in `docs/specs/**`).

Note: ADR-0020 is currently **Draft**, but the model-selection semantics are already pinned in the universal extensions spec.
This pack treats the spec + ADR pair as the implementation target; any contract drift should be resolved in the canonical specs first,
then reflected here.

- Start here: `scope_brief.md`
- Seam overview: `seam_map.md`
- Threading (contracts + dependencies + workstreams): `threading.md`

## Canonical contracts (source of truth)

- Model-selection key semantics: `docs/specs/universal-agent-api/extensions-spec.md`
- Capability gating + error taxonomy: `docs/specs/universal-agent-api/contract.md`
- Run/event semantics for backend failures: `docs/specs/universal-agent-api/run-protocol-spec.md`
- Capability naming + schema posture: `docs/specs/universal-agent-api/capabilities-schema-spec.md`
- Prior promotion posture / pass-through boundaries: `docs/adr/0016-universal-agent-api-bounded-backend-config-pass-through.md`
