# Universal add-dir run extension — seam extraction (ADR-0021)

Source: `docs/adr/0021-universal-agent-api-add-dirs.md`

This directory contains **seam** artifacts extracted to make the work owner-assignable and
parallelizable without hiding coupling. These files are planning aids; they are not normative
contracts (authoritative contracts remain in the system-of-record specs).

- Start here: `seam_map.md` because it is the complete execution map for every seam output,
  required contract-doc update, generated capability artifact, and the folded SEAM-5 integration
  closeout.
- Then read: `threading.md` for the contract registry, dependency edges, selector-branch coverage
  map, and the same SEAM-5 integration closeout acceptance checks in execution order.
- Background + success criteria: `scope_brief.md`
