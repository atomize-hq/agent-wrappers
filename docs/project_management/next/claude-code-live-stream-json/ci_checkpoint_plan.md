# CI Checkpoint Plan — Claude Code live stream-json

Status: Draft  
Date (UTC): 2026-02-18  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## Purpose

This plan defines the bounded CI checkpoints for ADR-0010 so we do not run redundant multi-OS CI on
every triad while still getting deterministic cross-platform signal at code-grounded seams.

Constraint (repo-specific):
- CI gates for this feature must run on **GitHub-hosted runners** (no self-hosted requirement).
- The CP1 checkpoint is implemented by a dedicated workflow created during execution:
  - `.github/workflows/claude-code-live-stream-json-smoke.yml`

## Checkpoint boundaries (code-grounded)

Current slice set (per ADR-0010 + `spec_manifest.md`):
- `C0`: add a streaming `--print --output-format stream-json` API to `crates/claude_code` (no `agent_api` wiring)
- `C1`: wire streaming into `crates/agent_api` Claude backend + advertise `agent_api.events.live`

Boundary choice:
- A single checkpoint after `C1` is within the default bounds (`min=2`, `max=4`) and validates the
  complete end-to-end surface for “live Claude events” (wrapper streaming API + `agent_api` wiring).

## Machine-readable plan (JSON)

```json
{
  "version": 1,
  "feature": "claude-code-live-stream-json",
  "min_triads_per_checkpoint": 2,
  "max_triads_per_checkpoint": 4,
  "slices": ["C0", "C1"],
  "checkpoints": [
    {
      "id": "CP1",
      "name": "Cross-platform compile+test parity (claude_code streaming + agent_api wiring)",
      "slice_group": ["C0", "C1"],
      "ending_slice": "C1",
      "checkpoint_task_id": "CP1-ci-checkpoint",
      "runner_mode": "github-hosted-only",
      "workflow": {
        "path": ".github/workflows/claude-code-live-stream-json-smoke.yml",
        "dispatch": "workflow_dispatch"
      },
      "ci_gates": {
        "compile_and_unit_test_matrix": {
          "os": ["ubuntu-latest", "macos-latest", "windows-latest"],
          "commands": [
            "linux: docs/project_management/next/claude-code-live-stream-json/smoke/linux-smoke.sh",
            "macos: docs/project_management/next/claude-code-live-stream-json/smoke/macos-smoke.sh",
            "windows: docs/project_management/next/claude-code-live-stream-json/smoke/windows-smoke.ps1"
          ]
        },
        "integration_gate_linux_only": {
          "os": ["ubuntu-latest"],
          "commands": ["make preflight"]
        }
      }
    }
  ],
  "tasks_json_wiring": {
    "checkpoint_tasks": [
      {
        "id": "CP1-ci-checkpoint",
        "depends_on_integration_task": "C1-integ",
        "blocks_next_slice_start": null
      }
    ]
  }
}
```

## Required `tasks.json` wiring (deterministic)

When authoring `docs/project_management/next/claude-code-live-stream-json/tasks.json`:

- Create an ops/integration-style task id: `CP1-ci-checkpoint`.
- Set `depends_on: ["C1-integ"]`.
- If future slices are added after `C1`, the first task of the next slice MUST depend on
  `CP1-ci-checkpoint` so no work proceeds past the checkpoint without the gate.

## Notes / constraints

- This plan intentionally runs multi-OS gates only at the checkpoint; per-slice integration remains
  responsible for local gating (fmt + clippy + pinned crate tests + `make preflight` as listed in the feature plan/specs).
