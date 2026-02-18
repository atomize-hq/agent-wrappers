# Platform Parity Spec — Universal Agent API

Status: Draft  
Date (UTC): 2026-02-16

This feature must validate on GitHub-hosted runners only.

## CI parity platforms (required)

- Linux: `ubuntu-latest`
- macOS: `macos-latest`
- Windows: `windows-latest`

## Gating rules

- All automated tests must be runnable without requiring installed Codex/Claude binaries.
- “Real binary” integration checks are allowed only as non-gating manual validation (see playbook).
- Linux-only repo gate `make preflight` remains required for integration tasks, but is not required on Windows.

## Required evidence

- At checkpoint `CP1`:
  - Workflow `.github/workflows/universal-agent-api-smoke.yml` passes on:
    - `ubuntu-latest`
    - `macos-latest`
    - `windows-latest`
  - Linux preflight (`make preflight`) passes for the same tested SHA.
