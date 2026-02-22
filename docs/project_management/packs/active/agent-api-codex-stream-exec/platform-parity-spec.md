# Platform Parity Spec — Agent API Codex `stream_exec` Parity

Status: Draft  
Date (UTC): 2026-02-20  
Feature directory: `docs/project_management/packs/active/agent-api-codex-stream-exec/`

This spec defines the platform expectations for the Codex backend refactor and its validation
evidence. It is additive to the universal platform parity baseline.

## Baseline (referenced; not duplicated)

- `docs/project_management/next/universal-agent-api/platform-parity-spec.md`

## CI parity platforms (required)

- Linux: `ubuntu-latest`
- macOS: `macos-latest`
- Windows: `windows-latest`

## Gating rules (normative)

- Automated validation MUST NOT require a real Codex binary.
- Validation MUST use a fake-binary/fixture strategy (defined in `C2-spec.md`) that works on all
  three platforms.
- The Codex backend MUST remain correct under both LF and CRLF newline conventions.

## Platform-specific invariants (normative)

### Newlines and UTF-8

- The adapter MUST tolerate CRLF-delimited JSONL input (Windows).
- The adapter MUST treat a trailing `\\r` as whitespace for JSON parsing and MUST NOT leak raw
  lines containing CRLF into any universal error string.

### Process spawning and env

- Env precedence (request env overrides backend env) MUST hold on all platforms.
- The backend MUST NOT depend on platform-specific shell features to spawn the fake binary.

### Cancellation / kill behavior envelope

- Tests MUST NOT assume that process termination on drop is instantaneous or identical across OSes.
- Timeouts used in tests MUST include a bounded grace window for process cleanup.

## Required evidence (observable)

- `cargo test -p agent_api --features codex` passes on:
  - `ubuntu-latest`
  - `macos-latest`
  - `windows-latest`

## CI/CD enforcement (normative; removes ambiguity)

This feature’s cross-platform evidence MUST be produced on GitHub-hosted runners via:

- `.github/workflows/agent-api-codex-stream-exec-smoke.yml`

That workflow MUST:
- run the feature-local smoke scripts under `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/`
- include the repo’s “public API guard” job (no backend types in `agent_api` public API)
- run `make preflight` on Linux.
