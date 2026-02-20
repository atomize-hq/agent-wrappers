# Manual Testing Playbook — Agent API Codex `stream_exec` parity

Status: Draft  
Date (UTC): 2026-02-20  
Feature directory: `docs/project_management/packs/active/agent-api-codex-stream-exec/`

This playbook is a non-gating manual validation path. Automated validation for this feature MUST
remain fixture/fake-binary based and run on GitHub-hosted runners without a real Codex install.

Inputs:
- ADR: `docs/adr/0011-agent-api-codex-stream-exec.md`
- Spec manifest: `docs/project_management/packs/active/agent-api-codex-stream-exec/spec_manifest.md`
- Decision register: `docs/project_management/packs/active/agent-api-codex-stream-exec/decision_register.md`

## Preconditions (optional)

- Rust toolchain installed (matches repo requirements).
- Optional: a real Codex CLI binary available (either on `PATH` or via `CODEX_BINARY`).
- A scratch `CODEX_HOME` and scratch working directory under a temp folder (avoid mutating your real Codex state).

## Playbook

### Step 1 — Fixture/fake-binary smoke (recommended; no real Codex needed)

Run the feature-local smoke script for your OS (these scripts create a temporary fake `codex`
binary and ensure required tests run):

- Linux: `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/linux-smoke.sh`
- macOS: `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/macos-smoke.sh`
- Windows: `docs/project_management/packs/active/agent-api-codex-stream-exec/smoke/windows-smoke.ps1`

### Step 2 — Targeted test runs (still fixture-based)

- `cargo test -p agent_api --all-features`
- `cargo test -p agent_api --features codex`

Optional (bounded by operator choice; only run when you explicitly want broad signal):
- `cargo test --workspace --all-targets --all-features`

### Step 3 — Optional real-binary sanity (only if you have Codex installed)

1. Set an isolated home (recommended):
   - Linux/macOS: `export CODEX_HOME="$(mktemp -d)"`
   - Windows (PowerShell): `$env:CODEX_HOME = (New-Item -ItemType Directory -Path (Join-Path $env:TEMP ("codex-home-" + [guid]::NewGuid()))).FullName`
2. Point at the real binary (if not on `PATH`):
   - Linux/macOS: `export CODEX_BINARY=/path/to/codex`
   - Windows: `$env:CODEX_BINARY = "C:\\path\\to\\codex.exe"`
3. Run a wrapper example (safe to run without mutating your real home):
   - This example automatically falls back to sample payloads if no Codex binary is found:
     - `cargo run -p codex --example stream_last_message`
   - If a real Codex binary is configured, provide a prompt to exercise streaming:
     - `cargo run -p codex --example stream_last_message -- "Summarize repo status"`

What to confirm (manual observation):
- Streaming produces at least one event before completion resolves (“live” evidence).
- `final_text` policy matches `decision_register.md` / `contract.md` (v1: always `None` for Codex runs).
- Error messages do not include raw JSONL lines or raw stderr/stdout content (redaction posture).

## Recording results

Record outcomes (commands + pass/fail + OS) in this feature’s `session_log.md` under the relevant
integration task once execution triads begin.
