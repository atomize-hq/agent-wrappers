# CODEX_CLI_PARITY (Triads)

Planning and execution runbook for ADR 0001.

Status: superseded for CI/workflow implementation details by ADR 0002 (`docs/adr/0002-codex-cli-parity-coverage-mapping.md`).
For the intended GitHub Actions pipeline (snapshots → union → reports → validate), use:
- `docs/project_management/next/codex-cli-parity-coverage-mapping/`

Start here:
- `docs/project_management/next/codex-cli-parity/plan.md`
- `docs/project_management/next/codex-cli-parity/tasks.json`
- `docs/project_management/next/codex-cli-parity/session_log.md`
- `docs/adr/0001-codex-cli-parity-maintenance.md`

Canonical triad process: `docs/project_management/task-triads-feature-setup-standard.md`.

Execution order (dependency DAG):
- C0 (snapshot tooling) → C1 (CI/workflows + version policy) → C2 (JSONL compat) → C3 (ops playbook).

Key non-obvious conventions (kept in-spec, repeated here for quick start):
- Upstream releases use tags like `rust-v0.77.0` (workflows accept bare semver inputs like `0.77.0`).
- Linux validation binaries are downloaded in CI/workflows from `openai/codex` release assets (e.g., `codex-x86_64-unknown-linux-musl.tar.gz`) and extracted to `./codex-x86_64-unknown-linux-musl` as a gitignored workspace artifact.

Local binary storage convention (recommended):
- Store multiple versions under `./.codex-bins/<version>/codex-x86_64-unknown-linux-musl` (directory is gitignored).
- Keep `./codex-x86_64-unknown-linux-musl` as a symlink to the “active” version when running local validated checks.
- Switch active version with: `ln -sfn .codex-bins/<version>/codex-x86_64-unknown-linux-musl codex-x86_64-unknown-linux-musl`
