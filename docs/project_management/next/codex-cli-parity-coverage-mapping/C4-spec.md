# C4-spec – CI Wiring (Snapshot → Union → Report → Validate)

## Scope
- Update CI/workflows to run the full ADR 0002 pipeline using the `xtask` commands from C0–C3:
  - Per-target snapshots (Linux/macOS/Windows) for an upstream version.
  - Union merge and conflict recording.
  - Wrapper coverage generation.
  - Coverage report generation and version metadata updates.
  - Deterministic validation (`xtask codex-validate`) as a required gate.
- CI must upload raw help captures as artifacts (not committed) per `RULES.json.storage.ci_artifacts.raw_help`.
- CI must be compatible with orgs that disallow workflow write perms:
  - PR creation remains best-effort.
  - Snapshot/report outputs must be available as workflow artifacts so maintainers can open PRs manually.

## Acceptance Criteria
- Workflows produce (as artifacts or PR commits, depending on permissions) the committed artifact set for a new upstream version:
  - `snapshots/<version>/*.json`, `reports/<version>/*.json`, `versions/<version>.json`, and updated pointers (as applicable).
- CI fails hard if any committed artifact is schema-invalid or violates `RULES.json` invariants.
- CI retention pruning runs deterministically (mechanical keep-set) and never deletes pinned pointer versions.

## Out of Scope
- Expanding the target matrix beyond the minimal v1 expected targets (can be added later).

