<!-- generated-by: xtask close-agent-maintenance; owner: maintenance-control-plane -->

# Remediation log

## Request

- request ref: `docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml`
- request sha256: `27f3c0797b0ae6af9d8297d0d816d6c8e3cc730b72fbe84a6674f1aa0d9cad6a`
- request recorded at: `2026-07-22T00:14:53Z`
- request commit: `df0762ea109af1206b85c585a87030a8e3bbfe38`

## Trigger context

- detected_by: `.github/workflows/agent-maintenance-release-watch.yml`
- current_validated: `0.125.0`
- target_version: `0.144.6`
- latest_stable: `0.145.0`
- version_policy: `latest_stable_minus_one`
- source_kind: `github_releases`
- source_ref: `openai/codex`
- dispatch_kind: `packet_pr`
- dispatch_workflow: `agent-maintenance-open-pr.yml`
- branch_name: `automation/codex-maintenance-0.144.6`

## Resolved findings

- [registry_manifest_drift] The codex 0.144.6 packet materialized the version-scoped snapshots, coverage reports, union, version metadata, wrapper coverage, and lockfile provenance required by the live maintenance request (baseline 0.125.0 -> target 0.144.6).
  surfaces:
  - cli_manifests/codex/snapshots/0.144.6/aarch64-apple-darwin.json
  - cli_manifests/codex/snapshots/0.144.6/x86_64-unknown-linux-musl.json
  - cli_manifests/codex/snapshots/0.144.6/union.json
  - cli_manifests/codex/reports/0.144.6/coverage.aarch64-apple-darwin.json
  - cli_manifests/codex/reports/0.144.6/coverage.x86_64-unknown-linux-musl.json
  - cli_manifests/codex/reports/0.144.6/coverage.any.json
  - cli_manifests/codex/versions/0.144.6.json
  - cli_manifests/codex/wrapper_coverage.json
  - cli_manifests/codex/artifacts.lock.json
- [support_publication_drift] Support-matrix publication was regenerated to match the landed codex 0.144.6 manifest truth, including the deterministic passthrough evidence note derived from the committed 0.144.6 coverage passthrough_candidates.
  surfaces:
  - cli_manifests/support_matrix/current.json
  - docs/specs/unified-agent-api/support-matrix.md

## Deferred findings

- No deferred findings remain: Live drift for codex is clean (check-agent-drift --agent codex -> status: clean) after the 0.144.6 packet write, the bounded non-TUI passthrough surface, and green-gate validation (fmt, codex-validate, support-matrix --check, capability-matrix --check, capability-matrix-audit, make preflight). No maintenance drift findings remain deferred. The frozen support_surface_audit was reconciled via prepare-agent-maintenance/refresh-agent to the post-passthrough coverage: the 68 discovered upstream surfaces are now covered by the bounded passthrough surface, so required_uplifts_this_run is 0; only the two preexisting `codex completion` non-TUI rows stay deferred by the request (requires_new_architectural_seam; TODOS.md#close-codex-completion-maintenance-gap) as support-surface debt, not drift. Two out-of-envelope xtask follow-ups (snapshot-discovery parser fix; support-matrix passthrough-note tooling) are recorded in governance/maintainer-follow-up.md and governance/orchestration-friction-log.md.
