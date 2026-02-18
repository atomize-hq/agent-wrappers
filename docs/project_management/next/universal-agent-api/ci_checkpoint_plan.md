# CI Checkpoint Plan — Universal Agent API

Status: Draft  
Date (UTC): 2026-02-16  
Feature directory: `docs/project_management/next/universal-agent-api/`

## Purpose

This plan defines the bounded CI checkpoints for the Universal Agent API feature so we do not run
redundant multi-OS CI on every triad while still getting deterministic cross-platform signal at
code-grounded seams.

Constraint (repo-specific):
- This repo must rely on **GitHub-hosted runners only** (no self-hosted behavior smoke gates).
- The CP1 checkpoint is implemented by a dedicated workflow created in C0:
  - `.github/workflows/universal-agent-api-smoke.yml`

## Checkpoint boundaries (code-grounded)

Current slice set (per ADR 0009 + `spec_manifest.md`):
- `C0`: core `agent_api` crate (types/traits/gateway/capabilities/events; no real backends)
- `C1`: Codex backend adapter (feature-gated)
- `C2`: Claude Code backend adapter (feature-gated)

Boundary choice:
- A single checkpoint after `C2` is within the default bounds (`min=2`, `max=4`) and ensures the
  cross-platform signal validates the *complete* end-to-end universal surface (core + both backends).

## Machine-readable plan (JSON)

```json
{
  "version": 1,
  "feature": "universal-agent-api",
  "min_triads_per_checkpoint": 2,
  "max_triads_per_checkpoint": 4,
  "slices": ["C0", "C1", "C2"],
  "checkpoints": [
    {
      "id": "CP1",
      "name": "Cross-platform compile+test parity (core + codex + claude backends)",
      "slice_group": ["C0", "C1", "C2"],
      "ending_slice": "C2",
      "checkpoint_task_id": "CP1-ci-checkpoint",
      "runner_mode": "github-hosted-only",
      "workflow": {
        "path": ".github/workflows/universal-agent-api-smoke.yml",
        "dispatch": "workflow_dispatch"
      },
      "ci_gates": {
        "compile_and_unit_test_matrix": {
          "os": ["ubuntu-latest", "macos-latest", "windows-latest"],
          "commands": [
            "linux: docs/project_management/next/universal-agent-api/smoke/linux-smoke.sh",
            "macos: docs/project_management/next/universal-agent-api/smoke/macos-smoke.sh",
            "windows: docs/project_management/next/universal-agent-api/smoke/windows-smoke.ps1"
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
        "depends_on_integration_task": "C2-integ",
        "blocks_next_slice_start": null
      }
    ]
  }
}
```

## Required `tasks.json` wiring (deterministic)

When authoring `docs/project_management/next/universal-agent-api/tasks.json`:

- Create an ops/integration-style task id: `CP1-ci-checkpoint`.
- Set `depends_on: ["C2-integ"]`.
- If future slices are added after `C2`, the first task of the next slice MUST depend on
  `CP1-ci-checkpoint` so no work proceeds past the checkpoint without the gate.

## Notes / constraints

- No self-hosted behavior smoke gates are planned; all gates must run on GitHub-hosted runners.
- If any slice introduces platform-specific behavior (PTY, path handling, env precedence), we may
  split into two checkpoints (e.g., `C0+C1` and `C2`) with explicit justification and updated JSON.
