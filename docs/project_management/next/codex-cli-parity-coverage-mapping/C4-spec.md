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

## CI Contracts (normative)

### Orchestration trigger

This triad extends the existing parity workflows rather than inventing parallel pipelines:
- Update `.github/workflows/codex-cli-update-snapshot.yml` to run the ADR 0002 pipeline end-to-end for a given upstream version.
- `codex-cli-release-watch.yml` dispatch behavior remains the trigger mechanism (candidate selection already implemented).

### Required jobs (v1)

`codex-cli-update-snapshot.yml` must:
1. Acquire pinned upstream binaries (from `cli_manifests/codex/artifacts.lock.json`) for the expected targets in `cli_manifests/codex/RULES.json.union.expected_targets`.
2. Generate per-target snapshots + raw help captures:
   - run `xtask codex-snapshot` once per available target
   - upload raw help under `cli_manifests/codex/raw_help/<version>/<target_triple>/**` as CI artifacts (not committed)
3. On Linux, run:
   - `xtask codex-union` to write `snapshots/<version>/union.json`
   - `xtask codex-wrapper-coverage` to write `wrapper_coverage.json`
   - `xtask codex-report` to write `reports/<version>/*.json`
   - `xtask codex-version-metadata` to write `versions/<version>.json` with `status=reported`
   - `xtask codex-validate` to hard-fail if artifacts violate `SCHEMA.json` or `RULES.json`
4. PR creation is best-effort; regardless of PR success, upload an artifact bundle containing:
   - `cli_manifests/codex/snapshots/<version>/**`
   - `cli_manifests/codex/reports/<version>/**`
   - `cli_manifests/codex/versions/<version>.json`
   - `cli_manifests/codex/wrapper_coverage.json`

### CI gating condition (to avoid breaking main before first baseline)

`ci.yml` must add a hard-fail validation job that runs only when the repo has entered the “committed artifacts” regime:
- condition (normative): run `xtask codex-validate` only when:
  - `hashFiles('cli_manifests/codex/versions/*.json') != ''`

Once at least one `versions/<version>.json` is merged, the validation job becomes active and must remain a hard gate.

## Acceptance Criteria
- Workflows produce (as artifacts or PR commits, depending on permissions) the committed artifact set for a new upstream version:
  - `snapshots/<version>/*.json`, `reports/<version>/*.json`, `versions/<version>.json`, and updated pointers (as applicable).
- CI fails hard if any committed artifact is schema-invalid or violates `RULES.json` invariants.
- CI retention pruning runs deterministically (mechanical keep-set) and never deletes pinned pointer versions.

## Out of Scope
- Expanding the target matrix beyond the minimal v1 expected targets (can be added later).
