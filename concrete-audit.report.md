# Concrete Audit Report

Generated at: 2026-03-12T19:34:17Z

## Summary
- Files audited: 49
- Issues: 4 total (blocker 1 / critical 1 / major 2 / minor 0)

### Highest-risk gaps
1. CA-0001 — Codex fork flow lacks a concrete add-dir transport or rejection contract
2. CA-0003 — Claude resume/fork argv contract does not pin where add-dir appears
3. CA-0002 — InvalidRequest error requirements remain too loose for stable add-dir tests
4. CA-0004 — Capability advertisement completion criteria omit the canonical generated artifact

## Issues

### CA-0001 — Codex fork flow lacks a concrete add-dir transport or rejection contract
- Severity: blocker
- Category: contract
- Location: `docs/specs/codex-app-server-jsonrpc-contract.md` L109-L117
- Excerpt: “Optional (subset used/pinned here): cwd, approvalPolicy, sandbox, persistExtendedHistory.”
- Problem: the canonical Codex fork transport defines no field that can carry accepted add_dirs, yet the universal extension spec requires fork flows to honor that set or fail safely. The current docs do not say which transport field or rejection boundary makes that possible.
- Required to be concrete:
  - Specify the exact Codex fork/turn request field or transport mechanism that carries accepted add_dirs, or explicitly state that Codex fork rejects add_dirs before the run handle is returned.
  - If rejection is intended, pin the exact error layer and error class.
  - State whether `thread/fork`, `turn/start`, or both must receive the effective add-dir set.
  - Align the Codex transport contract with the universal add_dirs fork requirement.
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/specs/universal-agent-api/extensions-spec.md:245-247`
  - `docs/project_management/packs/active/agent-api-add-dirs/scope_brief.md:113-114`

### CA-0002 — InvalidRequest error requirements remain too loose for stable add-dir tests
- Severity: major
- Category: language
- Location: `docs/specs/universal-agent-api/extensions-spec.md` L281-L284
- Excerpt: “InvalidRequest messages ... MUST be safe-by-default ... Backends SHOULD use stable, testable messages...”
- Problem: the canonical spec does not pin whether conformance is about exact strings, a stable message-id set, or some structured error shape. That leaves materially different safe messages compatible with the current text, even though the plan expects stable testable behavior.
- Required to be concrete:
  - Pin whether tests assert exact strings, finite message ids, or structured metadata.
  - If text is contractual, define the stable templates or allowed variants per failure class.
  - Define how the failing field/index is represented.
  - State whether failures may share one generic safe message or must remain distinguishable.
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-add-dirs/seam-2-shared-agent-api-normalizer.md:61-74`
  - `docs/project_management/packs/active/agent-api-add-dirs/seam-5-tests.md:23-27`
  - `docs/adr/0021-universal-agent-api-add-dirs.md:206-212`

### CA-0003 — Claude resume/fork argv contract does not pin where add-dir appears
- Severity: critical
- Category: contract
- Location: `docs/specs/claude-code-session-mapping-contract.md` L92-L123
- Excerpt: “Resume/fork mappings pin ordered subsequences ... but never mention `--add-dir`.”
- Problem: the plan requires Claude to pass one variadic `--add-dir <DIR...>` group for resume and fork flows, but the canonical backend contract never states where that group belongs relative to `--continue`, `--resume`, `--fork-session`, `--verbose`, and the final prompt token.
- Required to be concrete:
  - Pin the exact `--add-dir <DIR...>` placement for selector=`last` and selector=`id` in both resume and fork flows.
  - State whether the add-dir group appears before or after session-selection flags.
  - State the absence behavior explicitly in the Claude backend contract.
  - Update the contract examples so tests can assert one canonical argv shape.
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-add-dirs/seam-4-claude-code-mapping.md:14-15`
  - `docs/project_management/packs/active/agent-api-add-dirs/seam-4-claude-code-mapping.md:63-68`
  - `docs/specs/universal-agent-api/extensions-spec.md:289-292`

### CA-0004 — Capability advertisement completion criteria omit the canonical generated artifact
- Severity: major
- Category: testing
- Location: `docs/project_management/packs/active/agent-api-add-dirs/seam-5-tests.md` L61-L65
- Excerpt: “Verification only names cargo test -p agent_api, make test, and make preflight.”
- Problem: the feature’s done-state includes backend advertisement, but the verification plan never says whether the generated capability matrix must change or how to regenerate it. The current docs therefore do not tell an implementer how to prove the published support surface matches the code.
- Required to be concrete:
  - State whether `docs/specs/universal-agent-api/capability-matrix.md` must be updated in the same change.
  - Name the exact matrix-generation command.
  - Tie the acceptance criteria to the canonical matrix row for `agent_api.exec.add_dirs.v1`.
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-add-dirs/scope_brief.md:85-86`
  - `docs/specs/universal-agent-api/README.md:18-19`
  - `docs/specs/universal-agent-api/capability-matrix.md:23-27`

## Audited files
- See `concrete-audit.report.json` `meta.audited_files` for the full 49-file audited set.
