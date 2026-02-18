# Kickoff Prompt — CP1-ci-checkpoint (GitHub-hosted multi-OS)

## Scope
- Run the bounded CI checkpoint gates defined in:
  - `docs/project_management/next/universal-agent-api/ci_checkpoint_plan.md`
- This is a GitHub-hosted-only checkpoint (no self-hosted runners).

## Start Checklist
1. `git checkout feat/universal-agent-api && git pull --ff-only`
2. Read: `ci_checkpoint_plan.md`, `platform-parity-spec.md`, `session_log.md`, and this prompt.
3. Set `CP1-ci-checkpoint` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start CP1-ci-checkpoint`).

## Requirements
- Tested SHA: the HEAD commit on `feat/universal-agent-api` after `C2-integ` is completed.
- Trigger the dedicated workflow created in C0:
  - `.github/workflows/universal-agent-api-smoke.yml`
- The workflow MUST run the multi-OS smoke scripts on GitHub-hosted runners:
  - Linux: `docs/project_management/next/universal-agent-api/smoke/linux-smoke.sh`
  - macOS: `docs/project_management/next/universal-agent-api/smoke/macos-smoke.sh`
  - Windows: `docs/project_management/next/universal-agent-api/smoke/windows-smoke.ps1`
- Run Linux-only gate:
  - `make preflight`

## End Checklist
1. Record evidence in `session_log.md`:
   - tested SHA
   - workflow run ids/links (or command output if run manually)
   - per-OS pass/fail
2. Set `CP1-ci-checkpoint` status to `completed` in `tasks.json`.
3. Commit docs (`docs: finish CP1-ci-checkpoint`).
