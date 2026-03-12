# Cohesion Audit Report

## Meta
- Generated at: 2026-03-12T19:34:17Z
- Files audited: 49
- Scan used: no

## Summary
- Total issues: 3
- By severity: blocker=0, critical=1, major=2, minor=0
- High-confidence cohesion breaks: 3

## Issue index
| ID | Severity | Confidence | Type | Subject | Files |
|---|---|---|---|---|---|
| CH-0001 | major | high | verification_gap | Capability advertisement verification for agent_api.exec.add_dirs.v1 | docs/project_management/packs/active/agent-api-add-dirs/scope_brief.md; docs/project_management/packs/active/agent-api-add-dirs/seam-5-tests.md; docs/specs/universal-agent-api/capability-matrix.md |
| CH-0002 | critical | high | traceability_gap | Canonical backend mapping docs for add_dirs | docs/project_management/packs/active/agent-api-add-dirs/seam-3-codex-mapping.md; docs/project_management/packs/active/agent-api-add-dirs/seam-4-claude-code-mapping.md; docs/specs/claude-code-session-mapping-contract.md; docs/specs/codex-app-server-jsonrpc-contract.md |
| CH-0003 | major | high | missing_bridge_step | Codex fork parity risk resolution | docs/project_management/packs/active/agent-api-add-dirs/scope_brief.md; docs/project_management/packs/active/agent-api-add-dirs/threading.md; docs/project_management/packs/active/agent-api-add-dirs/seam-3-codex-mapping.md |

## Issues

### CH-0001 — Capability advertisement verification for `agent_api.exec.add_dirs.v1`
- Severity: major
- Confidence: high
- Type: verification_gap
- Subject: Capability advertisement verification for agent_api.exec.add_dirs.v1
- Locations:
  - `docs/project_management/packs/active/agent-api-add-dirs/scope_brief.md:85-91` (primary) — both built-in backends advertise and honor the key
  - `docs/project_management/packs/active/agent-api-add-dirs/seam-5-tests.md:61-65` (dependent) — verification only names `cargo test -p agent_api`, `make test`, and `make preflight`
  - `docs/specs/universal-agent-api/capability-matrix.md:23-27` (reference) — the canonical generated `agent_api.exec` section currently contains only `agent_api.exec.non_interactive`
  - `docs/specs/universal-agent-api/README.md:18-19` (reference) — the capability matrix is a canonical generated artifact
- What breaks: the pack treats backend advertisement as part of done-ness, but no seam carries that requirement into the generated canonical artifact that records supported capabilities. The implementation thread can therefore stop with code-level tests passing while the published capability inventory remains stale.
- Missing links:
  - No seam explicitly regenerates `docs/specs/universal-agent-api/capability-matrix.md`.
  - No acceptance criterion says the new capability row must appear there for both backends.
  - No verification step names the generator command.
- Required to be cohesive:
  - Add a verification/task step that regenerates the capability matrix and records the expected add_dirs row.
  - Tie that step to SEAM-5 or WS-INT.
  - Reference the canonical capability artifact directly in the pack’s acceptance criteria.
- Suggested evidence order: docs → codebase → git history → decision

### CH-0002 — Canonical backend mapping docs for `add_dirs`
- Severity: critical
- Confidence: high
- Type: traceability_gap
- Subject: Canonical backend mapping docs for add_dirs
- Locations:
  - `docs/project_management/packs/active/agent-api-add-dirs/scope_brief.md:35-37` (primary) — map normalized directories into both built-in backends
  - `docs/project_management/packs/active/agent-api-add-dirs/seam-3-codex-mapping.md:53-62` (dependent) — touch surface lists only code files
  - `docs/project_management/packs/active/agent-api-add-dirs/seam-4-claude-code-mapping.md:53-60` (dependent) — touch surface lists only code files
  - `docs/specs/claude-code-session-mapping-contract.md:82-123` (reference) — current Claude resume/fork mapping contract does not mention add-dir
  - `docs/specs/codex-app-server-jsonrpc-contract.md:105-137` (reference) — current Codex fork/turn contract does not mention add-dir
- What breaks: the pack assigns backend mapping work without naming the canonical backend-owned documents that must change. That disconnect lets code and backend contracts drift, even though downstream readers will use those docs to understand actual Claude and Codex behavior.
- Missing links:
  - No backend seam names the canonical backend mapping doc it must update.
  - No seam states which doc owns Codex fork add-dir support or safe rejection.
  - No acceptance gate checks doc/code agreement for backend mapping semantics.
- Required to be cohesive:
  - Extend SEAM-3 and SEAM-4 to include the backend-owned canonical contract docs in their touch surface.
  - State which doc owns Codex fork semantics and which doc owns Claude argv placement.
  - Require those docs to be updated before the feature is complete.
- Suggested evidence order: docs → codebase → git history → decision

### CH-0003 — Codex fork parity risk resolution
- Severity: major
- Confidence: high
- Type: missing_bridge_step
- Subject: Codex fork parity risk resolution
- Locations:
  - `docs/project_management/packs/active/agent-api-add-dirs/scope_brief.md:111-118` (primary) — Codex fork transport parity is called out as a known risk
  - `docs/project_management/packs/active/agent-api-add-dirs/threading.md:71-85` (dependent) — critical path still flows directly into SEAM-3 implementation
  - `docs/project_management/packs/active/agent-api-add-dirs/seam-3-codex-mapping.md:72-77` (dependent) — de-risk plan says to spike fork transport first, but that spike is not a named prerequisite
  - `docs/specs/codex-app-server-jsonrpc-contract.md:105-137` (reference) — current transport has no add-dir hook
- What breaks: the plan recognizes the Codex fork path as the main uncertainty but never turns that uncertainty into an explicit decision step. Readers cannot tell when the transport question is supposed to be answered or what downstream seams depend on that answer.
- Missing links:
  - No named prerequisite output exists for the fork-transport decision.
  - No workstream owns the transport-contract update if support is possible.
  - No acceptance gate names the exact safe rejection path if support is impossible.
- Required to be cohesive:
  - Insert an explicit bridge step or prerequisite artifact for the Codex fork transport decision.
  - Feed that decision into backend docs, code, and tests.
  - Update the critical path so the decision is resolved before broad backend work proceeds.
- Suggested evidence order: docs → codebase → git history → decision

## Audited files
- See `cohesion-audit.report.json` `meta.files` for the full 49-file audited set.
