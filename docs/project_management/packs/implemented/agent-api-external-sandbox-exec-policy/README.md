# External sandbox execution policy (dangerous) — seam extraction (ADR-0019)

Source: `docs/adr/0019-universal-agent-api-external-sandbox-exec-policy.md`
Backlog: `uaa-0016` — "Universal: external sandbox execution policy (dangerous)"

This directory contains **seam** artifacts extracted to make the work owner-assignable and parallelizable without hiding coupling.
These files are planning aids; they are not normative contracts (authoritative contracts remain in `docs/specs/**`).

- Start here: `scope_brief.md`
- Seam overview: `seam_map.md`
- Threading (contracts + dependencies + workstreams): `threading.md`

## Canonical contracts (source of truth)

- Core extension key owner doc: `docs/specs/universal-agent-api/extensions-spec.md`
- Capability id naming + extension gating requirement: `docs/specs/universal-agent-api/capabilities-schema-spec.md`
- Run protocol validation timing/ordering: `docs/specs/universal-agent-api/run-protocol-spec.md`
