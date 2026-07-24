# Universal parity acquisition & promotion — generalization plan

**Date:** 2026-07-24
**Base revision:** `origin/staging` @ `9400ee8e`
**Author:** lead orchestration session, from a read-only investigation of the shipped
xtask parity/maintenance system. Every claim below cites a verified `file:line` or a
committed artifact; the workstream-A design was dug into against repo truth before this
plan was written.
**Status:** **In execution** on `feat/parity-acquisition-generalization` (branched from
`origin/staging` @ `9400ee8e`). See §9 for live status, §10 for reconciliations against repo
truth, and §11 for the maintainer-gated steps that remain.

---

## 1. Why this exists

The universal onboarding/maintenance **lifecycle** is agent-agnostic and works — `opencode`
proved it end-to-end through the generic packet-PR path
(`docs/agents/lifecycle/opencode-maintenance/governance/proof/workflow-dispatch-summary.md`:
*"resolved directly to the generic packet-PR opener … without introducing a bespoke
workflow surface"*).

What was **never generalized** is the **multi-target parity acquisition and promotion** —
the step that turns a detected upstream release into a *complete, validated, promotable*
manifest. It exists only as **per-agent workflow copies** (codex + claude) that drive
generic engines, and is **absent for opencode**. So the generic maintenance flow produces
a **single-host, partial-union, `status: reported`** packet for every agent, and promotion
cannot complete.

The same wall blocks both enrolled union-model agents that have been maintained:

| Agent | Target | `versions/<v>.json` status | Union | `latest_validated.txt` |
| --- | --- | --- | --- | --- |
| codex | `0.144.6` | `reported` | 2 of 4 (`complete:false`) | `0.125.0` |
| opencode | `1.14.47` | `reported` | 1 of 6 (`complete:false`) | `1.4.11` |

`opencode` is therefore **not** actually working end-to-end: it proved the generic
PR-creation + implementation path, then hit the identical promotion wall.

---

## 2. Current-state map (verified)

| Layer | State | Evidence |
| --- | --- | --- |
| Lifecycle orchestration (`maintenance-watch` → `agent-maintenance-open-pr` → `execute-agent-maintenance` → `refresh-agent` → `close-agent-maintenance` → `check-agent-drift`) | **Generic**, agnostic, opencode-proven | all keyed by `<agent_id>`; opencode proof above |
| Parity **engines** (`codex-union` / `codex-report` / `codex-validate` / `codex-version-metadata` / `codex-wrapper-coverage`) | **Already generic — only codex-*named*** | all take `--root` + read `<root>/RULES.json` (`codex_union.rs:22-25`, `codex_report.rs:16-20`, `codex_version_metadata.rs:21-25`); **opencode's own packet runs `codex-validate --root cli_manifests/opencode`** (`opencode-maintenance/HANDOFF.md`), claude's promote runs `codex-version-metadata --root cli_manifests/claude_code` (`claude-code-promote.yml:82-98`) |
| Release **watcher** | **Generic + hardened** | reads registry for all agents; fetch hardening (PRs #148/#151/#152) is agent-neutral, proven live on all 3 (`failed_agents:0`) |
| Multi-target **acquisition** | **Per-agent workflow copies; opencode absent; not wired into the flow** | `codex-cli-update-snapshot.yml` and `claude-code-update-snapshot.yml` are near-duplicates; no opencode equivalent; neither is called by `agent-maintenance-open-pr.yml` |
| **Promotion** (writes `latest_validated`, sets `status: validated`) | **Per-agent workflow copies; opencode absent** | `codex-cli-promote.yml:167-180`, `claude-code-promote.yml:76-97`; both are standalone `workflow_dispatch` |
| **Snapshot generation** (capture a CLI's surface) | **Per-CLI** | `codex-snapshot`, `claude-snapshot`; opencode's are hand-produced by the relay |

**Core insight:** the generic flow's executor runs on **one host**, so it can only snapshot
**one target** → a partial union → `reported`. Advancing to a complete-union `validated`
state needs multi-target acquisition + promotion, which today is disconnected legacy copies
(codex, claude) and missing (opencode). That single gap is the root of both stuck packets.

---

## 3. Design north star

One RULES-driven, agent-parameterized parity subsystem:

1. **Engines stay as-is** (already generic). Rename to neutral names with back-compat
   aliases — cosmetic, removes the codex-branding that has been causing confusion.
2. **One reusable acquisition workflow** + **one reusable promotion workflow**
   (`workflow_call`), parameterized by `agent_id`, that read a new per-agent **acquisition
   descriptor** to: build the target matrix → download each target's binary → snapshot each
   → union → report → validate → (promotion) advance pointers. Retire the 4 copies.
3. **Wired into the generic maintenance flow:** the watcher-opened packet PR triggers
   acquisition *in the runner* so the PR lands with the **complete** multi-target union (the
   full cross-platform detail to review); **promotion stays a maintainer-gated step**.

---

## 4. Workstream A — repo-truth findings (dug in for this plan)

The four workflows are **two near-duplicate pairs** driving the **same generic engines**.
The agent-specific delta is exactly **three data-only dimensions** (acquisition) plus **one**
(promotion):

| Dimension | codex | claude_code |
| --- | --- | --- |
| upstream metadata fetch | GitHub API `releases/tags/rust-v<ver>` (`codex-cli-update-snapshot.yml`) | **GCS** `…/claude-code-releases/<ver>/manifest.json` (`claude-code-update-snapshot.yml:74-77`) |
| per-target `{runner, asset_name, binary_path, extract}` | hardcoded `case` — 4 Rust triples incl. `ubuntu-24.04-arm` | hardcoded `case` — 3 targets, asset `claude`/`claude.exe` |
| version+target → download URL | `github.com/openai/codex/releases/download/<tag>/<asset>` | `<bucket>/<ver>/<target>/<asset>` |
| **(promote)** per-target validation command | full CODEX_E2E suite (`codex-cli-promote.yml:140-144`) | `cargo test -p unified-agent-api-claude-code` (`claude-code-promote.yml:69`) |

Everything else is structurally identical: compute matrix from
`RULES.json union.expected_targets`, download per target, build `artifacts.lock.json`, the
per-target snapshot job, and the `union → report → validate → version-metadata` job on the
generic engines.

**Two-source fact + decision (2026-07-24):** today watch-source ≠ acquisition-source for
`claude_code` — it **watches** npm `stable` (`registry: source_kind=npm_dist_tag`, migrated in
#148) but **acquires** binaries from the legacy **GCS bucket** (`.../claude-code-releases/<ver>/manifest.json`),
whose LIST 401s (the reason we moved *watch* to npm) even though object-GET still works.

**Decision: switch `claude_code` acquisition to npm too** (validated against the registry, not
assumed). npm carries the real per-platform binaries:
- `@anthropic-ai/claude-code@2.1.206` declares **8 platform `optionalDependencies`**
  (`@anthropic-ai/claude-code-{linux-x64,linux-x64-musl,linux-arm64,linux-arm64-musl,darwin-x64,darwin-arm64,win32-x64,win32-arm64}`).
- `@anthropic-ai/claude-code-linux-x64-musl` is a **267.8 MB** package (`os:[linux] cpu:[x64]`) — a
  real native binary, not a shim.
- The main package's `install.cjs` resolves the binary via `optionalDependencies` / `process.platform`
  with **no** `storage.googleapis` reference — the install path does not touch GCS.
- claude's committed target names (`linux-x64`, `darwin-arm64`, `win32-x64`) already **map 1:1** onto
  the npm platform packages, so only the *download dimension* changes; targets/matrix/runners stay.

This unifies claude's watch + acquire on npm, retires the flaky GCS dependency, and reduces the
generic workflow to two acquisition `source_kind`s (github_releases + npm) rather than three. The
descriptor still models watch and acquire as **independent** fields (codex acquire stays
`github_releases`), but claude's both become npm. `opencode` almost certainly resolves the same way
(open decision #5).

**Descriptor home:** `RULES.json` has **no acquisition section today** — `union` holds only the
target model (`required_target`, `expected_targets`, `platform_mapping`, `partial_union_policy`,
`promotion_policy`). The per-target runner/asset/URL mapping lives **only** in the workflow
`case` blocks. Proposed: a new **`acquisition`** block in each agent's `RULES.json`, e.g.

```jsonc
"acquisition": {
  "source_kind": "github_releases",              // github_releases | gcs_bucket | npm
  "metadata": { "url_template": "https://api.github.com/repos/openai/codex/releases/tags/{tag}",
                "tag_template": "rust-v{version}" },
  "targets": {
    "x86_64-unknown-linux-musl": { "runs_on": "ubuntu-latest", "asset_name": "codex-{target}.tar.gz",
      "binary_path": "./codex-{target}", "extract": true,
      "url_template": "https://github.com/openai/codex/releases/download/{tag}/{asset}" }
    // … one entry per expected_target
  },
  "validation_commands": [ "cargo test -p unified-agent-api-codex", "…" ]
}
```

The reusable workflow reads this; **no per-agent YAML remains**. The `npm` variant (claude, and
likely opencode) sets `source_kind: "npm"` with a per-target `url_template` of
`https://registry.npmjs.org/{plat_pkg}/-/{plat_pkg_base}-{version}.tgz` where `plat_pkg` is the
platform package (e.g. `@anthropic-ai/claude-code-{target}`); the snapshot step extracts the binary
from that tarball and runs it on the matching-OS runner (native execution is still required to
capture the CLI surface, so the multi-OS matrix stays regardless of source).

---

## 5. Workstreams

### A. Unify acquisition + promotion into reusable workflows *(load-bearing)*

- **Objective:** one `.github/workflows/parity-acquire.yml` (`workflow_call`) + one
  `parity-promote.yml`, agent-parameterized from a `RULES.json` `acquisition` block. Retire
  the four copies.
- **Target design:** matrix from `union.expected_targets`; per-target download from
  `acquisition.targets[t].url_template`; snapshot via the agent's snapshot command (see C);
  union/report/validate/version-metadata via the generic engines with `--root cli_manifests/<agent>`;
  promotion writes `latest_validated` + per-target pointers + `version-metadata --status validated`
  and runs `acquisition.validation_commands`.
- **Write-set:** `.github/workflows/parity-acquire.yml` (new), `.github/workflows/parity-promote.yml`
  (new); `cli_manifests/{codex,claude_code,opencode}/RULES.json` (+`acquisition`);
  `cli_manifests/*/SCHEMA.json` (schema for the new block); deprecate/delete
  `{codex-cli,claude-code}-{update-snapshot,promote}.yml`; docs.
- **Acceptance:** dispatch `parity-acquire` for codex `0.145.0` → complete 4-target union
  committed; `parity-promote` → `latest_validated` advances; renamed validate is green.
  Same for claude; opencode acquires its declared target set.
- **Depends on:** nothing (engines exist). **Blocks:** B.
- **Risks:** acquisition-source auth (GCS object-GET, GH release); macOS/Windows/ARM runner
  cost; `source_kind` branching correctness.

### B. Wire acquisition + promotion into the generic maintenance flow

- **Objective:** the watcher-opened packet PR runs `parity-acquire` in the runner so the PR
  lands with the complete union; promotion stays a maintainer-gated `parity-promote` dispatch.
- **Current truth:** `agent-maintenance-open-pr.yml` runs only `prepare-agent-maintenance` +
  `refresh-agent` (docs); it never calls acquisition. The parity workflows are standalone.
- **Target:** `open-pr` (or the release-watch fan-out) invokes `parity-acquire.yml` against
  the packet branch for union-model agents, gated by a new per-agent registry field (e.g.
  `maintenance.parity_acquisition = "reusable"`); docs-only agents are unaffected (empty ⇒
  no acquisition, current behavior preserved).
- **Write-set:** `.github/workflows/agent-maintenance-open-pr.yml`,
  `.github/workflows/agent-maintenance-release-watch.yml` (if fan-out triggers it),
  `crates/xtask/data/agent_registry.toml` (+field), `docs/specs/agent-registry-contract.md`.
- **Acceptance:** a watcher run for a stale union-model agent opens a PR that fills in with a
  complete union; docs-only agents unchanged.
- **Depends on:** A.

### C. Snapshot-generation generalization *(the per-CLI piece)*

- **Objective:** a sanctioned snapshot adapter for every union-model agent. `codex-snapshot`
  and `claude-snapshot` exist; **opencode has none** (its snapshots are hand-produced by the
  autonomous relay — fragile). Decide: per-agent `*-snapshot` commands vs. a generic
  `agent-snapshot` with per-CLI adapter modules.
- **Write-set:** `crates/xtask/src/<agent>_snapshot*` (or a generic command + adapters);
  RULES/registry wiring so the reusable workflow selects the right snapshot command.
- **Depends on:** parallelizable with A/B; **required** for opencode acquisition to stop being
  manual.

### D. Support-tier gating + onboarding integration

- **Objective:** define the support tier at which an agent enters multi-target
  acquisition/promotion, and route the **same** reusable acquisition into onboarding so a new
  agent's baseline is captured consistently.
- **Current truth:** tiers `bootstrap → baseline_runtime → publication_backed → first_class`
  in `lifecycle-state.json`; the onboarding baseline snapshot comes from the single-host
  `runtime-follow-on`; no tier gate on acquisition exists.
- **Write-set:** onboarding/lifecycle tooling (`runtime-follow-on`, publication), registry,
  `docs/specs/cli-agent-onboarding-charter.md`, `docs/cli-agent-onboarding-factory-operator-guide.md`.
- **Depends on:** A (reusable acquisition must exist to be called from onboarding).

### E. Cleanup / consistency

- Rename `codex-*` engines → neutral (`manifest-union` / `manifest-report` / `manifest-validate` /
  `manifest-version-metadata`) with back-compat aliases (roadmap W6).
- Reconcile opencode `RULES.json` `union.expected_targets` (**3**: `linux-x64`, `darwin-arm64`,
  `win32-x64`) vs its committed `snapshots/1.14.47/union.json` `expected_targets` (**6**) drift;
  normalize opencode's thin `RULES.json` to the full schema (it lacks `automation`,
  `version_metadata`, `report`).
- Reconcile the two stuck packets (codex `0.144.6`, opencode `1.14.47`) through the new path.
- **Depends on:** after A/B land (rename touches workflow references).

---

## 6. Sequencing

```
A ──▶ B
C ── (parallel to A/B; gates opencode acquisition)
D ── (parallel; depends on A for the callable acquisition)
E ── (last; rename + reconciliation after A/B are green)
```

A is load-bearing and unblocks both stuck packets. B delivers the maintainer-facing flow.
C removes opencode's manual snapshot dependency. D generalizes the entry gate + onboarding.
E is cleanup that must follow the rename-sensitive work.

---

## 7. Execution protocol

Fresh branch off `origin/staging`. One bounded Codex implementation packet per workstream with
non-overlapping write sets; independent packets parallelized. Every material candidate gets a
parallel Codex + Opus read-only review lane, lead adjudication, and a lead final once-over with
`make preflight` before integration — the same protocol used for W1–W4.

**Standing constraint:** the repository's maintenance tooling owns release detection, target
selection, validation, and promotion. This campaign **builds the tooling**; it does not
hand-author any agent version upgrade. Nothing is pushed, merged, or promoted without the
maintainer.

---

## 8. Decisions (settled 2026-07-24; maintainer may override)

1. **Acquisition-descriptor home** — a new **`acquisition` block in each agent's
   `cli_manifests/<agent>/RULES.json`** (+ `SCHEMA.json`), co-located with the `union` target model.
2. **Snapshot generation (C)** — follow the existing **per-agent `*-snapshot` adapter** pattern (add
   `opencode-snapshot` mirroring `codex-snapshot`/`claude-snapshot`). A generic command + adapter
   modules is optional later cleanup, not required for done.
3. **Support-tier gate (D)** — enable multi-target acquisition for any agent that is **enrolled in
   `release_watch` and carries an `acquisition` block**. (Revisit if a stricter tier gate is wanted.)
4. **The two stuck packets** — reconcile `codex 0.144.6` / `opencode 1.14.47` **through the new
   path**; the actual promote stays maintainer-gated (prove readiness, don't self-promote).
5. **Acquisition sources — settled for all three agents:** `codex = github_releases`,
   `claude_code = npm`, `opencode = npm`. Validated against the npm registry (below). The registry's
   stale `opencode` entry (`github_releases`/`anomalyco`) is corrected to npm as part of E.

### Validation evidence (npm as binary source)

- **claude_code → npm**: `@anthropic-ai/claude-code@2.1.206` declares 8 platform
  `optionalDependencies`; `@anthropic-ai/claude-code-linux-x64-musl` is a **267.8 MB** real binary;
  `install.cjs` resolves via `optionalDependencies`/`process.platform` with **no** `storage.googleapis`
  reference. claude target names map 1:1 to npm platform packages (only the download URL changes).
- **opencode → npm**: `opencode-ai@1.18.4` declares 12 platform `optionalDependencies`;
  `opencode-linux-x64` is a **178.9 MB** real binary. Naming nuance for E: opencode uses
  `opencode-windows-x64` (not `win32-x64`), so the descriptor's `url_template` maps target →
  platform-package name per agent.

---

## 9. Execution status

| Workstream | Status | Evidence |
| --- | --- | --- |
| **A1** — generalize + rename the engines | **done** | `manifest-union` reproduces the committed codex `0.144.6` and claude `2.1.29` unions byte-for-byte; `claude_union` deleted |
| **A2** — `acquisition` descriptor + planner | **done** | `acquisition` block in all three `RULES.json`; `xtask manifest-acquisition-plan`; 9 contract tests |
| **A3** — reusable workflows | **done** | `parity-acquire.yml` + `parity-promote.yml` added; the 4 per-agent copies deleted |
| **B** — wire acquisition into the generic flow | **done** | `agent-maintenance-open-pr.yml` calls `parity-acquire` on the packet branch with `commit: true` |
| **C** — `opencode-snapshot` adapter | in progress | delegated packet |
| **D** — support-tier gate + onboarding | gate done | the gate is enforced by `manifest_acquisition::plan_for_agent`; onboarding routing pending |
| **E** — drift + stuck packets | in progress | engine rename landed in A1; `claude_code` duplicate `scope` key removed |

### What actually changed the shape of the system

The generic flow's executor runs on one host, so it could only ever produce a partial,
`complete:false` union. Three things now close that gap:

1. **One engine, not one per agent.** `union.tool_name` and `union.raw_help_layout` moved the
   only two agent-specific behaviors out of the union engine and into manifest data.
2. **One descriptor, not four workflows.** `acquisition` describes *how to obtain* a release for
   every expected target; `manifest-acquisition-plan` resolves it into a matrix.
3. **One lane, called from the flow.** The watcher-opened packet PR now runs the full
   cross-platform matrix in the runner and commits the complete union onto the packet branch.

## 10. Reconciliations against repo truth

These correct claims in §§1–8 that did not survive verification. Where this section and an
earlier section disagree, this section is authoritative.

1. **§2 / §3.1 — "engines are already generic, only codex-*named*" was not accurate.**
   `codex_union` hardcoded the `codex-cli` tool name, and a near-verbatim `claude_union` copy
   existed solely because claude needs a different `raw_help` directory layout (a hashed single
   directory rather than nested path tokens). Renaming alone would have left opencode with no
   union engine at all. **Consequence:** the rename was pulled forward from E into A and turned
   into a real generalization. Both behaviors are now `RULES.json` data, `claude_union` is
   deleted, and one `manifest-union` serves every agent. Equivalence was proven by regenerating
   both agents' committed unions and diffing.

2. **§5.B — the proposed registry field `maintenance.parity_acquisition = "reusable"` was not
   added.** Settled decision §8.3 already defines the gate as *enrolled in `release_watch` **and**
   carries an `acquisition` block*, and `docs/specs/agent-registry-contract.md` forbids a second
   enrollment inventory outside the registry. A new enablement field would have been exactly that
   second inventory, and could disagree with the manifest. **Consequence:** the gate is enforced
   in `manifest_acquisition::plan_for_agent`, which fails closed with a distinct error for each
   reason (`UnknownAgent`, `NotEnrolled`, `NoAcquisitionBlock`). `agent-maintenance-open-pr` uses
   a clean planner exit as the gate, so committed truth is the only source.

3. **§8.1 — "(+ `SCHEMA.json`)" mis-identified the file.** `cli_manifests/*/SCHEMA.json` is the
   normative schema for the *artifacts* (snapshots, wrapper coverage, reports); the repo has no
   JSON Schema for `RULES.json` at all. **Consequence:** the acquisition block is schema-validated
   in tested Rust (`manifest_acquisition::descriptor`) instead. That is strictly stronger than a
   JSON Schema here, because it also cross-checks the descriptor against the same file's
   `union.expected_targets` / `union.required_target` — a constraint no standalone schema could
   express.

4. **New finding — `cli_manifests/claude_code/RULES.json` carried a duplicate `scope` key.**
   A dead `"scope": "claude-code-cli-parity"` string shadowed by the real `"scope": { … }` object
   later in the same file. Every parser silently took the last one. Removed, with a parse-equality
   assertion proving the effective document is unchanged.

5. **New finding — opencode has no wrapper-coverage generator.** `codex-wrapper-coverage` and
   `claude-wrapper-coverage` are byte-identical apart from importing their own wrapper crate's
   `wrapper_coverage_manifest` module, and `crates/opencode` has no such module. **Consequence:**
   `acquisition.wrapper_coverage_command` is optional; when absent the acquire lane keeps the
   committed `wrapper_coverage.json` and says so in the log. Giving opencode a generated
   wrapper-coverage manifest is follow-on work, not part of this campaign.

6. **The three `artifacts.lock.json` files have three different schemas** (`version` +
   `upstream_repo`; `schema_version` + `upstream`; `schema_version` + `inventory`) and three
   different per-row version keys (`codex_version`, `claude_code_version`, `semantic_version`).
   **Consequence:** `acquisition.lockfile_version_key` names the row key, and the reusable lane
   rewrites only `.artifacts`, so every committed lockfile keeps the exact top-level shape it
   already has. No committed artifact is migrated to a new schema by this campaign.
