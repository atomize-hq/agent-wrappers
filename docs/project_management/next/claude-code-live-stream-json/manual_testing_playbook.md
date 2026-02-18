# Manual Testing Playbook — Claude Code live stream-json

Status: Draft  
Date (UTC): 2026-02-18  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## Purpose

Provide non-gating, operator-run steps to validate “live” behavior using a real `claude` binary.

## Preconditions

- A working `claude` CLI installed and authenticated in your environment.
- A build of this repo on a commit that includes C1 integration (and CP1 checkpoint preferably).

## Steps (suggested)

1. Run a Claude request that produces multiple streamed events (long enough to observe live output).
2. Observe that at least one event arrives before the process exits.
3. Confirm `agent_api` completion does not resolve until the event stream is final (DR-0012).
4. Confirm no raw backend lines are surfaced in `AgentWrapperEvent.data` or parse-error messages.

## Record

Record results (time, SHA, command used, observations) in:
- `docs/project_management/next/claude-code-live-stream-json/session_log.md`

