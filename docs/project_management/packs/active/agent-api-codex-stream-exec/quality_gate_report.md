# Quality Gate Report — Agent API Codex `stream_exec` parity

Status: Draft  
Date (UTC): 2026-02-20  
Reviewer: <TBD>

This report is required before execution triads begin. It must be produced by a third-party reviewer
running the planning lint checklist. Until reviewed, this file MUST NOT claim acceptance.

RECOMMENDATION: FLAG FOR HUMAN REVIEW

## Evidence checklist (reviewer fills)

- [ ] All required cross-platform / decision-heavy artifacts exist:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/decision_register.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/manual_testing_playbook.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/linux-smoke.sh`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/macos-smoke.sh`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/windows-smoke.ps1`
- [ ] `decision_register.md` pins the minimum required A/B decisions listed in:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/spec_manifest.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/impact_map.md` (“Concrete follow-ups”)
- [ ] Smoke scripts are fixture/fake-binary only (no real Codex required) and run bounded commands:
  - `cargo test -p agent_api --all-features`
  - `cargo test -p agent_api --features codex`
  - Optional heavier runs are opt-in (not default).
- [ ] Cross-platform constraints are addressed explicitly (Linux/macOS/Windows) and do not require self-hosted runners.
- [ ] Repo guidance is respected:
  - `make preflight` is treated as a Linux-only gate (not required on macOS/Windows smoke).

## Alignment checklist (reviewer fills)

- [ ] This pack does not contradict ADR-0011: `docs/adr/0011-agent-api-codex-stream-exec.md`
- [ ] This pack aligns with baseline universal specs (referenced, not duplicated):
  - `docs/project_management/next/universal-agent-api/contract.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`
- [ ] Safety posture is pinned (no raw JSONL line / stderr leakage) and testable via the planned C2 slice.

## What remains to reach ACCEPT (non-exhaustive)

- Run the planning lint checklist and reconcile any contradictions found during review.
- Ensure the new feature smoke workflow exists and is referenced by the platform parity spec:
  - `.github/workflows/agent-api-codex-stream-exec-smoke.yml`
- Update `RECOMMENDATION:` to `ACCEPT` only after third-party review.
