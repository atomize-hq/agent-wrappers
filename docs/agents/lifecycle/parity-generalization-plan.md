# Universal parity acquisition & promotion — generalization plan

**Date:** 2026-07-24
**Base revision:** `origin/staging` @ `9400ee8e`
**Author:** lead orchestration session, from a read-only investigation of the shipped
xtask parity/maintenance system. Every claim below cites a verified `file:line` or a
committed artifact; the workstream-A design was dug into against repo truth before this
plan was written.
**Status:** **Implementation complete; three acceptance criteria remain gated on maintainer-run
CI.** Branch `feat/parity-acquisition-generalization`, off `origin/staging` @ `9400ee8e`, 6
commits, nothing pushed. `make preflight` green; 431 tests passing. Both required review lanes ran
and every finding is adjudicated (§11 Opus, §14 Codex).

What still needs a human: proving the multi-OS matrix on real runners, reconciling the two stuck
packets, and any promotion. None can be done from a laptop — they need macOS/Windows/ARM runners
and a push. See §13 for the checklist.

Map: §9 status · §10 reconciliations against repo truth · §11 Opus adjudication · §12 stuck
packets · §13 maintainer checklist · §14 Codex adjudication.

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
| **A2** — `acquisition` descriptor + planner | **done** | `acquisition` block in all three `RULES.json`; `xtask manifest-acquisition-plan`; 12 contract tests; every resolved URL verified live against the GitHub and npm registries |
| **A3** — reusable workflows | **done** | `parity-acquire.yml` + `parity-promote.yml` added; the 4 per-agent copies deleted |
| **B** — wire acquisition into the generic flow | **done** | `agent-maintenance-open-pr.yml` calls `parity-acquire` on the packet branch with `commit: true` |
| **C** — `opencode-snapshot` adapter | **done** | yargs parser + 9 unit tests; verified against the real 1.18.4 binary (62 commands, no omissions) |
| **D** — support-tier gate + onboarding | **done** | gate enforced by `manifest_acquisition::plan_for_agent`; entry rule documented in the charter and the registry contract |
| **E** — drift + stuck packets | **done, except the maintainer-gated runs** | engine rename landed in A1; `claude_code` duplicate `scope` key removed; opencode `RULES.json` normalized to the full schema; win32→windows-x64 mapping verified live. Stuck-packet reconciliation needs CI runners — see §12 |

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

## 11. Review adjudication (round 1, candidate `68f7fb27`)

Both required lanes ran in parallel against the same base/candidate. The Opus adversarial lane
returned 13 findings; each is adjudicated below. Two were already remediated by workstreams C and
E landing after the reviewed commit.

| # | Severity | Finding | Verdict |
| --- | --- | --- | --- |
| 1 | blocker | opencode's descriptor is live but `opencode-snapshot` did not exist and its `RULES.json` lacked `sorting` | **accepted, fixed** — C added the adapter; E normalized the manifest. Also added the reviewer's suggested guard: the plan job now verifies every descriptor-named subcommand exists *before* any download, plus tests for both |
| 2 | major | `mapfile` is a bash-4 builtin; GitHub macOS runners ship bash 3.2, so every macOS target would die | **accepted, fixed** — replaced with a portable `while read` loop and an explicit array init for `set -u` |
| 3 | major | the acquisition commit is pushed with `GITHUB_TOKEN`, which does not start new workflow runs, so the packet PR's CI never sees the acquired artifacts | **accepted, fixed** — the union job now checks out with `AUTOMATION_TOKEN` |
| 4 | major | npm acquisition self-attests its own download; the retired claude lane verified against an upstream-published checksum | **accepted, fixed** — npm tarballs are now verified against `dist.integrity` (or `dist.shasum`) before the digest is written to the lockfile |
| 5 | major | `DISABLE_AUTOUPDATER` had no home in the acquire lane, so a self-updating CLI could invalidate its own pin mid-capture | **accepted, fixed** — new `acquisition.snapshot.env`, applied before the binary is ever executed |
| 6 | major | promotion cannot match lockfile rows written by the retired lanes, and fails opaquely | **accepted, fixed** — explicit, actionable error naming the row it wanted and telling the maintainer to re-acquire; migration noted in the workflow header |
| 7 | major | the post-matrix push has no rebase/retry, so a branch move discards ~45 minutes of work | **accepted, fixed** — bounded fetch/rebase/retry loop |
| 8 | minor | `branch_created` can be true for a branch `create-pull-request` just deleted | **accepted, fixed** — gated on `pull-request-operation` |
| 9 | minor | path-shaped descriptor fields were unvalidated; the `expand` doc comment overclaimed | **accepted, fixed** — `safe_relative_path` rejects `..`, absolute paths and backslashes; comment corrected; the unused `SCRATCH_DIR_PLACEHOLDER` removed |
| 10 | minor | both `VALIDATOR_SPEC.md` files normatively specify `OK: codex-validate` | **accepted, fixed** — specs updated to the neutral string and note the alias prints it too |
| 11 | minor | operator runbooks route maintainers to deleted workflows | **accepted, fixed** — already rewritten in the docs pass; no stale references remain under `cli_manifests/**` or `docs/**` outside frozen historical evidence |
| 12 | minor | no `SCHEMA.json` for the new block; a malformed descriptor is caught only by a command CI never runs | **accepted with reconciliation** — see §10.3 for why a JSON Schema is the wrong instrument here. The real gap was coverage, so `ci.yml` now resolves every enrolled agent's descriptor on every PR |
| 13 | nits | shell interpolation, `secrets: inherit` breadth, build-vs-reject conflation, writer/reader truncation mismatch, `install` on Windows, packument size, `support-matrix` churn | **accepted and fixed**, except `support-matrix` churn, which is intended: publication should track acquisition |

Frozen historical evidence (`docs/agents/.uaa-temp/**`, `**/governance/proof/**`, closeout JSON,
ADRs, `PLAN.md`, `ORCH_PLAN.md`) deliberately still names the retired workflows and the `codex-*`
commands. Those are records of what happened, not instructions, and the back-compat aliases keep
every command in them working.

## 12. The two stuck packets

Both are **reconcilable through the new path but not completable locally**, because completing
them requires runners this machine does not have. Neither is hand-authored.

**codex `0.144.6`** — regenerating its union through the new engine reproduces the committed
artifact exactly: 2 of 4 targets, `complete:false`, missing `aarch64-unknown-linux-musl` and
`x86_64-pc-windows-msvc`. Those two targets need an ARM-Linux runner and a Windows runner. Running
`parity-acquire` for `codex 0.144.6` produces all four and a `complete:true` union.

**opencode `1.14.47`** — worse than partial. Its committed `union.json` was hand-produced by the
relay and carries `expected_targets` of **6** against the manifest's **3**, and its only input is
`darwin-arm64` while `required_target` is `linux-x64`. The engine refuses to build that union at
all, which is correct: `partial_union_policy.when_required_target_missing` is `fail`. There is no
local fix — the required target must actually be snapshotted. Running `parity-acquire` for
`opencode 1.14.47` regenerates the union from its own `RULES.json`, which resolves the 3-vs-6
drift as a side effect rather than by editing a committed artifact.

Deliberately **not** done: adding a validator gate asserting that a committed union's
`expected_targets` matches `RULES.json`. That check is correct and worth having, but it would make
`make preflight` red against opencode's committed union until the packet is re-acquired — i.e. it
would gate the repo on an action only the maintainer can take. It belongs in the follow-up that
lands after opencode `1.14.47` is reconciled.

## 13. Maintainer checklist (everything gated on a human)

Nothing below was performed by this session.

1. **Review and push the branch.** `feat/parity-acquisition-generalization`, branched from
   `origin/staging` @ `9400ee8e`. Nothing has been pushed.
2. **Open the PR to `staging`.**
3. **Confirm the `AUTOMATION_TOKEN` secret exists.** Finding 3 depends on it: without a PAT the
   acquisition commit will not re-trigger the packet PR's CI, and the maintainer would be
   reviewing a green check that never saw the acquired artifacts.
4. **Prove acquisition on a real runner** — the one acceptance criterion that cannot be verified
   locally, because it needs macOS/Windows/ARM runners:
   - `parity-acquire.yml` with `agent_id: codex`, `target_version: 0.144.6`, `commit: false`
   - `parity-acquire.yml` with `agent_id: claude_code`, a current stable version, `commit: false`
   Expect `complete: true` in the job summary for both.
5. **Reconcile the two stuck packets** (§12) by running `parity-acquire` with `commit: true`
   against each packet branch.
6. **Prove promotion readiness** with `parity-promote.yml`, `dry_run: true`. Do not set
   `dry_run: false` until the dry run is green and the union has been reviewed.
7. **Then, and only then, promote.** Promotion advances `latest_validated` and publishes support
   claims; it stays a human decision at every tier.

## 14. Review adjudication (Codex lane, candidate `68f7fb27`)

The Codex lane completed after the Opus lane and returned four findings, none overlapping. Its
`clippy` and `cargo test` legs could not run — its sandbox is a read-only worktree and both
failed trying to create `target/`. That verification gap is closed by the lead session's own runs
(`make preflight` green, 431 tests passing).

| # | Severity | Finding | Verdict |
| --- | --- | --- | --- |
| 1 | blocker | `ci.yml`'s claude validation job hardcodes `asset_name: claude` and executes the downloaded blob directly; after the first acquisition through the new lane claude's rows are npm tarballs, so it matches no row and would try to run a `.tgz` | **accepted, fixed** — the job resolves its coordinates from the acquisition descriptor and extracts `archive_member`, like every other consumer |
| 2 | major | promotion builds its matrix from `union.inputs` and never consults `union.promotion_policy` | **accepted, remedy adjusted** — the proposed hard-fail on `complete != true` would contradict codex's and claude's committed `allow_promote_when_incomplete: true` linux-first policy. The fix is to make the declared policy load-bearing: promotion fails on an incomplete union unless the manifest permits it, and an agent that declares no stance (opencode) does not get one |
| 3 | major | descriptor fields are unvalidated, so a manifest edit reaches runner execution; `validation.commands` runs through `eval` | **accepted in part** — path safety landed in the previous round (`safe_relative_path`); `runs_on` is now constrained to a plain runner label. The structured-command model is **deferred**: the threat model is committed, reviewed data, and the proportionate control is review gating, so the reviewer's own `CODEOWNERS` observation was adopted instead |
| 4 | major | the packet-opener gate cannot tell "not enrolled" from "malformed descriptor", so a real regression silently opens a packet PR without the complete union | **accepted, fixed** — the planner now exits `3` for a genuine gate miss and `1` for a real failure; the workflow fails on anything else |
| — | note | no `CODEOWNERS` protects `cli_manifests/*/RULES.json` | **accepted, fixed** — added, covering the descriptors, the registry, and the four lanes they drive |

### Local end-to-end proof of the acquisition chain

The one thing that genuinely cannot be verified without CI runners is the multi-OS matrix. Every
other step of the lane was executed for real on this machine's native target
(`claude_code` / `darwin-arm64` / `2.1.219`):

1. plan resolved from `RULES.json`
2. tarball downloaded (74,831,594 bytes)
3. **verified against the registry's published `sha512` `dist.integrity`** — the finding-4 fix
4. `package/claude` extracted per `archive_member`
5. executed under the descriptor's `DISABLE_AUTOUPDATER=1` snapshot env → `2.1.219 (Claude Code)`
6. `claude-snapshot` captured 49 commands

The `opencode` chain was proven the same way end-to-end through the engines
(`opencode-snapshot` → `manifest-union` → `manifest-report` → `manifest-version-metadata`) against
the real 1.18.4 binary.

## 15. Real-runner validation (2026-07-25)

The maintainer pushed the branch, confirmed `AUTOMATION_TOKEN` exists as a repository secret, and
authorized checklist items 2–4. Two things came out of attempting them.

### 15.1 The reusable lanes are not dispatchable until they reach `main`

`workflow_dispatch` only registers if the workflow file exists on the repository's **default
branch**. This repo's default branch is `main`, not `staging`, and both new lanes exist only on the
feature branch:

```
HTTP 404: workflow parity-acquire.yml not found on the default branch
HTTP 404: workflow parity-promote.yml not found on the default branch
```

So checklist items 2 (prove acquisition on runners), 3 (reconcile the stuck packets) and 4 (prove
promotion readiness) are blocked on merge order — `feat/… → staging → main` — not on anything in
the implementation. `only-staging-to-main.yml` guards the second hop, so both merges are human.

`agent-maintenance-open-pr.yml` is on `main` and `workflow_call` has no default-branch requirement,
so dispatching *it* at the feature ref looked like a way in. It is not: its `open-pr` job checks out
`ref: staging` unconditionally, so the gate step would run `staging`'s xtask, which has no
`manifest-acquisition-plan` command, and the run would fail before reaching the acquire job.

**Decision:** merge the chain first, then run items 2–4 as real dispatches. Recorded so the next
person does not re-derive the 404.

### 15.2 A real defect the runners caught that local proof could not

`ci.yml` *is* on `main` and does carry `workflow_dispatch`, so it can be dispatched at the feature
ref — running the feature branch's own definition. Doing so failed
`Claude Code Linux (latest validated)`:

```
manifest-acquisition-plan --agent claude_code --version 2.1.29
error: cli_manifests/claude_code/artifacts.lock.json has no row for
       claude_code_version=2.1.29 target=linux-x64 asset=claude-code-linux-x64-2.1.29.tgz
```

The round-2 remedy for the Codex BLOCKER took the **asset name from the descriptor** and then
demanded a lockfile row matching it. That is the wrong authority for an already-pinned version. The
descriptor says how a *new* version would be acquired; the lockfile row records what was *actually*
pinned. They disagree across the distribution migration: `2.1.29` is pinned as a bare `claude`
binary from the old storage bucket, while the descriptor resolves an npm platform tarball. So the
job demanded a row that does not exist and failed against committed truth.

The Codex finding was correct — hardcoding `asset_name: claude` would break once acquisition writes
tarball rows — but the remedy over-corrected into assuming the post-migration shape exclusively.

**Fix** (`4ea94956`): select the row by `(version, target)` and let it name its own asset, URL,
digest and size; infer archive shape from the pinned asset name. Target, binary path and snapshot
env still come from the descriptor, since those are version-independent. The job is now correct on
both sides of the migration. Guarded by
`c4_spec_ci_pins_the_latest_validated_binary_from_the_lockfile_row_not_the_descriptor`, which fails
against the old selector.

**Why local proof missed it:** the round-2 end-to-end proof used `2.1.219`, a version that exists on
npm, so the descriptor path worked. The committed `latest_validated` is `2.1.29`, which predates the
migration. Only CI, running against committed pointers, exercised that combination.

### 15.3 Status after the fix

`ci.yml` dispatched at the feature ref: **16 jobs green**, `Publish readiness` skipped as expected.
`make preflight` locally: **exit 0**.

One process note worth keeping: an earlier local preflight was reported green when it had in fact
exited 2 on the same `fmt-check` failure CI caught. The wrapper ended with `tail`, so the captured
exit status was `tail`'s. Local and CI never actually disagreed. Propagate the real exit code.
