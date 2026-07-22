# Agent maintenance & onboarding — workflow smoothing roadmap

**Date:** 2026-07-22
**Base revision:** `origin/staging` (`1ae6ee12`)
**Authored by:** lead orchestration session, from two read-only Codex discovery
lanes (profile `atomize_systems_azure`, sandbox `read-only`) plus the
`codex-maintenance` orchestration friction log. Every seam below was cited by a
discovery lane; a representative subset was independently spot-checked by the
lead at `1ae6ee12`.

This document is cross-cutting: the friction it catalogs is in the **xtask
`agent-maintenance` lifecycle tooling and its CI**, not in any single agent's
packet and not in the orchestration harness. It supersedes, in scope, the
narrower `codex-maintenance/governance/orchestration-friction-log.md`, whose
entries are folded in below by id (E1–E9, A1–A5).

## System framing

Agent maintenance is a **live, multi-agent production pipeline**, not a
codex-only experiment. Shared automated release-watch is enrolled for `codex`,
`claude_code`, and `opencode` (each has a completed or in-flight packet-PR
path). `gemini_cli` and `aider` are onboarded but maintenance-deferred;
`openhands` is approved but not enrolled.

The pipeline has been **effectively dead since ~2026-06-01**: the release-watch
CI has not produced a new packet PR in ~7 weeks, and a backlog of stale open
packet PRs accumulated (at evaluation: 6 codex, 12 claude_code, 13 opencode).
Root causes are E1 (fail-stop watcher) + E2 (silent CI failure) + E3 (no
supersession).

## Confirmed friction (all verified against `1ae6ee12`)

| id | Defect | Primary seam |
| --- | --- | --- |
| E1 | `maintenance-watch` aborts the whole multi-agent queue on one agent's fetch failure; no `--agent` filter | `crates/xtask/src/agent_maintenance/watch.rs:177-230,286-452` |
| E2 | release-watch CI has no failure surfacing (no `if: failure()`, artifact, summary, or alert) | `.github/workflows/agent-maintenance-release-watch.yml:16-91` |
| E3 | PR-open workflow never supersedes older same-agent packet PRs | `.github/workflows/agent-maintenance-open-pr.yml:69-134`; `watch.rs:226-229` |
| E4 | doc renderer emits no `execute-agent-maintenance --write --run-id ...` command block at all | `crates/xtask/src/agent_maintenance/docs.rs:242-273`; `execute.rs:132-138` |
| E5 | execute dry-run→write baseline hashes the whole workspace, so operator-owned governance edits break the handshake | `crates/xtask/src/agent_maintenance/execute/workflow.rs:74-76,195-224` |
| E6 | required target `x86_64-unknown-linux-musl` has no sanctioned local snapshot lane | `cli_manifests/codex/RULES.json:170-190`; `.github/workflows/codex-cli-*.yml` |
| E6a | derived `support_surface_audit` demands exact live equality; no "satisfied by completed run" state | `crates/xtask/src/agent_maintenance/request.rs:216-324`; `refresh.rs:133-145`; `closeout/validate.rs:104-120` |
| E6b | watcher `fetch_github_releases` reads only the first releases page (no pagination) | `watch.rs:286-337` (contrast GCS path `382-418`) |
| E7 | snapshot-discovery parsed wrapped description lines as phantom subcommands (**fixed** `7910680f`; lands on staging via #144) — regression test still missing | `crates/xtask/src/codex_snapshot/discovery.rs:370-385` |
| E7a | execute leaks orphaned unsandboxed nested `codex exec` processes; no process-group reaping | `crates/xtask/src/agent_maintenance/execute/runtime.rs:198-240` |
| E9 | execute write mode never reloads the request after runtime writes/gates, so audit staleness surfaces only at closeout | `execute/workflow.rs:152-224`; `closeout/validate.rs:104-120` |
| A1 | release-watch carries per-agent `dispatch_workflow` but hardcodes `workflow_id: agent-maintenance-open-pr.yml` | `agent-maintenance-release-watch.yml:60-90` |
| A2 | maintained agent's writable surface is the entire maintenance root, letting it rewrite its own HANDOFF/governance docs | `contract_policy.rs:346-368` |
| A3 | refresh allowlist duplicates the doc-renderer's path knowledge (second source of truth) | `refresh.rs:330-345`; `docs.rs:388-436` |
| A4 | GitHub release fetch is bare `curl -fsSL` — no user-agent, auth, or retry | `watch.rs:312-313,438-448` |
| A5 | ephemeral execute run packets are written inside the repo tree (`docs/agents/.uaa-temp/...`) | `execute/packet.rs:121-151` |

### Onboarding & generality (secondary)

- Onboarding itself is coherent (canonical operator guide; enrolled → runtime_integrated → published → closed_baseline). Gaps: the exact `onboard-agent` output inventory / lifecycle seed / manifest skeleton are code-first (not enumerated in docs); `openhands` is approved-but-not-enrolled with no status surface; there is no authoritative enrollment matrix distinguishing `shared_release_watch` / `manual_refresh_only` / `approved_not_enrolled`.
- The maintenance **relay host is hardwired to Codex**: the executor runs `codex exec --dangerously-bypass-approvals-and-sandbox` as the execution host even when maintaining other agents, and the packet prompt opens with `@codex` for everyone (`contract_policy.rs:18-24,196-229`; `execute/runtime.rs:198-240`). Snapshot/union/report tooling exists only for codex+claude; `codex-validate` and `EMBEDDED_RUNTIME_SUPPORT_FAMILIES = ["codex"]` are codex-branded (`derive.rs:21`).

## Remediation waves

| Wave | Tier | Fixes | Write surface |
| --- | --- | --- | --- |
| W1 watcher resilience | P0 | E1, E6b, A4 | `crates/xtask/src/agent_maintenance/watch.rs` (+ watch tests) |
| W2 CI surfacing + supersession | P0 | E2, A1, E3 | `.github/workflows/agent-maintenance-{release-watch,open-pr}.yml` |
| W3 audit lifecycle state | P1 | E6a + E9 (one defect) | `request.rs`, `refresh.rs`, `closeout/validate.rs`, `execute/workflow.rs` |
| W4 contract hygiene | P1 | E5, E4, A2, A3 | `execute/workflow.rs`, `docs.rs`, `contract_policy.rs`, `refresh.rs` |
| W5 runtime robustness | P2 | E7a, E7 test, E6 doc, A5 | `execute/runtime.rs`, discovery tests |
| W6 host-neutral generality | P3 | relay-host split, `manifest-validate` alias, de-`@codex`, generalize literal | `contract_policy.rs`, `codex_validate.rs`, `derive.rs` |
| W7 enrollment clarity | P3 | enrollment matrix, openhands disambiguation, onboard-agent surface manifest | registry + operator guide |

**Authorized scope (2026-07-22): P0 + P1** (W1–W4). W3 precedes W4 (shared
`execute/workflow.rs`, `refresh.rs`). W5–W7 deferred.

## Protocol

Fresh branch `maintenance/workflow-smoothing-p0p1` off `origin/staging`. One
bounded Codex implementation packet per wave with non-overlapping write sets;
independent waves parallelized. Every material candidate gets a parallel
Codex + Opus read-only review lane, lead adjudication, and a lead final
once-over before integration. The 0.144.6 packet branch (PR #144) is frozen and
independent of this campaign.
