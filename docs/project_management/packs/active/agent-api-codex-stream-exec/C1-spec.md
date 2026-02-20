# C1 Spec — `agent_api` Codex Backend Refactor (`stream_exec` adoption)

Status: Draft  
Date (UTC): 2026-02-20  
Owner: agent-api-codex-stream-exec triad (C1)

## Scope (required)

Refactor the `agent_api` Codex backend to:

- use `codex::CodexClient::stream_exec` (typed `ThreadEvent` stream + completion) as the sole
  streaming source, and
- remove local spawn + stdout JSONL line ingestion from `agent_api`.

### In-scope deliverables

- `agent_api` Codex backend uses the adapter protocol in:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md`
- Preserve baseline universal invariants:
  - DR-0012 completion finality (completion waits for universal events stream finality)
  - event envelope bounds enforcement
  - raw backend line prohibition in v1
- Preserve env semantics:
  - request env overrides backend env keys
  - no parent env mutation
- Implement the exec-policy extension surface pinned in DR-0010:
  - validate supported keys + types before spawning
  - apply defaults deterministically (non-interactive by default; `workspace-write` sandbox by default)
  - map overrides into the Codex wrapper spawn configuration

### Out of scope (explicit)

- Changing the universal event envelope schema.
- Adding any new `AgentWrapperRunRequest.extensions` keys beyond those pinned in DR-0010.
- Improving event taxonomy beyond the pinned mapping in `contract.md` (e.g., emitting `ToolResult`
  for Codex in v1).

## Acceptance Criteria (observable)

- With `agent_api` built with `--features codex`:
  - The Codex backend still advertises:
    - `agent_api.run`
    - `agent_api.events`
    - `agent_api.events.live`
    - `backend.codex.exec_stream`
    - `agent_api.exec.non_interactive`
    - `backend.codex.exec.sandbox_mode`
    - `backend.codex.exec.approval_policy`
- The Codex backend no longer spawns `tokio::process::Command` directly for `codex exec` and no
  longer uses `BufReader(...).lines()` in `agent_api` for Codex JSONL ingestion.
- Redaction invariant holds:
  - no emitted universal error/event message contains raw JSONL lines from `ExecStreamError`
    (especially Parse/Normalize display strings).
- Env precedence invariant holds:
  - for overlapping keys, `request.env` wins over `config.env` for the spawned Codex process.
