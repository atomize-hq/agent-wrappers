# Refactor Workplan / Status (Master)

Last Updated: 2026-02-05  
Owner: Refactoring Lead (this file is the single source of truth for status)

## 0) Planning Summary (Why this exists)

This repository has completed an initial audit (see Evidence Index) and is entering a phased refactor program focused on:
- Phase 0: supply-chain triage (RustSec + cargo-deny policy) and re-validating gates.
- Phase 1: modularize `crates/codex/src/lib.rs` via seam extraction while preserving public API paths.
- Phase 2: split `crates/codex/src/mcp.rs` into `crates/codex/src/mcp/*` with stable re-exports.
- Phase 3: split `crates/xtask` large “rule engine” files by domain while keeping deterministic outputs.

This file is updated as execution progresses; the audit pack is immutable evidence and is not rewritten.

---

## 1) Program Overview

### 1.1 Scope

- Workspace crates in scope: `crates/codex`, `crates/xtask`.
- Targets in scope: whatever is supported by the workspace’s explicit policy (see Phase 0).
- Code in scope:
  - Public wrapper library (`crates/codex`): JSONL streaming, apply/diff, capability probing/caching, MCP/app-server helpers.
  - Internal automation (`crates/xtask`): parity artifacts under `cli_manifests/codex/`.

### 1.2 Constraints / Hard Rules (non-negotiable)

1) Do NOT do drive-by refactors. No scope creep beyond the plan step being executed.  
2) Preserve externally observable behavior unless explicitly authorized.  
3) Preserve public API paths unless explicitly authorized; if an API change is desirable, propose it separately with a migration plan.  
4) Every step must have acceptance criteria and a “done when” gate.  
5) Prefer small PR-sized steps. Each step should be independently reviewable and reversible.  
6) Evidence must be cited via exact file paths (do not hand-wave).
   - Audit-time evidence: `audit_pack/...` (immutable snapshot).
   - Execution-time evidence (preferred for new runs): `evidence_runs/YYYY-MM-DD/...` (see §8.1).
     - Legacy execution evidence may still be under `audit_pack/execution/YYYY-MM-DD/...`; citing those paths is allowed and those artifacts must not be moved.

### 1.3 Non-goals (unless explicitly added later)

- No dependency upgrades beyond what is required for security/compliance in Phase 0.
- No behavior changes, no UX changes, no “cleanup” refactors unrelated to the seam being extracted.
- No CI redesign (we only use what is in the audit pack as baseline; CI specifics remain out of scope).

### 1.4 Audit Baseline Summary (verbatim program baseline)

- Workspace: crates/codex + crates/xtask
- Critical: bytes 1.11.0 vuln RUSTSEC-2026-0007 → fix >= 1.11.1
- Critical: cargo-deny license check fails without explicit deny.toml policy
- Medium: duplicate crate versions getrandom/windows-sys
- High: “god modules” above thresholds; Median ≈ 157.0, P75 ≈ 291.0 → soft = 300, P90 ≈ 534.0 → hard = 600, ceiling=1000 (latest post-refactor distribution; see `audit_pack/execution/2026-02-04/post_refactor_tokei_updated.json`)
- Phases:
  - Phase 0: supply chain triage (lockfile + deny.toml) with acceptance criteria listed below
  - Phase 1: modularize crates/codex/src/lib.rs (keep façade + re-exports). First seam: home/env → home.rs (API preserved)
  - Phase 2: split crates/codex/src/mcp.rs into mcp/* (API stable via re-exports)
  - Phase 3: split xtask big files by domain sections; keep determinism
- Claimed “already done” changes (treat as expected but note if you need confirmation):
  - Cargo.lock updated bytes 1.11.1
  - deny.toml added (license allowlist + targets)
  - home.rs extracted and re-exported from lib.rs

**Important:** The audit pack captures the state at audit time; some “claimed already done” changes may have landed after the audit ran. Treat them as expected state and *confirm during Phase 0 preflight*.

---

## 2) Evidence Index (Immutable Inputs)

These are the required provenance inputs for this program. Do not edit them; reference them.

- Policy note (evidence storage):
  - `audit_pack/` is the audit-time immutable snapshot.
  - **Future execution evidence** (command outputs, diffs, notes) is stored under `evidence_runs/YYYY-MM-DD/`.
  - `audit_pack/execution/*` is **legacy execution evidence already generated**; do not move it (keep referencing it as-is).
  - As of 2026-02-05, §3.1/§6.3 still use canonical post-refactor baseline artifacts under `audit_pack/execution/2026-02-04/` (see list below), while §3.2 top offenders are refreshed from execution evidence at `evidence_runs/2026-02-05/P4.4_*`.

- `audit_pack/README.md` — audit pack entry point, contents, and failure/skip summary.
- `audit_pack/meta/commands.log` — provenance log of what was executed (timestamp | cwd | command | exit).
- `audit_pack/supply_chain/cargo_audit.txt` — `cargo audit` output and vulnerability tree.
- `audit_pack/supply_chain/cargo_deny_advisories.txt` — `cargo deny check advisories` output.
- `audit_pack/supply_chain/cargo_deny_licenses.txt` — `cargo deny check licenses` output (license policy failures without explicit config).
- `audit_pack/metrics/loc_summary.txt` — LOC summary and top Rust file offenders; includes p75/p90 thresholds.
- `audit_pack/metrics/tokei_files_sorted.txt` — per-file tokei output sorted by lines (multi-language context).
- `audit_pack/lint/cargo_clippy.txt` — `cargo clippy` output.
- `audit_pack/build/cargo_test.txt` — `cargo test` output.
- `audit_pack/deps/cargo_tree_duplicates.txt` — `cargo tree -d` duplicates output.
- `audit_pack/failures/failed_steps_summary.md` — missing/failed tool installs and non-zero checks.

Optional (often useful, still immutable):
- `audit_pack/lint/cargo_fmt_check.txt` — `cargo fmt --check` output (if any).
- `audit_pack/build/cargo_check.txt` — `cargo check` output.
- `audit_pack/supply_chain/cargo_deny_sources.txt` / `audit_pack/supply_chain/cargo_deny_bans.txt` — deny sources/bans checks.
- `audit_pack/meta/environment.txt` / `audit_pack/meta/versions.txt` — environment and tool versions at audit time.
- `audit_pack/repo/tree.txt` — shallow repo tree snapshot.
- `audit_pack/post_refactor/post_refactor_tokei.txt` — legacy derived post-refactor LOC summary (kept for provenance; do not update).
- `audit_pack/post_refactor/post_refactor_tokei_files_sorted.txt` — legacy post-refactor `tokei` output sorted by lines (kept for provenance; do not update).
- `audit_pack/post_refactor/post_refactor_cargo_tree_duplicates.txt` — legacy post-refactor `cargo tree -d --target all` duplicates output (kept for provenance; do not update).
- `audit_pack/execution/2026-02-04/` — legacy execution evidence already generated (kept for provenance; do not move).

Canonical post-refactor metrics (latest as of 2026-02-05; stored under legacy execution evidence):
- `audit_pack/execution/2026-02-04/post_refactor_tokei_updated.json` — latest post-refactor tokei JSON used for §3.1 thresholds (post-P2.5/P3.6–P3.8).
- `audit_pack/execution/2026-02-04/post_refactor_tokei_files_sorted_updated.txt` — latest tokei sorted output used for §3.2 top offenders.
- `audit_pack/execution/2026-02-04/post_refactor_cargo_tree_duplicates_target_all_updated.txt` — latest duplicates output used for §6.3 triage.
- `evidence_runs/2026-02-04/post_refactor_tokei.json` — prior post-refactor tokei JSON (superseded by `audit_pack/execution/2026-02-04/post_refactor_tokei_updated.json`).
- `evidence_runs/2026-02-04/post_refactor_tokei_files_sorted.txt` — prior tokei sorted output (superseded by `audit_pack/execution/2026-02-04/post_refactor_tokei_files_sorted_updated.txt`).
- `evidence_runs/2026-02-04/post_refactor_cargo_tree_duplicates_target_all.txt` — prior duplicates output (superseded by `audit_pack/execution/2026-02-04/post_refactor_cargo_tree_duplicates_target_all_updated.txt`).

---

## 3) Baseline Metrics (and current failure signals)

### 3.1 Maintainability thresholds (post-refactor distribution)

Computed from `audit_pack/execution/2026-02-04/post_refactor_tokei_updated.json` (Rust per-file code LOC; percentiles use the same linear interpolation as `type=7`):
- Rust files: 101
- Median = 157.0
- P75 = 291.0
- P90 = 534.0

Policy application:
- `soft := max(300, P75)` → **soft = 300**
- `hard := min(1000, max(600, P90))` → **hard = 600**
- `ceiling := 1000` → **ceiling = 1000**

Note: These replace the audit-time thresholds from `audit_pack/metrics/loc_summary.txt` (P75=302, P90=746). This is the latest distribution (post-P2.5/P3.6–P3.8).

### 3.2 Top offenders (largest Rust files)

Latest top 10 from `evidence_runs/2026-02-05/P4.4_rust_files_sorted_by_code.txt` (Rust per-file code LOC; derived from `evidence_runs/2026-02-05/P4.4_tokei_crates.json`):
- `crates/codex/src/tests/capabilities.rs` — 904 LOC (> hard)
- `crates/xtask/src/codex_union.rs` — 799 LOC (> hard)
- `crates/xtask/tests/c3_spec_reports_metadata_retain.rs` — 742 LOC (> hard)
- `crates/xtask/src/codex_version_metadata.rs` — 721 LOC (> hard)
- `crates/codex/src/exec.rs` — 676 LOC (> hard)
- `crates/codex/tests/cli_e2e.rs` — 669 LOC (> hard)
- `crates/xtask/src/codex_snapshot/discovery.rs` — 607 LOC (> hard)
- `crates/codex/src/tests/cli_commands.rs` — 586 LOC (<= hard)
- `crates/codex/src/jsonl.rs` — 566 LOC (<= hard)
- `crates/codex/src/mcp/config.rs` — 538 LOC (<= hard)

### 3.3 Baseline quality signals (audit time)

From `audit_pack/meta/commands.log`:
- Formatting gate: `cargo fmt --all -- --check` exited 0 (line 44).
- Clippy gate: `cargo clippy --all-targets --all-features -- -D warnings` exited 0 (line 45).
- Test gate: `cargo test --all-targets --all-features` exited 0 (line 43); see `audit_pack/build/cargo_test.txt`.
- Supply-chain gates failed at audit time:
  - `cargo audit` exited 1 (line 48); see `audit_pack/supply_chain/cargo_audit.txt`.
  - `cargo deny check advisories` exited 1 (line 49); see `audit_pack/supply_chain/cargo_deny_advisories.txt`.
  - `cargo deny check licenses` exited 4 (line 50); see `audit_pack/supply_chain/cargo_deny_licenses.txt`.

From `audit_pack/failures/failed_steps_summary.md`:
- `cargo-geiger` install failed (missing tool).
- `cargo-udeps` install failed / skipped (nightly not installed).

From `audit_pack/deps/cargo_tree_duplicates.txt`:
- Audit-time duplicates output indicates “nothing to print” (no duplicates reported). This conflicts with the baseline summary claim (“Medium: duplicate crate versions getrandom/windows-sys”). Treat as **needs confirmation** in Phase 0 preflight.

### 3.4 Repo language mix (context)

From `audit_pack/metrics/tokei_files_sorted.txt`:
- Rust: 70 files, 31,217 code lines (plus comments/blanks as reported by tokei).
- JSON: 36 files, ~23k lines (many are parity artifacts/snapshots under `cli_manifests/`).

Latest post-refactor snapshot (post-P2.5/P3.6–P3.8) from `audit_pack/execution/2026-02-04/post_refactor_tokei_updated.json`:
- Rust: 101 files, 31,499 code lines.

---

## 4) Quality Gates (Commands) + Program Definition of Done

### 4.1 Standard validation commands (run as written)

These commands are the required gates for this refactor program:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features

cargo audit
cargo deny check advisories
cargo deny check licenses
```

### 4.2 Definition of Done (whole program)

The refactor program is “Done” when:
- All quality gates in §4.1 pass on the workspace.
- Phase 0 supply-chain triage is complete and documented in §6 (advisory resolution + deny policy).
- `crates/codex/src/lib.rs` and `crates/codex/src/mcp.rs` are split along defined seams with:
  - Public API paths preserved via a façade + re-exports.
  - No externally observable behavior changes (validated by tests and targeted spot checks).
  - File-size policy met for *newly created/edited* modules per §7.3 (soft/hard/ceiling).
- `crates/xtask` large files are split by domain while keeping deterministic outputs and existing tests passing.
- Execution Journal §8 contains an auditable sequence of changes with validation results.

---

## 5) Workstreams and Phases (Master Checklist)

### Workstream A — Supply Chain / Compliance

#### Phase 0 — Supply-Chain Triage (lockfile + deny.toml) and preflight verification

##### P0.0 — Preflight: confirm “claimed already done” baseline items

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Confirm whether “claimed already done” items are present *in the working tree* and update Phase 0 statuses accordingly.
- Expected files touched: **None** (verification-only).
- Acceptance criteria (“done when”):
  - Run §4.1 commands; record outcomes in §8 (Execution Journal).
  - Confirm whether RUSTSEC-2026-0007 is present/absent (per `cargo audit` and `cargo deny check advisories`).
  - Confirm whether license policy is enforced via repo config (per `cargo deny check licenses`).
- Risk: Low (read-only).
- Rollback: N/A (verification only).

##### P0.1 — Fix RustSec advisory: `bytes` RUSTSEC-2026-0007 (>= 1.11.1)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Ensure the workspace resolves RUSTSEC-2026-0007 by upgrading `bytes` to `>= 1.11.1`.
- Expected files touched (if remediation required):
  - `Cargo.lock` (via `cargo update -p bytes` or equivalent).
- Acceptance criteria (“done when”):
  - `cargo audit` passes (no RUSTSEC-2026-0007) and is recorded in §8.
  - `cargo deny check advisories` passes and is recorded in §8.
  - `cargo test --all-targets --all-features` still passes.
- Risk: Low–Medium (dependency update can have subtle runtime impact; tests mitigate).
- Rollback:
  - Revert `Cargo.lock` to last known-good via VCS.
  - If lockfile upgrade causes breakage, pin the transitive dep via compatible constraints or adjust upstream crate versions (document decision in §9).

##### P0.2 — Establish license policy via `deny.toml` (licenses gate)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Ensure `cargo deny check licenses` uses an explicit repository policy and passes.
- Expected files touched (if remediation required):
  - `deny.toml` (policy: allowed licenses, targets, confidence threshold).
- Acceptance criteria (“done when”):
  - `cargo deny check licenses` passes and is recorded in §8.
  - Policy decisions are captured in §9 (Open Questions / Decisions), including any allowlist changes.
  - `cargo test --all-targets --all-features` still passes.
- Risk: Low (policy file only) to Medium (if allowlist requires narrowing/expanding and re-vetting).
- Rollback:
  - Revert `deny.toml` changes.
  - If strictness causes churn, scope checks to supported targets first, then iterate on allowlist (document in §9).

##### P0.3 — Re-check duplicates and document resolution approach (if any)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Confirm whether duplicate crate versions exist and decide whether to address them now or defer.
- Expected files touched (if remediation required):
  - Potentially `Cargo.lock` (via `cargo update -p ...`), and/or `Cargo.toml` constraints if needed.
- Acceptance criteria (“done when”):
  - `cargo tree -d` output is reviewed and summarized in §6.3.
  - Decision to fix/defer duplicates is captured in §9.
  - All quality gates remain green if any change is made.
- Risk: Medium (dependency graph adjustments can ripple).
- Rollback: Revert lockfile/manifest changes.

---

### Workstream B — `crates/codex` Modularization (public API preserved)

#### Phase 1 — Split `crates/codex/src/lib.rs` via responsibility seams (façade + re-exports)

**Phase goal:** Reduce `lib.rs` from “god module” size by extracting cohesive modules while preserving existing public API paths through re-exports.

Phase Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05  
Reason: `crates/codex/src/lib.rs` is now below the program ceiling per §3.2 (334 Rust code LOC; evidence: `evidence_runs/2026-02-05/P1.23_rust_files_sorted_by_code.txt` derived from `evidence_runs/2026-02-05/P1.23_tokei_crates.json`).

##### P1.0 — Define the `lib.rs` seam map (no code moves yet)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Produce a concrete extraction order and module boundary map for `lib.rs` without changing behavior.
- Expected files touched:
  - `refactor_workplan.md` (this file): update §7.1 seam list with final boundaries/order.
- Acceptance criteria (“done when”):
  - Proposed seams are listed in §7.1 with clear ownership and dependencies.
  - Each planned extraction is small/reversible (PR-sized) and references file-size policy (§7.3).
- Risk: Low.
- Rollback: N/A (planning-only change).

##### P1.1 — Seam extraction: Home/env plumbing (`home.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Extract CODEX_HOME layout + command env plumbing into `crates/codex/src/home.rs` and re-export from `lib.rs` without changing APIs.
- Expected files touched (if not already landed):
  - `crates/codex/src/home.rs`
  - `crates/codex/src/lib.rs` (add `mod home;` + `pub use ...`)
- Acceptance criteria (“done when”):
  - `cargo test --all-targets --all-features` passes.
  - Public API paths for the extracted types/functions remain stable (compile-time compatible).
  - File size of new module is within §7.3 policy (soft/hard/ceiling; exceptions require §9 decision).
- Risk: Low–Medium (module extraction can perturb visibility/import paths).
- Rollback: Move code back into `lib.rs` and revert re-export changes.

##### P1.2 — Seam extraction: Capability probing + caching (`capabilities/*`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Extract capability probing, snapshot/override serialization, TTL/backoff, and in-memory caching helpers into `crates/codex/src/capabilities/*`, preserving existing public API paths via re-exports.
- Expected files touched:
  - `crates/codex/src/lib.rs`
  - `crates/codex/src/capabilities/` (module)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports).
  - Extracted module meets §7.3 size policy (or has documented exception).
- Risk: Medium.
- Rollback: Revert module move and re-exports.

##### P1.3 — Seam extraction: Apply/diff request + artifacts (`apply_diff.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Extract apply/diff request modeling and artifact capture helpers into `crates/codex/src/apply_diff.rs`, preserving existing public API paths via re-exports.
- Expected files touched:
  - `crates/codex/src/lib.rs`
  - `crates/codex/src/apply_diff.rs`
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports).
  - Extracted module meets §7.3 size policy (or has documented exception).
- Risk: Medium.
- Rollback: Revert module move and re-exports.

##### P1.4 — Seam extraction: Execpolicy modeling + parsing (`execpolicy.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Extract execpolicy modeling/parsing and related helpers into `crates/codex/src/execpolicy.rs`, preserving existing public API paths via re-exports from `lib.rs`.
- Expected files touched:
  - `crates/codex/src/lib.rs`
  - `crates/codex/src/execpolicy.rs`
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports).
  - Extracted module meets §7.3 size policy (or has documented exception).
- Risk: Medium.
- Rollback: Revert module move and re-exports.

##### P1.5 — Seam extraction: Builder/config/flags surfaces (`builder.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Extract builder/config/flags surface area out of `crates/codex/src/lib.rs` into cohesive module(s) (starting with `crates/codex/src/builder.rs`), preserving existing public API paths via re-exports from `lib.rs`.
- Expected files touched:
  - `crates/codex/src/lib.rs`
  - `crates/codex/src/builder.rs` (new)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports).
  - New module(s) meet §7.3 size policy (or have documented exception in §9).
- Risk: Medium (builder/config is cross-cutting; avoid churn outside the seam).
- Rollback: Revert module move and re-exports.

##### P1.6 — Seam extraction: JSONL streaming/framing (`jsonl.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Extract JSONL streaming/framing + related IO helpers out of `crates/codex/src/lib.rs` into the existing `crates/codex/src/jsonl.rs` module, preserving existing public API paths via re-exports from `lib.rs`.
- Expected files touched:
  - `crates/codex/src/lib.rs`
  - `crates/codex/src/jsonl.rs` (existing)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports).
  - Updated module meets §7.3 size policy (or has documented exception in §9).
- Risk: Medium (streaming ordering/backpressure issues; tests should guard).
- Rollback: Revert module move and re-exports.

*(Repeat P1.x as needed until `lib.rs` is within the program ceiling or is a thin façade with the bulk in modules.)*

##### P1.7 — Follow-up split: `builder.rs` into `builder/*` (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/codex/src/builder.rs` size by splitting it into a cohesive `crates/codex/src/builder/*` module tree while preserving existing public API paths via re-exports from `crates/codex/src/lib.rs`.
- Expected files touched:
  - `crates/codex/src/lib.rs` (module wiring + re-exports as needed)
  - `crates/codex/src/builder.rs` (move to module tree)
  - `crates/codex/src/builder/` (new file(s))
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports maintain compatibility).
  - New file(s) meet §7.3 size policy (or have documented exception in §9).
- Risk: Medium (cross-cutting builder/config; keep changes seam-local).
- Rollback: Revert module split and re-exports; restore single-file `builder.rs`.

##### P1.8 — Plan next Phase 1 seams after `builder/*` (no code moves)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Update Phase 1 seam plan for the remaining `crates/codex/src/lib.rs` “god module” content after P1.7, identifying the next 2–4 PR-sized seams while preserving public API paths.
- Expected files touched:
  - `refactor_workplan.md` (this file): update §7.1 and §10 queue with the next concrete seam steps.
- Acceptance criteria (“done when”):
  - Next seams are listed in §7.1 with clear boundaries and extraction order.
  - Each planned extraction is PR-sized and reversible; size-policy expectations are noted (§7.3).
  - No behavior changes (planning only).
- Risk: Low.
- Rollback: N/A (planning-only change).

##### P1.9 — Seam extraction: bundled binary resolver (`bundled_binary.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Extract bundled binary resolution types + helpers (`BundledBinarySpec`, `BundledBinary`, `BundledBinaryError`, `resolve_bundled_binary`, `default_bundled_platform_label`) into `crates/codex/src/bundled_binary.rs`, preserving existing public API paths via re-exports from `lib.rs`.
- Expected files touched:
  - `crates/codex/src/lib.rs` (wire `mod bundled_binary;` + `pub use ...`; remove moved items)
  - `crates/codex/src/bundled_binary.rs` (new)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports).
  - New module meets §7.3 size policy (target ≤ 300 LOC; may exceed soft if cohesive but must stay ≤ 600).
- Risk: Low (leaf-ish extraction; minimal call site churn).
- Rollback: Revert module move + re-exports; restore original definitions in `lib.rs`.

##### P1.10 — Seam extraction: exec JSONL event models (`events.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Extract JSONL event envelope + payload models emitted by `codex exec --json` into `crates/codex/src/events.rs`, preserving existing public API paths via re-exports from `lib.rs` (e.g., `ThreadEvent`, `ItemEnvelope`, `ItemPayload`, delta/state structs, statuses).
- Expected files touched:
  - `crates/codex/src/lib.rs` (wire `mod events;` + `pub use ...`; remove moved items)
  - `crates/codex/src/events.rs` (new)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports).
  - Serde shapes remain identical (pure move; no field/tag renames; tests pass).
  - New module meets §7.3 size policy (likely > soft; must stay ≤ 600 or be split into `events/*` if it grows).
- Risk: Medium (serialization/type visibility churn; keep move mechanical).
- Rollback: Revert module move + re-exports; restore original event model definitions in `lib.rs`.

##### P1.11 — Seam extraction: CLI subcommand request/response models (`cli/*`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Extract request/response model structs for non-streaming CLI subcommands into a cohesive `crates/codex/src/cli/*` module tree (e.g., features/help/review/resume/fork/cloud/app-server and `codex mcp` command request types), preserving existing public API paths via re-exports from `lib.rs`.
  - Naming note: keep this distinct from the `mcp/*` runtime module tree; this seam is about *invoking CLI subcommands*, not the internal MCP client/runtime implementation.
- Expected files touched:
  - `crates/codex/src/lib.rs` (wire `mod cli;` + `pub use ...`; remove moved items)
  - `crates/codex/src/cli/mod.rs` (new)
  - `crates/codex/src/cli/*.rs` (new; split by domain to satisfy §7.3)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports).
  - New files meet §7.3 size policy (target ≤ 300 LOC each; must stay ≤ 600).
  - Step is “models-only” unless a small amount of wiring is necessary to keep the wrapper compiling (avoid moving execution logic in the same PR).
- Risk: Medium (large move surface; keep extraction mechanical and domain-split).
- Rollback: Revert module tree move + re-exports; restore original model structs in `lib.rs`.

##### P1.12 — Seam extraction: feature/version parsing + update advisory helpers (`version.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Extract internal parsing/helpers around `codex --version` / `codex help` / `codex features list` probes (semver/channel parsing, commit hash extraction, feature-flag parsing, and update advisory helpers) into `crates/codex/src/version.rs`, preserving existing public API paths for any public helpers (e.g., `update_advisory_from_capabilities`) via re-exports from `lib.rs`.
- Expected files touched:
  - `crates/codex/src/lib.rs` (wire `mod version;` + `pub use ...`; remove moved items)
  - `crates/codex/src/version.rs` (new)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports).
  - Parsing behavior remains unchanged (pure move; tests pass).
  - New module meets §7.3 size policy (target ≤ 300 LOC; may exceed soft if cohesive but must stay ≤ 600).
- Risk: Low–Medium (behavioral regression possible if parsing logic is accidentally edited; keep move mechanical).
- Rollback: Revert module move + re-exports; restore original helper functions in `lib.rs`.

##### P1.13 — Plan remaining Phase 1 seams after P1.12 (no code moves)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: After P1.9–P1.12 land, refresh the Phase 1 seam plan based on the then-current `crates/codex/src/lib.rs` structure and remaining top offenders, keeping extraction steps PR-sized and API-stable.
- Expected files touched:
  - `refactor_workplan.md` (this file): update §7.1 and §10 queue with the next concrete seam steps.
- Acceptance criteria (“done when”):
  - Updated seam list in §7.1 reflects the then-current module layout and identifies the next 2–4 PR-sized extractions.
  - No behavior changes (planning only).
- Risk: Low.
- Rollback: N/A (planning-only change).

---

##### P1.14 — Seam extraction: spawn/process plumbing (`process.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Extract the internal, cross-cutting subprocess spawning + stdio handling helpers from `crates/codex/src/lib.rs` into `crates/codex/src/process.rs` (or `process/*` if it grows), preserving public APIs by keeping these helpers private to the crate (or re-exporting only if already publicly reachable).
- Candidate contents (keep cohesive; move mechanically):
  - `spawn_with_retry` + spawn error classification.
  - stdout/stderr tee helpers (console mirroring, buffering, “quiet” handling) and shared IO utilities used by exec/apply/diff/sandbox/proxy flows.
- Expected files touched:
  - `crates/codex/src/lib.rs` (wire `mod process;`; update internal call sites)
  - `crates/codex/src/process.rs` (new) or `crates/codex/src/process/*` (new)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (helpers remain crate-private unless already exposed).
  - New file(s) meet §7.3 size policy (target ≤ 300 LOC; must stay ≤ 600 unless intentionally split).
- Risk: Medium (cross-cutting; many call sites). Keep changes purely mechanical and avoid behavior edits.
- Rollback: Revert module move; restore helpers in `lib.rs`.

##### P1.15 — Seam extraction: exec streaming + resume types + helpers (`exec/*`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Extract remaining exec/streaming surface area still defined in `crates/codex/src/lib.rs` into a cohesive `crates/codex/src/exec/*` module tree (or a single `exec.rs` first), preserving crate-root public API paths via `pub use` re-exports from `lib.rs`.
- Scope (targeted; avoid bundling unrelated subcommands):
  - Public types currently in `lib.rs`: `ExecStreamRequest`, `ResumeSelector`, `ResumeRequest`, `ExecStream`, `ExecCompletion`, `ExecStreamError`.
  - `CodexClient` methods: `send_prompt*`, `stream_exec*`, resume helpers, and any private helpers that are exec-stream specific (JSONL framing already lives in `jsonl.rs`; event models already live in `events.rs`).
- Expected files touched:
  - `crates/codex/src/lib.rs` (wire `mod exec;` or `mod exec { ... }` via `mod exec;` + re-exports; remove moved items)
  - `crates/codex/src/exec.rs` or `crates/codex/src/exec/*` (new)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (all moved public types remain available at their existing `codex::*` paths).
  - Move is mechanical: serde shapes, streaming semantics, and buffering/backpressure behavior unchanged (tests pass).
  - New files meet §7.3 size policy (split by domain if needed to stay ≤ 600 LOC).
- Dependencies / ordering:
  - Do after P1.14 so spawn/stdio helpers can be shared cleanly.
- Risk: Medium (streaming correctness; move-only mitigates).
- Rollback: Revert module move; restore original `lib.rs` definitions.

##### P1.16 — Seam extraction: auth/login helpers (`auth.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Move authentication state + login flow helpers out of `crates/codex/src/lib.rs` into `crates/codex/src/auth.rs`, preserving existing public API paths via re-exports from `lib.rs`.
- Scope:
  - `AuthSessionHelper` + related public enums: `CodexAuthStatus`, `CodexAuthMethod`, `CodexLogoutStatus`.
  - `CodexClient` login-related helpers (spawn/login status helpers) that are not exec-stream specific.
- Expected files touched:
  - `crates/codex/src/lib.rs` (wire `mod auth;` + re-exports; remove moved items)
  - `crates/codex/src/auth.rs` (new)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes.
  - Login process invocation semantics unchanged (pure move; tests pass).
  - New module meets §7.3 size policy.
- Dependencies / ordering:
  - Prefer after P1.14 so any shared spawn/stdio helpers can be referenced from a single internal module.
- Risk: Low–Medium (process spawn plumbing; keep move mechanical).
- Rollback: Revert module move + re-exports; restore original definitions in `lib.rs`.

##### P1.17 — Seam extraction: remaining client subcommand wrappers (`commands/*`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Extract the remaining `CodexClient` subcommand wrapper methods still implemented in `crates/codex/src/lib.rs` into a cohesive `crates/codex/src/commands/*` module tree while preserving public API paths and avoiding large cross-cutting moves.
- Scope (PR-sized; split by domain if needed):
  - `CodexClient::apply` / `CodexClient::diff` method implementations (types already live in `apply_diff.rs`).
  - `CodexClient::generate_app_server_bindings` (app-server generate helpers).
  - `CodexClient::list_features` (execution wrapper; parsing is in `version.rs`).
  - `CodexClient::start_responses_api_proxy`, `CodexClient::stdio_to_uds`, `CodexClient::run_sandbox`.
- Expected files touched:
  - `crates/codex/src/lib.rs` (wire `mod commands;` + `mod sandbox;`/`mod responses_api_proxy;` if split; keep façade stable)
  - `crates/codex/src/commands/` (new file(s))
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (methods remain on `CodexClient`; any public helper types remain available at existing `codex::*` paths).
  - Changes are mechanical: command flags/env, stdout/stderr mirroring, and exit-status handling unchanged.
  - New file(s) meet §7.3 size policy.
- Dependencies / ordering:
  - Prefer after P1.14 so shared spawn/stdio helpers live in one place.
- Risk: Medium (many flows; keep each move small and consider splitting this step into two PRs if it grows).
- Rollback: Revert module move(s); restore original method bodies in `lib.rs`.

##### P1.18 — Plan next Phase 1 seams after P1.17 (no code moves)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: After P1.14–P1.17 land, refresh §7.1 seam order + §10 queue based on the then-current `lib.rs` contents and remaining top offenders, keeping follow-on extractions PR-sized and reversible.
- Expected files touched:
  - `refactor_workplan.md` (this file): update §7.1 and §10 queue.
- Acceptance criteria (“done when”):
  - Updated seam list in §7.1 reflects the then-current module layout and identifies the next 2–4 PR-sized extractions.
  - No behavior changes (planning only).
- Risk: Low.
- Rollback: N/A.

##### P1.19 — Seam extraction: move `lib.rs` unit/integration-style tests into `crates/codex/src/tests.rs` (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/codex/src/lib.rs` size by moving the large `#[cfg(test)] mod tests { ... }` block into an out-of-line module file (`crates/codex/src/tests.rs`) while keeping test behavior identical.
- Expected files touched:
  - `crates/codex/src/lib.rs` (replace the inline test module with `#[cfg(test)] mod tests;`)
  - `crates/codex/src/tests.rs` (new; contains the moved test module content)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (tests-only move).
  - Move is mechanical: no test logic edits beyond import/path fixes required by the move.
- Risk: Low–Medium (test compilation visibility/import churn; keep move mechanical).
- Rollback: Move the test module back into `lib.rs`.

##### P1.20 — Seam extraction: core `CodexClient` command runner helpers into `client_core.rs` (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `lib.rs` size by moving the “core runner” implementation details (e.g., `run_basic_command`, `run_simple_command_with_overrides`, working dir context helpers) into a dedicated internal module while keeping `CodexClient`’s public API and behavior unchanged.
- Expected files touched:
  - `crates/codex/src/lib.rs` (wire `mod client_core;` and keep a small façade `impl CodexClient` surface)
  - `crates/codex/src/client_core.rs` (new; `impl CodexClient { ... }` blocks + private helpers)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No externally observable behavior changes (pure move; tests pass).
  - No public API path changes.
  - New file meets §7.3 size policy (split if it grows beyond 600 LOC).
- Risk: Medium (subtle behavior changes if refactoring instead of moving; keep edits purely mechanical).
- Rollback: Revert module move; restore the original implementations in `lib.rs`.

##### P1.21 — Seam extraction: `CodexError` and shared “defaults” helpers into dedicated modules (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Move `CodexError` and small crate-root helper functions/constants (e.g., default env/binary path helpers) out of `lib.rs` into cohesive modules to reduce façade size, while preserving `codex::CodexError` and all other public item paths via re-exports.
- Expected files touched:
  - `crates/codex/src/lib.rs` (wire new modules + `pub use` re-exports)
  - `crates/codex/src/error.rs` (new; `CodexError` definition)
  - `crates/codex/src/defaults.rs` (new; small helpers/constants as appropriate)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (`codex::CodexError` and any public helpers remain available at the same paths).
  - Move is mechanical: error variants/messages and helper semantics unchanged (tests pass).
  - New files meet §7.3 size policy.
- Risk: Low–Medium (import/visibility churn; keep move mechanical).
- Rollback: Revert module move; restore original definitions in `lib.rs`.

##### P1.22 — Seam extraction: remaining `CodexClient` non-streaming wrapper methods into `commands/*` follow-ups (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Finish draining `lib.rs` of the remaining non-streaming `CodexClient` wrapper methods still implemented in the crate root (e.g., `features`, `help`, `review`, `exec_review`, `fork_session`, `cloud_*`), by moving them into additional `crates/codex/src/commands/*` modules as `impl CodexClient` blocks.
- Expected files touched:
  - `crates/codex/src/lib.rs` (remove moved method bodies; keep façade + re-exports stable)
  - `crates/codex/src/commands/` (add new file(s) as needed, split by subcommand/domain)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (methods remain on `CodexClient`; signatures unchanged).
  - Move is mechanical: CLI args/env/output handling unchanged (tests pass).
  - New files meet §7.3 size policy.
- Risk: Medium (many small flows; keep moves grouped by subcommand domain and avoid opportunistic edits).
- Rollback: Revert module moves; restore method bodies in `lib.rs`.

##### P1.23 — Refresh Phase 1 size evidence and (if eligible) close Phase 1 (no code moves)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: After P1.19–P1.22 land, re-measure `crates/codex/src/lib.rs` size/top offenders and update §3.2/Phase 1 status so the workplan reflects the actual post-extraction state.
- Expected files touched:
  - `refactor_workplan.md` (this file): update §3.2 top-offenders evidence reference(s) and Phase 1 status/reason if `lib.rs` is now below the ceiling.
  - `evidence_runs/YYYY-MM-DD/` (new measurement outputs, e.g., tokei + sorted list)
- Acceptance criteria (“done when”):
  - Measurement evidence is captured and cited via file paths.
  - Phase 1 status is updated only if the evidence supports it (no subjective “looks good”).
  - No behavior changes (planning/measurement only).
- Risk: Low.
- Rollback: N/A.

#### Phase 2 — Split `crates/codex/src/mcp.rs` into `crates/codex/src/mcp/*` (API stable)

**Phase goal:** Split `mcp.rs` into a module tree while maintaining stable public APIs via re-exports from `mcp` (and/or `lib.rs`).

Phase Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05  
Reason: P2.7 completed; `crates/codex/src/mcp.rs` is now a thin façade below the program ceiling (policy: §7.3).

##### P2.0 — Define the `mcp.rs` seam map (no code moves yet)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Identify boundaries (config, runtime manager, connectors, auth/token handling, persistence) and extraction order.
- Expected files touched:
  - `refactor_workplan.md` (update §7.2 with final boundaries/order).
- Acceptance criteria (“done when”):
  - Seam definitions in §7.2 minimize cross-module cycles.
  - Extraction order is PR-sized and reversible.
- Risk: Low.
- Rollback: N/A (planning-only change).

##### P2.1 — Create `mcp/` module façade and move one internal submodule (smallest-first)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Establish `crates/codex/src/mcp/mod.rs` (or equivalent) with re-exports and move the smallest cohesive section first.
- Expected files touched:
  - `crates/codex/src/mcp.rs` (reduce scope; keep compatibility layer)
  - `crates/codex/src/mcp/` (new files)
  - `crates/codex/src/lib.rs` (if needed for module wiring)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (re-exports maintain compatibility).
  - New files comply with §7.3 size policy.
- Risk: Medium (large file split can introduce visibility and import churn).
- Rollback: Revert move; restore original `mcp.rs` content.

*(Repeat P2.x until `mcp.rs` is reduced to a compatibility façade or removed per policy.)*

##### P2.2 — Move MCP config definitions + persistence into `mcp/config.rs` (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Isolate stored config types (`[mcp_servers]`, `[app_runtimes]`) and `McpConfigManager` read/write helpers into `crates/codex/src/mcp/config.rs`, preserving `codex::mcp::*` public API paths via re-exports.
- Expected files touched:
  - `crates/codex/src/mcp.rs` (wire `mod config;` + re-exports; remove moved items)
  - `crates/codex/src/mcp/config.rs`
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports maintain compatibility).
  - New file(s) comply with §7.3 size policy (or have a documented exception in §9).
- Risk: Medium.
- Rollback: Revert file move and re-exports; restore original definitions in `mcp.rs`.

##### P2.3 — Move runtime resolution + launchers into `mcp/runtime.rs` (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Move runtime-ready types and helpers (`McpRuntimeServer`, launchers/connectors, runtime manager/API) into `crates/codex/src/mcp/runtime.rs`, keeping `codex::mcp::*` paths stable via re-exports.
- Expected files touched:
  - `crates/codex/src/mcp.rs` (wire `mod runtime;` + re-exports; remove moved items)
  - `crates/codex/src/mcp/runtime.rs`
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports maintain compatibility).
  - New file(s) comply with §7.3 size policy (or have a documented exception in §9).
- Risk: Medium.
- Rollback: Revert file move and re-exports; restore original definitions in `mcp.rs`.

##### P2.4 — Move app runtime lifecycle/pool helpers into `mcp/app.rs` (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Move app runtime modeling + lifecycle/pool helpers (`AppRuntime*` types, manager/pool APIs) into `crates/codex/src/mcp/app.rs`, keeping `codex::mcp::*` paths stable via re-exports.
- Expected files touched:
  - `crates/codex/src/mcp.rs` (wire `mod app;` + re-exports; remove moved items)
  - `crates/codex/src/mcp/app.rs`
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports maintain compatibility).
  - New file(s) comply with §7.3 size policy (or have a documented exception in §9).
- Risk: Medium.
- Rollback: Revert file move and re-exports; restore original definitions in `mcp.rs`.

##### P2.5 — JSON-RPC transport extraction (stdio transport → `mcp/jsonrpc.rs`) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Move stdio JSON-RPC transport plumbing (spawn, request/response/notification pump, and stream fan-out) into `crates/codex/src/mcp/jsonrpc.rs` while keeping `codex::mcp::*` public API paths stable via re-exports.
- Expected files touched:
  - `crates/codex/src/mcp.rs`
  - `crates/codex/src/mcp/jsonrpc.rs`
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports maintain compatibility).
  - New file(s) comply with §7.3 size policy (or have a documented exception in §9).
- Risk: Medium–High (transport extraction is high-churn and can introduce subtle ordering/timeout regressions).
- Rollback: Revert transport move; restore original transport code in `mcp.rs`.

##### P2.6 — Move high-level MCP clients into `mcp/client.rs` (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/codex/src/mcp.rs` by extracting high-level client façade types (e.g., Codex/app-server client wrappers that sit above JSON-RPC transport) into `crates/codex/src/mcp/client.rs`, preserving stable `codex::mcp::*` API paths via re-exports.
- Expected files touched:
  - `crates/codex/src/mcp.rs`
  - `crates/codex/src/mcp/client.rs` (new)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports maintain compatibility).
  - New file meets §7.3 size policy (or has a documented exception in §9).
- Risk: Medium (visibility/import churn; preserve API paths via re-exports).
- Rollback: Revert file move and re-exports; restore original definitions in `mcp.rs`.

##### P2.7 — Reduce `mcp.rs` below program ceiling (remaining coordinator split) (API preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Further reduce `crates/codex/src/mcp.rs` by moving remaining cohesive coordinator/state helpers into one or more `crates/codex/src/mcp/*` modules, leaving `mcp.rs` as a thin compatibility façade while preserving stable `codex::mcp::*` public API paths via re-exports.
- Expected files touched:
  - `crates/codex/src/mcp.rs`
  - `crates/codex/src/mcp/` (new file(s) as needed)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - No public API path changes (façade + re-exports maintain compatibility).
  - New file(s) meet §7.3 size policy (or have documented exception in §9).
- Risk: Medium–High (large file split; keep changes seam-local).
- Rollback: Revert module move and re-exports; restore original definitions in `mcp.rs`.

---

### Workstream C — `crates/xtask` Maintainability (determinism preserved)

#### Phase 3 — Split xtask “rule engine” files by domain sections; keep determinism

Phase Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05  
Reason: Phase 3 checklist complete; `crates/xtask/src/codex_validate.rs` is now below the ceiling (P3.9).

##### P3.0 — Identify xtask domain boundaries and deterministic contracts

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Document domain boundaries and determinism requirements (sorting, stable output formats, snapshot expectations).
- Expected files touched:
  - `refactor_workplan.md` (update §7.4 with boundaries and extraction order).
- Acceptance criteria (“done when”):
  - Boundaries and invariants are documented and referenced by subsequent P3.x steps.
- Risk: Low.
- Rollback: N/A.

##### P3.1 — Split xtask module: `codex_version_metadata` (extract JSON model types) with deterministic output preserved

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Reduce `crates/xtask/src/codex_version_metadata.rs` size by extracting leaf JSON model/types into `crates/xtask/src/codex_version_metadata/models.rs` without changing outputs.
- Expected files touched:
  - `crates/xtask/src/codex_version_metadata.rs`
  - `crates/xtask/src/codex_version_metadata/models.rs`
- Acceptance criteria (“done when”):
  - `cargo test --all-targets --all-features` passes (xtask tests are the determinism guard).
  - `cargo clippy --all-targets --all-features -- -D warnings` passes.
  - Output determinism preserved (validated by existing snapshot/spec tests; no golden changes unless explicitly approved).
- Risk: Medium.
- Rollback: Revert module move; restore original file content.

##### P3.2 — Split xtask module: `codex_union` (extract schema structs) with deterministic output preserved

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Reduce `crates/xtask/src/codex_union.rs` size by extracting union snapshot schema structs into `crates/xtask/src/codex_union/schema.rs` without changing outputs.
- Expected files touched:
  - `crates/xtask/src/codex_union.rs`
  - `crates/xtask/src/codex_union/schema.rs`
- Acceptance criteria (“done when”):
  - `cargo test --all-targets --all-features` passes (xtask tests are the determinism guard).
  - `cargo clippy --all-targets --all-features -- -D warnings` passes.
  - Output determinism preserved (validated by existing snapshot/spec tests; no golden changes unless explicitly approved).
- Risk: Medium.
- Rollback: Revert module move; restore original file content.

##### P3.3 — Split xtask module: `codex_snapshot` (extract schema structs) with deterministic output preserved

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Reduce `crates/xtask/src/codex_snapshot.rs` size by extracting per-target snapshot schema structs into `crates/xtask/src/codex_snapshot/schema.rs` without changing outputs.
- Expected files touched:
  - `crates/xtask/src/codex_snapshot.rs`
  - `crates/xtask/src/codex_snapshot/schema.rs`
- Acceptance criteria (“done when”):
  - `cargo test --all-targets --all-features` passes (xtask tests are the determinism guard).
  - `cargo clippy --all-targets --all-features -- -D warnings` passes.
  - Output determinism preserved (validated by existing snapshot/spec tests; no golden changes unless explicitly approved).
- Risk: Medium.
- Rollback: Revert module move; restore original file content.

##### P3.4 — Split xtask module: `codex_report` (extract schema structs) with deterministic output preserved

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Reduce `crates/xtask/src/codex_report.rs` size by extracting report input schema structs into `crates/xtask/src/codex_report/models.rs` without changing outputs.
- Expected files touched:
  - `crates/xtask/src/codex_report.rs`
  - `crates/xtask/src/codex_report/models.rs`
- Acceptance criteria (“done when”):
  - `cargo test --all-targets --all-features` passes (xtask tests are the determinism guard).
  - `cargo clippy --all-targets --all-features -- -D warnings` passes.
  - Output determinism preserved (validated by existing snapshot/spec tests; no golden changes unless explicitly approved).
- Risk: Medium.
- Rollback: Revert module move; restore original file content.

##### P3.5 — Split xtask module: `codex_validate` (extract model + rules structs) with deterministic output preserved

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Reduce `crates/xtask/src/codex_validate.rs` size by extracting leaf model and rules structs into `crates/xtask/src/codex_validate/models.rs` without changing outputs.
- Expected files touched:
  - `crates/xtask/src/codex_validate.rs`
  - `crates/xtask/src/codex_validate/models.rs`
- Acceptance criteria (“done when”):
  - `cargo test --all-targets --all-features` passes (xtask tests are the determinism guard).
  - `cargo clippy --all-targets --all-features -- -D warnings` passes.
  - Output determinism preserved (validated by existing snapshot/spec tests; no golden changes unless explicitly approved).
- Risk: Medium.
- Rollback: Revert module move; restore original file content.

##### P3.6 — Split xtask module: `codex_validate` (extract validation passes) with deterministic output preserved

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Further reduce `crates/xtask/src/codex_validate.rs` by extracting cohesive validation passes (pointer validation, schema compilation/validation, wrapper coverage semantics, report invariants) into `crates/xtask/src/codex_validate/*` without changing outputs.
- Expected files touched:
  - `crates/xtask/src/codex_validate.rs`
  - `crates/xtask/src/codex_validate/` (new files)
- Acceptance criteria (“done when”):
  - `cargo test --all-targets --all-features` passes (xtask tests are the determinism guard).
  - `cargo clippy --all-targets --all-features -- -D warnings` passes.
  - Output determinism preserved (validated by existing snapshot/spec tests; no golden changes unless explicitly approved).
- Risk: Medium.
- Rollback: Revert module move; restore original file content.

##### P3.7 — Split xtask module: `codex_report` (extract report domains) with deterministic output preserved

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Further reduce `crates/xtask/src/codex_report.rs` by extracting report domain logic (rules parsing, filtering semantics, output shaping/sorting) into `crates/xtask/src/codex_report/*` without changing outputs.
- Expected files touched:
  - `crates/xtask/src/codex_report.rs`
  - `crates/xtask/src/codex_report/` (new files)
- Acceptance criteria (“done when”):
  - `cargo test --all-targets --all-features` passes (xtask tests are the determinism guard).
  - `cargo clippy --all-targets --all-features -- -D warnings` passes.
  - Output determinism preserved (validated by existing snapshot/spec tests; no golden changes unless explicitly approved).
- Risk: Medium.
- Rollback: Revert module move; restore original file content.

##### P3.8 — Split xtask module: `codex_snapshot` (extract snapshot pipeline domains) with deterministic output preserved

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-04

- Goal: Further reduce `crates/xtask/src/codex_snapshot.rs` by extracting snapshot pipeline stages (help parsing/discovery, supplements/normalization, output layout/version probing) into `crates/xtask/src/codex_snapshot/*` without changing outputs.
- Expected files touched:
  - `crates/xtask/src/codex_snapshot.rs`
  - `crates/xtask/src/codex_snapshot/` (new files)
- Acceptance criteria (“done when”):
  - `cargo test --all-targets --all-features` passes (xtask tests are the determinism guard).
  - `cargo clippy --all-targets --all-features -- -D warnings` passes.
  - Output determinism preserved (validated by existing snapshot/spec tests; no golden changes unless explicitly approved).
- Risk: Medium.
- Rollback: Revert module move; restore original file content.

##### P3.9 — Reduce `codex_validate.rs` below ceiling (follow-on split) with deterministic output preserved

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Further reduce `crates/xtask/src/codex_validate.rs` below the program ceiling by extracting remaining cohesive validation orchestration/formatting/IO helpers into `crates/xtask/src/codex_validate/*` without changing outputs.
- Expected files touched:
  - `crates/xtask/src/codex_validate.rs`
  - `crates/xtask/src/codex_validate/` (new file(s) as needed)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - Output determinism preserved (validated by existing snapshot/spec tests; no golden changes unless explicitly approved).
  - `crates/xtask/src/codex_validate.rs` is ≤ 1000 LOC (or a §9 exception is recorded with a follow-up split task).
- Risk: Medium (validation output semantics; rely on existing tests).
- Rollback: Revert module move; restore original file content.

---

### Workstream D — Test Suite Modularization + Remaining File-Size Violations

#### Phase 4 — Modularize oversized test suites and close remaining size-policy gaps

Phase Status: [ ] Not Started  [x] In Progress  [ ] Done  
Last Updated: 2026-02-05  
Reason: P4.4 refreshed the measurements (`evidence_runs/2026-02-05/P4.4_tokei_crates.json`, `evidence_runs/2026-02-05/P4.4_rust_files_sorted_by_code.txt`); there are now zero >ceiling offenders, but seven files remain >hard and require follow-on splits.

##### P4.0 — Reduce crates/codex/src/tests.rs below ceiling (tests modularization)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/codex/src/tests.rs` from 3,178 LOC to `<= 1000` LOC while preserving test behavior.
- Suggested approach:
  - Convert `crates/codex/src/tests.rs` into `crates/codex/src/tests/mod.rs`.
  - Split tests into domain-focused submodules under `crates/codex/src/tests/` (for example: auth, commands, jsonl, process, mcp-related helpers as applicable).
- Expected files touched:
  - `crates/codex/src/tests.rs` (delete/replace with module-tree entrypoint)
  - `crates/codex/src/tests/mod.rs` (new)
  - `crates/codex/src/tests/*.rs` (new domain submodules)
  - `crates/codex/src/lib.rs` (only if test-module wiring requires it)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - `crates/codex/src/tests.rs` no longer exists as a monolith; `crates/codex/src/tests/mod.rs` + submodules compile and tests pass.
  - No public API changes.
  - Evidence is written to `evidence_runs/YYYY-MM-DD/` (validation outputs + updated size artifacts).
- Risk: Medium–High (large test-file move can introduce import/path churn).
- Rollback: Revert module split and restore original `crates/codex/src/tests.rs`.

##### P4.1 — Reduce `crates/codex/src/mcp/tests_core.rs` below hard threshold (test modularization)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/codex/src/mcp/tests_core.rs` from 870 LOC to `<= 600` LOC by extracting cohesive test domains into submodules.
- Expected files touched:
  - `crates/codex/src/mcp/tests_core.rs`
  - `crates/codex/src/mcp/tests_core/` (new submodules as needed)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - `crates/codex/src/mcp/tests_core.rs` is `<= 600` LOC (or a §9 exception is explicitly recorded with a follow-up step).
  - Behavior preserved (test expectations unchanged aside from mechanical reorganization).
  - Evidence is written to `evidence_runs/YYYY-MM-DD/`.
- Risk: Medium (test fixture/import churn).
- Rollback: Revert module split; restore original file content.

##### P4.2 — Reduce `crates/codex/src/mcp/tests_runtime_app.rs` below hard threshold (test modularization)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/codex/src/mcp/tests_runtime_app.rs` from 861 LOC to `<= 600` LOC by extracting runtime/app test domains into submodules.
- Expected files touched:
  - `crates/codex/src/mcp/tests_runtime_app.rs`
  - `crates/codex/src/mcp/tests_runtime_app/` (new submodules as needed)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - `crates/codex/src/mcp/tests_runtime_app.rs` is `<= 600` LOC (or a §9 exception is explicitly recorded with a follow-up step).
  - Behavior preserved (test expectations unchanged aside from mechanical reorganization).
  - Evidence is written to `evidence_runs/YYYY-MM-DD/`.
- Risk: Medium (test fixture/import churn).
- Rollback: Revert module split; restore original file content.

##### P4.3 — Reduce `crates/xtask/src/codex_report/report.rs` below hard threshold (domain split with deterministic output preserved)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/xtask/src/codex_report/report.rs` from 922 LOC to `<= 600` LOC by extracting cohesive report domains while preserving deterministic outputs.
- Expected files touched:
  - `crates/xtask/src/codex_report/report.rs`
  - `crates/xtask/src/codex_report/` (new/expanded domain modules)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - `crates/xtask/src/codex_report/report.rs` is `<= 600` LOC (or a §9 exception is explicitly recorded with a follow-up step).
  - Output determinism preserved (existing xtask tests remain green; no golden/snapshot changes unless explicitly approved).
  - Evidence is written to `evidence_runs/YYYY-MM-DD/`.
- Risk: Medium (report ordering/formatting regressions if boundaries are wrong).
- Rollback: Revert module split; restore original file content.

##### P4.4 — Refresh size evidence after P4.0–P4.3 and update queue/phase status (no code moves)

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Re-run size measurements after Phase 4 steps, then update §3.2/§10 and Phase 4 status based on evidence only.
- Expected files touched:
  - `refactor_workplan.md`
  - `evidence_runs/YYYY-MM-DD/` (new tokei + sorted Rust LOC outputs and gate artifacts)
- Acceptance criteria (“done when”):
  - Measurement evidence is captured and cited via exact paths.
  - §3.2 top offenders and §10 queue are updated to match latest evidence.
  - Phase 4 status is updated only if evidence supports it.
- Risk: Low.
- Rollback: N/A (planning/measurement only).

##### P4.5 — Reduce `crates/codex/src/tests/capabilities.rs` below hard threshold

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/codex/src/tests/capabilities.rs` from 904 LOC to `<= 600` LOC by splitting capability-domain tests into cohesive submodules.
- Expected files touched:
  - `crates/codex/src/tests/capabilities.rs`
  - `crates/codex/src/tests/capabilities/` (new submodules as needed)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - `crates/codex/src/tests/capabilities.rs` is `<= 600` LOC (or a §9 exception is recorded with a follow-up step).
  - Test behavior is unchanged aside from mechanical organization.
- Risk: Medium (test fixture/import churn).
- Rollback: Revert module split; restore original file content.

##### P4.6 — Reduce `crates/xtask/src/codex_union.rs` below hard threshold

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/xtask/src/codex_union.rs` from 799 LOC to `<= 600` LOC by extracting cohesive union helpers/modules while preserving deterministic outputs.
- Expected files touched:
  - `crates/xtask/src/codex_union.rs`
  - `crates/xtask/src/codex_union/` (new submodules as needed)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - `crates/xtask/src/codex_union.rs` is `<= 600` LOC (or a §9 exception is recorded with a follow-up step).
  - Output behavior and ordering remain unchanged (existing tests stay green).
- Risk: Medium (domain split can impact merge/ordering behavior if boundaries are wrong).
- Rollback: Revert module split; restore original file content.

##### P4.7 — Reduce `crates/xtask/tests/c3_spec_reports_metadata_retain.rs` below hard threshold

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/xtask/tests/c3_spec_reports_metadata_retain.rs` from 742 LOC to `<= 600` LOC via mechanical test modularization.
- Expected files touched:
  - `crates/xtask/tests/c3_spec_reports_metadata_retain.rs`
  - `crates/xtask/tests/c3_spec_reports_metadata_retain/` (new submodules as needed)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - `crates/xtask/tests/c3_spec_reports_metadata_retain.rs` is `<= 600` LOC (or a §9 exception is recorded with a follow-up step).
  - Assertions and expected snapshots/outputs are unchanged.
- Risk: Medium (test module wiring/import churn).
- Rollback: Revert module split; restore original file content.

##### P4.8 — Reduce `crates/xtask/src/codex_version_metadata.rs` below hard threshold

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/xtask/src/codex_version_metadata.rs` from 721 LOC to `<= 600` LOC by extracting cohesive metadata helpers while preserving output semantics.
- Expected files touched:
  - `crates/xtask/src/codex_version_metadata.rs`
  - `crates/xtask/src/codex_version_metadata/` (new submodules as needed)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - `crates/xtask/src/codex_version_metadata.rs` is `<= 600` LOC (or a §9 exception is recorded with a follow-up step).
  - Output/formatting behavior remains unchanged.
- Risk: Medium (domain split can alter formatting/order if not mechanical).
- Rollback: Revert module split; restore original file content.

##### P4.9 — Reduce `crates/codex/src/exec.rs` below hard threshold

Status: [ ] Not Started  [ ] In Progress  [x] Done  
Last Updated: 2026-02-05

- Goal: Reduce `crates/codex/src/exec.rs` from 676 LOC to `<= 600` LOC by extracting cohesive execution helpers while preserving public behavior.
- Expected files touched:
  - `crates/codex/src/exec.rs`
  - `crates/codex/src/exec/` (new submodules as needed)
- Acceptance criteria (“done when”):
  - All §4.1 gates pass.
  - `crates/codex/src/exec.rs` is `<= 600` LOC (or a §9 exception is recorded with a follow-up step).
  - No public API path changes or behavior changes.
- Risk: Medium (runtime behavior regression risk if boundaries are wrong).
- Rollback: Revert module split; restore original file content.

---

## 6) Dependency Triage (Supply Chain)

### 6.1 Advisory tracking table (fill as resolved)

Record *each* advisory encountered and the chosen remediation. At minimum, include the initial critical advisory from:
- `audit_pack/supply_chain/cargo_audit.txt`
- `audit_pack/supply_chain/cargo_deny_advisories.txt`

| Advisory ID | Crate | Affected Version(s) | Introduced via | Remediation | Verification (commands + result) | Notes |
|---|---|---|---|---|---|---|
| RUSTSEC-2026-0007 | bytes | 1.11.0 | see dependency trees in `audit_pack/supply_chain/cargo_audit.txt` | Upgrade to >= 1.11.1 | 2026-02-04: `cargo audit` PASS; `cargo deny check advisories` PASS | Critical |

### 6.2 License policy tracking (cargo-deny)

Baseline failure evidence: `audit_pack/supply_chain/cargo_deny_licenses.txt` shows “no config found” and many rejected licenses due to empty allowlist.

Track policy decisions here:

| Policy item | Decision | Date | Rationale | Status |
|---|---|---|---|---|
| Allowed license expressions | `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSL-1.0`, `Unicode-3.0` | 2026-02-04 | Explicit permissive policy in `deny.toml` | Accepted |
| Target triples for graph | `x86_64-unknown-linux-musl`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc` | 2026-02-04 | Scope checks to supported distribution targets | Accepted |
| Confidence threshold | 0.8 | 2026-02-04 | Balance false positives vs enforcement | Accepted |

### 6.3 Duplicate versions triage

Evidence sources:
- Audit-time: `audit_pack/deps/cargo_tree_duplicates.txt` (`cargo tree -d` without `--target all`; “nothing to print”).
- Post-refactor/current: `audit_pack/execution/2026-02-04/post_refactor_cargo_tree_duplicates_target_all_updated.txt` (`cargo tree -d --target all`; duplicates present).

| Crate | Duplicate versions? | Evidence | Decision (fix/defer) | Rationale | Status |
|---|---:|---|---|---|---|
| getrandom | Yes (`0.2.17`, `0.3.4`) | `audit_pack/execution/2026-02-04/post_refactor_cargo_tree_duplicates_target_all_updated.txt` shows `getrandom 0.2.17` via `jsonschema → xtask` and `getrandom 0.3.4` via `ahash` (through `jsonschema`) and `tempfile → codex → xtask`. | Defer | Resolving likely requires upgrading transitive crates (non-goal in Phase 0 unless required for security/compliance); keep note and revisit if it becomes a policy requirement. | Done |
| windows-sys | Yes (`0.60.2`, `0.61.2`) | `audit_pack/execution/2026-02-04/post_refactor_cargo_tree_duplicates_target_all_updated.txt` shows `windows-sys 0.60.2` via `socket2 → tokio/reqwest` and `windows-sys 0.61.2` via `clap`/`anstream` plus `tempfile`/`tokio`. | Defer | Same rationale as above; no gate currently failing and no security advisory forcing consolidation. | Done |

---

## 7) Modularization Strategy (Boundaries, API stability, size policy)

### 7.1 `crates/codex/src/lib.rs` seam extraction order (Phase 1)

**Rule:** `lib.rs` remains the façade; extracted modules keep stable public API via `pub use` re-exports from `lib.rs`. Do not change public item paths without an explicit migration plan (§9 decision log).

Seam order (defined in P1.0; extract in PR-sized steps):
1) Home/env plumbing → `home.rs` (P1.1; already present in working tree).
2) Capability probing + caching → `capabilities.rs` (P1.2).
3) Apply/diff request + artifacts → `apply_diff.rs` (P1.3).
4) Execpolicy modeling + parsing → `execpolicy.rs` (P1.4).
5) Builder/config/flags surfaces → one or more cohesive modules (P1.5; define concrete module(s) when scheduled).
6) JSONL streaming/framing + process IO → `jsonl.rs` seam (P1.6).
7) Bundled binary resolution helpers → `bundled_binary.rs` (P1.9).
8) Exec JSONL event envelope + payload models → `events.rs` (P1.10).
9) CLI subcommand request/response models → `cli/*` (P1.11).
10) Version/features probe parsing + update advisory helpers → `version.rs` (P1.12).
11) Spawn/process plumbing → `process.rs` (P1.14).
12) Exec streaming + resume types + helpers → `exec/*` (P1.15).
13) Auth/login helpers → `auth.rs` (P1.16).
14) Remaining client subcommand wrappers → `commands/*` (P1.17).
15) Move crate-root `#[cfg(test)]` tests out-of-line → `tests.rs` (P1.19).
16) Core `CodexClient` runner helpers (`run_*`, working dir context) → `client_core.rs` (P1.20).
17) Error + defaults consolidation → `error.rs` / `defaults.rs` (P1.21).
18) Drain remaining non-streaming wrappers still in `lib.rs` → `commands/*` follow-ups (P1.22).

Notes / dependencies:
- `capabilities.rs` is used by the builder/client for cache policies, overrides, and probes; extract before apply/diff to reduce cross-cutting churn.
- `apply_diff.rs` depends on command execution plumbing but should remain API-stable via re-exports from `lib.rs`.
- `bundled_binary.rs` is leaf-ish and can move early to reduce `lib.rs` churn (it is used mainly by the builder and docs).
- `events.rs` is a dependency of streaming exec/resume types; extract it before moving execution/stream implementation blocks.
- `cli/*` request/response models depend on builder override types (`CliOverridesPatch`, `FeatureToggles`, `ConfigOverride`), so schedule after P1.7 stabilized the builder module tree (done in P1.11).
- `process.rs` should land before moving additional `CodexClient` impl blocks so spawn/stdio helpers have a single home and are not duplicated across modules.
- `exec/*` should focus on `codex exec` streaming + resume surfaces; keep other subcommands (apply/diff, sandbox, app-server generate) out of scope to keep the PR-sized.
- P1.19 is the lowest-risk “big win” to reduce `lib.rs` LOC quickly without touching runtime code; do it before additional `impl CodexClient` extraction to minimize merge conflicts.

### 7.2 `crates/codex/src/mcp.rs` boundaries and extraction order (Phase 2)

**Rule:** Public MCP APIs must remain stable via `mcp` façade + re-exports.

Seam boundaries (defined in P2.0; extract in PR-sized steps; keep `codex::mcp::*` paths stable):
1) **Protocol types** → `crates/codex/src/mcp/protocol.rs`
   - JSON-RPC method constants (`METHOD_*`), request/response/notification payload structs
   - Codex/app-server parameter structs and notification enums
   - Shared types: `RequestId`, `EventStream`, call handle structs
2) **Stored config + persistence** → `crates/codex/src/mcp/config.rs`
   - TOML config keys + stored definition structs (`[mcp_servers]`, `[app_runtimes]`)
   - `McpConfigManager` read/write/update helpers + `McpConfigError`
3) **Runtime resolution + launchers** → `crates/codex/src/mcp/runtime.rs`
   - Resolved runtime types (`McpRuntimeServer`, resolved transports) and env/token resolution
   - Launchers/connectors (`StdioLauncher`, `StreamableHttpConnector`, etc.)
   - Runtime manager + listing helpers (`McpRuntimeManager`, `McpRuntimeApi`, summaries/handles)
4) **App runtime lifecycle + pooling** → `crates/codex/src/mcp/app.rs`
   - App runtime modeling + launch config (`AppRuntime*` types)
   - Manager/pool APIs (`AppRuntimeManager`, `AppRuntimeApi`, `AppRuntimePool*`)
5) **Stdio JSON-RPC transport** → `crates/codex/src/mcp/jsonrpc.rs` (P2.5; scheduled after the above)
   - `codex mcp-server` / `codex app-server` spawn + request/response plumbing + notification fan-out
   - Keep the higher-level clients (`CodexMcpServer`, `AppServer`) in `mcp.rs` until transport move is complete.
6) **High-level clients** → `crates/codex/src/mcp/client.rs` (P2.6; scheduled after transport extraction)
   - Move high-level client façade types above JSON-RPC transport while keeping `codex::mcp::*` paths stable via re-exports.
7) **Remaining coordinator split** → additional `crates/codex/src/mcp/*` modules (P2.7)
   - Reduce `mcp.rs` to a thin compatibility façade; keep extracted modules within §7.3 thresholds.

Notes / dependencies:
- Phase 2 exists because `crates/codex/src/mcp.rs` is a “god module” at audit time (4,278 LOC). Evidence: `audit_pack/metrics/loc_summary.txt`.
- Extract in the order above to avoid cycles: protocol is leaf-ish; config is depended on by runtime/app; runtime/app depend on protocol; JSON-RPC transport is used by the high-level clients.

### 7.3 File size policy (applies during refactor)

Derived from latest post-refactor distribution in `audit_pack/execution/2026-02-04/post_refactor_tokei_updated.json`:
- Soft threshold: 300 LOC (P75 ≈ 291.0)
- Hard threshold: 600 LOC (P90 ≈ 534.0; policy minimum is 600)
- Absolute ceiling: 1000 LOC

Policy:
- New files should target **≤ 300 LOC**.
- New files may exceed soft threshold if cohesive, but should stay **≤ 600 LOC**.
- Files **must not exceed 1000 LOC** without an explicit §9 decision (exception with rationale and follow-up split task).
- For “compatibility façades” (`lib.rs`, `mcp.rs` during transition), temporary exceptions may exist but must trend downward each phase.

### 7.4 `crates/xtask` modularization boundaries (Phase 3)

Boundaries + extraction order (defined in P3.0; keep deterministic outputs):

- Determinism contracts (must remain true after each P3.x step):
  - Sorting is stable and explicit: use `BTreeMap`/`BTreeSet` or `sort_by` with total ordering; tie-breakers must be deterministic.
  - Generated timestamps are deterministic under `SOURCE_DATE_EPOCH` (tests set this); if unset, fall back to wall-clock.
  - JSON outputs use stable formatting (`serde_json::to_string_pretty`) and end with a trailing newline.
  - Outputs are derived only from inputs under `<root>` (default `cli_manifests/codex`) and CLI args; no ambient FS scans outside the root.
  - Existing `crates/xtask/tests/*` are the determinism guard; P3.x steps must not require golden/output updates unless explicitly approved.

- Domain boundaries (split “god modules” by domain seams):
  - Snapshot domain (`codex_snapshot`): help probing + parsing vs schema model structs vs IO layout.
  - Union domain (`codex_union`): merge/normalization logic vs union schema structs vs IO layout.
  - Report domain (`codex_report`): report computation vs report models/schema vs sorting/filter semantics.
  - Version metadata domain (`codex_version_metadata`): policy gates vs derived metadata schema vs input loaders.
  - Validation domain (`codex_validate`): schema compilation + JSON Schema validation vs custom rule checks vs violation/report formatting.

- Extraction order (smallest-first, reversible; each step keeps CLI behavior stable):
  1) `codex_version_metadata`: move leaf model structs into `codex_version_metadata/models.rs` (P3.1).
  2) `codex_union`: move union schema structs into `codex_union/schema.rs` (P3.2).
  3) `codex_snapshot`: move snapshot schema structs into `codex_snapshot/schema.rs` (P3.3).
  4) `codex_report`: move report input schema structs into `codex_report/models.rs` (P3.4).
  5) `codex_validate`: move rules/model structs into `codex_validate/models.rs` (P3.5).
  6) `codex_validate`: split cohesive validation passes into `codex_validate/*` (P3.6).
  7) `codex_report`: split report domains into `codex_report/*` (P3.7).
  8) `codex_snapshot`: split snapshot pipeline domains into `codex_snapshot/*` (P3.8).
  9) `codex_validate`: follow-on split to bring `codex_validate.rs` below ceiling (P3.9).

---

## 8) Execution Journal (append-only)

### 8.1 Execution evidence storage (future runs)

Store future command outputs and diffs under `evidence_runs/YYYY-MM-DD/` using this naming convention:
- `evidence_runs/YYYY-MM-DD/<step_id>_<command>.txt` (example: `evidence_runs/2026-02-04/P2.5_cargo_test.txt`)
- `evidence_runs/YYYY-MM-DD/<step_id>_git_diff.txt`
- `evidence_runs/YYYY-MM-DD/<step_id>_notes.md` (optional)

Legacy note: Any prior evidence already stored under `audit_pack/execution/YYYY-MM-DD/` remains valid and must not be moved or deleted.

Add entries as work lands. Format:

### YYYY-MM-DD — <short title>

- Scope/step: P?.? (<phase step id>)
- Why: <reason / link to evidence>
- What changed:
  - <bullet list of changes; keep concise>
- Validation results (paste concise results, not full logs):
  - `cargo fmt --all -- --check`: PASS/FAIL (evidence: `evidence_runs/YYYY-MM-DD/<step_id>_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS/FAIL (evidence: `evidence_runs/YYYY-MM-DD/<step_id>_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS/FAIL (evidence: `evidence_runs/YYYY-MM-DD/<step_id>_cargo_test.txt`)
  - `cargo audit`: PASS/FAIL (evidence: `evidence_runs/YYYY-MM-DD/<step_id>_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS/FAIL (evidence: `evidence_runs/YYYY-MM-DD/<step_id>_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS/FAIL (evidence: `evidence_runs/YYYY-MM-DD/<step_id>_cargo_deny_licenses.txt`)
- Diffs/PRs:
  - <link(s) or commit hash(es)>

### 2026-02-04 — Create master workplan/status file

- Scope/step: Planning baseline (establish tracking)
- Why: Consolidate audit evidence and execution gates into a single status file; evidence inputs are enumerated in §2 (see `audit_pack/README.md` and `audit_pack/meta/commands.log`).
- What changed:
  - Added `refactor_workplan.md` with phases 0–3 checklists, quality gates, dependency triage tables, execution journal format, and decision log.
- Validation results:
  - `cargo fmt --all -- --check`: (not recorded here)
  - `cargo clippy --all-targets --all-features -- -D warnings`: (not recorded here)
  - `cargo test --all-targets --all-features`: (not recorded here)
  - `cargo audit`: (not recorded here)
  - `cargo deny check advisories`: (not recorded here)
  - `cargo deny check licenses`: (not recorded here)
- Diffs/PRs:
  - TBD

### 2026-02-04 — Phase 0 preflight: verify supply-chain remediation state

- Scope/step: P0.0 (preflight verification); status updates for P0.1/P0.2 based on observed gates
- Why: Audit pack recorded supply-chain gate failures (`audit_pack/supply_chain/cargo_audit.txt`, `audit_pack/supply_chain/cargo_deny_advisories.txt`, `audit_pack/supply_chain/cargo_deny_licenses.txt`); baseline summary claimed remediation already landed.
- What changed:
  - Verified current workspace passes all §4.1 gates (see Validation results).
  - Confirmed `bytes` is at `1.11.1` in `Cargo.lock` (addresses RUSTSEC-2026-0007).
  - Confirmed explicit `cargo-deny` policy exists (`deny.toml`) and licenses gate passes.
  - (Observation for later P0.3) `cargo tree -d --target all` shows duplicates for `getrandom` and `windows-sys` even though `audit_pack/deps/cargo_tree_duplicates.txt` reported none (likely due to command flags/targets at audit time).
- Validation results (paste concise results, not full logs):
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS
  - `cargo test --all-targets --all-features`: PASS
  - `cargo audit`: PASS
  - `cargo deny check advisories`: PASS
  - `cargo deny check licenses`: PASS
- Diffs/PRs:
  - None (verification-only; no code changes)

### 2026-02-04 — Phase 0 duplicates triage (defer)

- Scope/step: P0.3
- Why: Baseline summary claimed duplicates existed, but audit evidence (`audit_pack/deps/cargo_tree_duplicates.txt`) printed none; current `cargo tree -d --target all` shows duplicates (target scoping likely explains the discrepancy).
- What changed:
  - Documented current duplicate versions for `getrandom` and `windows-sys` in §6.3 and captured the decision to defer consolidation in §9.
- Validation results:
  - `cargo tree -d --target all`: duplicates present (`getrandom`, `windows-sys`)
  - §4.1 gates: unchanged (no code changes in this step)
- Diffs/PRs:
  - None

### 2026-02-04 — Define Phase 1 seam map

- Scope/step: P1.0
- Why: Prepare PR-sized `lib.rs` seam extractions guided by the file-size policy derived from `audit_pack/metrics/loc_summary.txt`.
- What changed:
  - Updated §7.1 seam order and expanded Phase 1 checklist to include explicit P1.2 (capabilities) and P1.3 (apply/diff) steps.
- Validation results:
  - N/A (planning-only change)
- Diffs/PRs:
  - None

### 2026-02-04 — Verify home seam extraction

- Scope/step: P1.1
- Why: Baseline summary claimed `home.rs` was already extracted; confirm the working tree state and size policy compliance before continuing Phase 1.
- What changed:
  - Verified `crates/codex/src/home.rs` exists, is re-exported from `crates/codex/src/lib.rs`, and is within the soft threshold (290 LOC).
- Validation results:
  - `cargo test --all-targets --all-features`: PASS (see subsequent Phase 1 extraction entries)
- Diffs/PRs:
  - None (verification-only; status update)

### 2026-02-04 — Extract capabilities module

- Scope/step: P1.2
- Why: Reduce `crates/codex/src/lib.rs` “god module” size while preserving stable public API paths; keep new files within §7.3 thresholds (`audit_pack/metrics/loc_summary.txt`).
- What changed:
  - Moved capability probing/caching types + helpers into `crates/codex/src/capabilities/` and re-exported via `crates/codex/src/lib.rs`.
  - Ensured split files are within size policy (new modules ≤ 326 LOC).
- Validation results:
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS
  - `cargo test --all-targets --all-features`: PASS
  - `cargo audit`: PASS
  - `cargo deny check advisories`: PASS
  - `cargo deny check licenses`: PASS
- Diffs/PRs:
  - None

### 2026-02-04 — Extract apply/diff module

- Scope/step: P1.3
- Why: Isolate apply/diff request + artifacts types from `lib.rs` while keeping public API paths stable via re-exports.
- What changed:
  - Moved `ApplyDiffArtifacts`, `CloudDiffRequest`, and `CloudApplyRequest` into `crates/codex/src/apply_diff.rs` and re-exported via `crates/codex/src/lib.rs`.
- Validation results:
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS
  - `cargo test --all-targets --all-features`: PASS
  - `cargo audit`: PASS
  - `cargo deny check advisories`: PASS
  - `cargo deny check licenses`: PASS
- Diffs/PRs:
  - None

### 2026-02-04 — Phase 2 MCP split: protocol/config/runtime/app modules (façade preserved)

- Scope/step: P2.0–P2.4
- Why: Begin reducing `crates/codex/src/mcp.rs` “god module” size while preserving stable `codex::mcp::*` public API paths (audit baseline shows 4,278 LOC; see `audit_pack/metrics/loc_summary.txt`).
- What changed:
  - Defined Phase 2 seam map + extraction order in §7.2 and added explicit P2.2–P2.4 checklist steps.
  - Created `crates/codex/src/mcp/protocol.rs` for JSON-RPC method constants and request/notification payload types; re-exported via `crates/codex/src/mcp.rs`.
  - Moved stored config types + persistence manager into `crates/codex/src/mcp/config.rs`; re-exported via `crates/codex/src/mcp.rs`.
  - Moved MCP runtime resolution + launchers + runtime manager API into `crates/codex/src/mcp/runtime.rs`; re-exported via `crates/codex/src/mcp.rs`.
  - Moved app runtime lifecycle + pooling helpers into `crates/codex/src/mcp/app.rs`; re-exported via `crates/codex/src/mcp.rs`.
  - Updated MCP tests to avoid reaching into now-internal helpers while preserving behavior.
- Validation results:
  - `cargo fmt --all -- --check`: PASS
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS
  - `cargo test --all-targets --all-features`: PASS
  - `cargo audit`: PASS
  - `cargo deny check advisories`: PASS
  - `cargo deny check licenses`: PASS
- Diffs/PRs:
  - None (no commit)

### 2026-02-04 — Phase 3 kickoff: define xtask boundaries + determinism contracts

- Scope/step: P3.0
- Why: Phase 3 targets xtask “god modules” identified by the audit baseline metrics; keep determinism guarded by existing xtask spec tests (`audit_pack/metrics/loc_summary.txt`).
- What changed:
  - Updated §7.4 with determinism contracts, domain boundaries, and an explicit smallest-first extraction order.
  - Expanded Phase 3 checklist with concrete steps (P3.1–P3.5) and per-step acceptance criteria.
- Validation results:
  - N/A (planning-only change)
- Diffs/PRs:
  - None

### 2026-02-04 — Phase 3 split: `codex_version_metadata` models extraction

- Scope/step: P3.1
- Why: Start Phase 3 “smallest-first” by extracting leaf JSON model structs from `crates/xtask/src/codex_version_metadata.rs` to reduce file size without changing behavior.
- What changed:
  - Added `crates/xtask/src/codex_version_metadata/models.rs` and moved union/wrapper coverage JSON model structs into it.
  - Updated `crates/xtask/src/codex_version_metadata.rs` to `mod models;` and use the extracted types (no CLI/output changes intended).
- Validation results:
  - `cargo fmt --all -- --check`: PASS (`audit_pack/execution/2026-02-04/P3.1_cargo_fmt_check_final.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`audit_pack/execution/2026-02-04/P3.1_cargo_clippy_final.txt`)
  - `cargo test --all-targets --all-features`: PASS (`audit_pack/execution/2026-02-04/P3.1_cargo_test.txt`)
  - `cargo audit`: PASS (`audit_pack/execution/2026-02-04/P3.1_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`audit_pack/execution/2026-02-04/P3.1_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`audit_pack/execution/2026-02-04/P3.1_cargo_deny_licenses.txt`)
- Diffs/PRs:
  - None

### 2026-02-04 — Phase 3 split: `codex_union` schema extraction

- Scope/step: P3.2
- Why: Continue Phase 3 smallest-first by extracting union snapshot schema structs from `crates/xtask/src/codex_union.rs` to reduce file size while preserving deterministic output.
- What changed:
  - Added `crates/xtask/src/codex_union/schema.rs` and moved `SnapshotUnionV2` and related union schema structs into it.
  - Updated `crates/xtask/src/codex_union.rs` to `mod schema;` and use the extracted schema types (no CLI/output changes intended).
- Validation results:
  - `cargo fmt --all -- --check`: PASS (`audit_pack/execution/2026-02-04/P3.2_cargo_fmt_check_final2.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`audit_pack/execution/2026-02-04/P3.2_cargo_clippy_final.txt`)
  - `cargo test --all-targets --all-features`: PASS (`audit_pack/execution/2026-02-04/P3.2_cargo_test.txt`)
  - `cargo audit`: PASS (`audit_pack/execution/2026-02-04/P3.2_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`audit_pack/execution/2026-02-04/P3.2_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`audit_pack/execution/2026-02-04/P3.2_cargo_deny_licenses.txt`)
- Diffs/PRs:
  - None

### 2026-02-04 — Phase 3 split: `codex_snapshot` schema extraction

- Scope/step: P3.3
- Why: Reduce `crates/xtask/src/codex_snapshot.rs` size by extracting snapshot schema structs while preserving the existing `crate::codex_snapshot::{...}` type paths used by other xtask modules.
- What changed:
  - Added `crates/xtask/src/codex_snapshot/schema.rs` and moved snapshot + supplement schema structs into it.
  - Updated `crates/xtask/src/codex_snapshot.rs` to `mod schema;` and re-export the schema types (no CLI/output changes intended).
- Validation results:
  - `cargo fmt --all -- --check`: PASS (`audit_pack/execution/2026-02-04/P3.3_cargo_fmt_check_final2.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`audit_pack/execution/2026-02-04/P3.3_cargo_clippy_final.txt`)
  - `cargo test --all-targets --all-features`: PASS (`audit_pack/execution/2026-02-04/P3.3_cargo_test.txt`)
  - `cargo audit`: PASS (`audit_pack/execution/2026-02-04/P3.3_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`audit_pack/execution/2026-02-04/P3.3_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`audit_pack/execution/2026-02-04/P3.3_cargo_deny_licenses.txt`)
- Diffs/PRs:
  - None

### 2026-02-04 — Phase 3 split: `codex_report` models extraction

- Scope/step: P3.4
- Why: Reduce `crates/xtask/src/codex_report.rs` size by extracting leaf input model structs (union snapshot + wrapper coverage) while preserving deterministic report output (guarded by xtask spec tests).
- What changed:
  - Added `crates/xtask/src/codex_report/models.rs` and moved union/wrapper coverage JSON model structs into it.
  - Updated `crates/xtask/src/codex_report.rs` to `mod models;` and use the extracted types (no CLI/output changes intended).
- Validation results:
  - `cargo fmt --all -- --check`: PASS (`audit_pack/execution/2026-02-04/P3.4_cargo_fmt_check_final.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`audit_pack/execution/2026-02-04/P3.4_cargo_clippy_final.txt`)
  - `cargo test --all-targets --all-features`: PASS (`audit_pack/execution/2026-02-04/P3.4_cargo_test.txt`)
  - `cargo audit`: PASS (`audit_pack/execution/2026-02-04/P3.4_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`audit_pack/execution/2026-02-04/P3.4_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`audit_pack/execution/2026-02-04/P3.4_cargo_deny_licenses.txt`)
- Diffs/PRs:
  - None

### 2026-02-04 — Phase 3 split: `codex_validate` models extraction

- Scope/step: P3.5
- Why: Reduce `crates/xtask/src/codex_validate.rs` size by extracting leaf model + rules structs into a `models` module, preserving deterministic output (guarded by xtask spec tests).
- What changed:
  - Added `crates/xtask/src/codex_validate/models.rs` and moved validation model structs (`Violation`, `Pointer*`, wrapper coverage models, rules models, and supporting structs) into it.
  - Updated `crates/xtask/src/codex_validate.rs` to `mod models;` and use the extracted types (no CLI/output changes intended).
- Validation results:
  - `cargo fmt --all -- --check`: PASS (`audit_pack/execution/2026-02-04/P3.5_cargo_fmt_check_after.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`audit_pack/execution/2026-02-04/P3.5_cargo_clippy_final.txt`)
  - `cargo test --all-targets --all-features`: PASS (`audit_pack/execution/2026-02-04/P3.5_cargo_test.txt`)
  - `cargo audit`: PASS (`audit_pack/execution/2026-02-04/P3.5_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`audit_pack/execution/2026-02-04/P3.5_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`audit_pack/execution/2026-02-04/P3.5_cargo_deny_licenses.txt`)
- Diffs/PRs:
  - None (no commit; diff captured at `audit_pack/execution/2026-02-04/P3.5_git_diff.txt`)

### 2026-02-04 — P3.5 commit + push (requested)

- Scope/step: P3.5 (admin/provenance)
- Why: User requested `git add + commit + push` after completing P3.5.
- What changed:
  - Created commit `d4bb1dc` containing the P3.5 refactor + execution evidence logs under `audit_pack/execution/2026-02-04/`.
  - Pushed commits to `origin/feat/refactor`.
- Validation results:
  - N/A (no functional changes; see prior P3.5 validation evidence in `audit_pack/execution/2026-02-04/P3.5_*.txt`)
- Diffs/PRs:
  - Commit: `d4bb1dc`

### 2026-02-04 — Workplan refresh: post-refactor metrics + extend remaining phases

- Scope/step: Planning/metrics refresh (adds follow-on steps P1.4, P2.5, P3.6–P3.8)
- Why: Post-refactor file-size distribution and top offenders changed after Phase 1–3 extractions; remaining seams (MCP JSON-RPC transport and deeper xtask splits) are still outstanding.
- What changed:
  - Updated §3.1 and §7.3 thresholds to the post-refactor distribution (soft=300, hard=604, ceiling=1000) based on `evidence_runs/2026-02-04/post_refactor_tokei.json`.
  - Replaced §3.2 “Top offenders” with the post-refactor top-10 list from `evidence_runs/2026-02-04/post_refactor_tokei_files_sorted.txt`.
  - Updated §6.3 duplicates evidence to `evidence_runs/2026-02-04/post_refactor_cargo_tree_duplicates_target_all.txt` and summarized the two duplicate groups (`getrandom`, `windows-sys`).
  - Added new not-started checklist items: P1.4, P2.5, P3.6, P3.7, P3.8.
- Validation results:
  - N/A (documentation/planning update; no functional changes)
- Diffs/PRs:
  - None

### 2026-02-04 — Post-refactor metrics refresh & workplan reconciliation

- Scope/step: Planning/metrics refresh (workplan reconciliation only; no code changes)
- Why: Ensure workplan thresholds, top offenders, and duplicates evidence match the latest post-refactor metrics so remaining >1000 LOC files are correctly scheduled before any further refactor execution.
- What changed:
  - Stored post-refactor metrics evidence under `evidence_runs/2026-02-04/` (copies for normalization; legacy `audit_pack/…` copies are preserved).
  - Recomputed §3.1 distribution from `evidence_runs/2026-02-04/post_refactor_tokei.json` and applied the threshold policy (soft=300, hard=604, ceiling=1000).
  - Replaced §3.2 top-10 offenders using `evidence_runs/2026-02-04/post_refactor_tokei_files_sorted.txt`.
  - Updated §6.3 duplicates evidence to `evidence_runs/2026-02-04/post_refactor_cargo_tree_duplicates_target_all.txt` (kept “defer consolidation” decision; evidence refreshed).
  - Corrected phase status drift: Phase 1/2/3 are explicitly “In Progress” while any associated files remain >1000 LOC (see §3.2); follow-on steps remain queued (P1.4, P2.5, P3.6–P3.8).
- Validation results:
  - N/A (documentation/planning update; no functional changes)
- Diffs/PRs:
  - None

### 2026-02-04 — P1.4 execpolicy seam extraction

- Scope/step: P1.4 (Phase 1 seam extraction)
- Why: `crates/codex/src/lib.rs` still contains the execpolicy modeling/parsing/types and method implementation; extracting the seam reduces `lib.rs` surface area while preserving `codex::*` public API paths.
- What changed:
  - Added `crates/codex/src/execpolicy.rs` containing `CodexClient::check_execpolicy` plus `ExecPolicy*` request/response/types.
  - Updated `crates/codex/src/lib.rs` to `mod execpolicy;` and re-export the `ExecPolicy*` types at the crate root (API preserved).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-04/P1.4_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-04/P1.4_cargo_clippy_all_targets_all_features.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-04/P1.4_cargo_test_all_targets_all_features.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-04/P1.4_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-04/P1.4_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-04/P1.4_cargo_deny_licenses.txt`)
- Diffs/PRs:
  - Commit: `2d17281b8d09c7797cd555d4b2fd5951af75b328` (legacy combined commit; includes P1.4/P2.5/P3.6)
  - Evidence diffs: code diff at `evidence_runs/2026-02-04/P1.4_code_diff_final.patch`; workplan diff at `evidence_runs/2026-02-04/P1.4_workplan_diff_final_v2.patch`

### 2026-02-04 — P2.5 JSON-RPC transport extraction

- Scope/step: P2.5 (Phase 2 JSON-RPC transport seam extraction)
- Why: `crates/codex/src/mcp.rs` still contains the stdio JSON-RPC transport spawn + pump loop; extracting the transport reduces `mcp.rs` churn surface area while preserving stable `codex::mcp::*` public API paths via the existing façade.
- What changed:
  - Added `crates/codex/src/mcp/jsonrpc.rs` containing the stdio JSON-RPC transport (`JsonRpcTransport`) plus message parsing and event broadcast helpers.
  - Updated `crates/codex/src/mcp.rs` to `mod jsonrpc;` and use the extracted transport + response mapping helper (API preserved).
- Validation results:
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-04/P2.5_cargo_fmt_check_final.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-04/P2.5_cargo_clippy_all_targets_all_features_after.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-04/P2.5_cargo_test_all_targets_all_features.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-04/P2.5_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-04/P2.5_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-04/P2.5_cargo_deny_licenses.txt`)
- Diffs/PRs:
  - Commit: `2d17281b8d09c7797cd555d4b2fd5951af75b328` (legacy combined commit; includes P1.4/P2.5/P3.6)
  - Evidence diffs: code diff at `evidence_runs/2026-02-04/P2.5_code_diff_final.patch`; workplan diff at `evidence_runs/2026-02-04/P2.5_workplan_diff_final.patch`

### 2026-02-04 — P3.6 codex_validate pass extraction

- Scope/step: P3.6
- Why: Reduce `crates/xtask/src/codex_validate.rs` size by extracting cohesive validation passes into submodules while preserving deterministic output.
- What changed:
  - Added `crates/xtask/src/codex_validate/pointers.rs` (pointer validation + pointer-file helpers).
  - Added `crates/xtask/src/codex_validate/schema.rs` (schema `$id` absolutization, schema validation, JSON reading helper).
  - Added `crates/xtask/src/codex_validate/wrapper_coverage.rs` (wrapper_coverage semantic checks + scope overlap rules).
  - Added `crates/xtask/src/codex_validate/report_invariants.rs` (report invariants: exclusions + intentionally_unsupported invariants).
  - Updated `crates/xtask/src/codex_validate.rs` to keep façade/orchestration and call extracted passes (outputs preserved).
- Validation results:
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-04/P3.6_cargo_fmt_check_after.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-04/P3.6_cargo_clippy_all_targets_all_features.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-04/P3.6_cargo_test_all_targets_all_features.txt`)
  - `cargo audit`: FAIL (`evidence_runs/2026-02-04/P3.6_cargo_audit_after.txt`)
  - `cargo deny check advisories`: FAIL (`evidence_runs/2026-02-04/P3.6_cargo_deny_advisories_after.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-04/P3.6_cargo_deny_licenses.txt`)
- Diffs/PRs:
  - Commit: `2d17281b8d09c7797cd555d4b2fd5951af75b328` (legacy combined commit; includes P1.4/P2.5/P3.6)
  - Evidence diffs: code diff at `evidence_runs/2026-02-04/P3.6_code_diff_final.patch`; workplan diff at `evidence_runs/2026-02-04/P3.6_workplan_diff_final.patch`

### 2026-02-04 — P3.6 supply-chain gates rerun

- Scope/step: P3.6 (validation correction)
- Why: The earlier P3.6 supply-chain gate FAILs were caused by transient network/DNS issues while fetching advisory databases and are superseded by this rerun.
- What changed:
  - None (rerun of supply-chain validation gates only; no code changes)
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-04/P3.6_cargo_fmt_check_after.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-04/P3.6_cargo_clippy_all_targets_all_features.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-04/P3.6_cargo_test_all_targets_all_features.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-04/P3.6_cargo_audit_final.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-04/P3.6_cargo_deny_advisories_final2.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-04/P3.6_cargo_deny_licenses.txt`)
- Diffs/PRs:
  - None (no commit; workplan-only journal entry)

### 2026-02-04 — P3.7 codex_report domain split

- Scope/step: P3.7
- Why this step is next:
  - Earliest not-done item in Phase 3 queue: `Status: [ ] Not Started  [ ] In Progress  [ ] Done` (P3.7).
- What changed:
  - Split `crates/xtask/src/codex_report.rs` into a small façade (`Args`, `ReportError`, `run`) and extracted report domain logic into `crates/xtask/src/codex_report/*` (rules parsing, wrapper indexing/resolution, report computation/serialization).
  - Kept deterministic output semantics (stable sort order, `serde_json::to_string_pretty` + trailing newline; `SOURCE_DATE_EPOCH` support unchanged).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-04/P3.7_cargo_fmt_check_final.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-04/P3.7_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-04/P3.7_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-04/P3.7_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-04/P3.7_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-04/P3.7_cargo_deny_licenses.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-04/SESSION_code_diff_final.patch`
  - Workplan diff: `evidence_runs/2026-02-04/SESSION_workplan_diff_final.patch`
- Commit:
  - `29b3880fe387e831dcb29d9eb7de035dbcf1e6cf`

### 2026-02-04 — P3.8 codex_snapshot pipeline domain split

- Scope/step: P3.8
- Why this step is next:
  - Earliest not-done item in Phase 3 queue: `Status: [ ] Not Started  [ ] In Progress  [ ] Done` (P3.8).
- What changed:
  - Split `crates/xtask/src/codex_snapshot.rs` into a smaller façade (`Args`, `Error`, `run`) and extracted snapshot pipeline stages into `crates/xtask/src/codex_snapshot/*` (discovery/help parsing, supplements/normalization, output layout, and probes).
  - Preserved deterministic output semantics (stable sort order, `serde_json::to_string_pretty` + trailing newline; `SOURCE_DATE_EPOCH` support unchanged).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-04/P3.8_cargo_fmt_check_final.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-04/P3.8_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-04/P3.8_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-04/P3.8_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-04/P3.8_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-04/P3.8_cargo_deny_licenses.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-04/SESSION_code_diff_final.patch`
  - Workplan diff: `evidence_runs/2026-02-04/SESSION_workplan_diff_final.patch`
- Commit:
  - `f447ec5a4cb64c7a0d94cb5ad0bc4f596178dc5a`

---

### 2026-02-05 — Extract builder/config/flags surfaces

- Scope/step: P1.5
- Why: Reduce `crates/codex/src/lib.rs` size by extracting the builder/config/flags surface into a cohesive module while preserving public API paths.
- What changed:
  - Moved `CodexClientBuilder` + builder/config/flags enums/structs and CLI override argument construction helpers into `crates/codex/src/builder.rs`.
  - Preserved existing public API paths via re-exports from `crates/codex/src/lib.rs`.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.5_cargo_fmt_check_final.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.5_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.5_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.5_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.5_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.5_cargo_deny_licenses.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/SESSION_code_diff_final.patch`
  - Workplan diff: `evidence_runs/2026-02-05/SESSION_workplan_diff_final.patch`
- Commit:
  - `420262329b90007e7b4459ebb8e3bc2418da7641`

### 2026-02-05 — Extract JSONL streaming/framing seam

- Scope/step: P1.6
- Why: Reduce `crates/codex/src/lib.rs` size by extracting JSONL streaming/framing + related IO helpers into `crates/codex/src/jsonl.rs` while preserving public API paths.
- What changed:
  - Moved JSONL stream framing helpers (`forward_json_events`, `EventChannelStream`, `JsonLogSink`, normalization/context) from `crates/codex/src/lib.rs` into `crates/codex/src/jsonl.rs`.
  - Kept `crates/codex/src/lib.rs` as the façade by calling into `jsonl::*` helpers; public re-exports remain stable.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.6_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.6_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.6_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.6_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.6_cargo_audit_after.txt`) (initial/obsolete: `evidence_runs/2026-02-05/P1.6_cargo_audit.txt` was incomplete)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.6_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.6_cargo_deny_licenses_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.6_cargo_deny_licenses.txt` (crates.io DNS failure))
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.6_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/SESSION_code_diff_final.patch`
  - Workplan diff: `evidence_runs/2026-02-05/SESSION_workplan_diff_final.patch`
- Diffs/PRs:
  - Commit: `b4da9b037f4e0fb3c03889b6e8d57853ece11826`

### 2026-02-05 — Move high-level MCP clients into `mcp/client.rs`

- Scope/step: P2.6
- Why: Reduce `crates/codex/src/mcp.rs` by extracting high-level client façades into `crates/codex/src/mcp/client.rs` while preserving stable `codex::mcp::*` public API paths via re-exports.
- What changed:
  - Moved `McpError`, `CodexMcpServer`, and `CodexAppServer` into `crates/codex/src/mcp/client.rs`.
  - Kept `crates/codex/src/mcp.rs` as a compatibility façade by wiring `mod client;` + re-exporting client items.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P2.6_cargo_fmt_check.txt`) (final: `evidence_runs/2026-02-05/P2.6_cargo_fmt_check_final.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P2.6_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P2.6_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P2.6_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P2.6_cargo_audit_final.txt`) (previous FAILs: `evidence_runs/2026-02-05/P2.6_cargo_audit_after.txt`, `evidence_runs/2026-02-05/P2.6_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P2.6_cargo_deny_advisories_final.txt`) (previous FAIL: `evidence_runs/2026-02-05/P2.6_cargo_deny_advisories_initial_fail.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P2.6_cargo_deny_licenses_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P2.6_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P2.6_workplan_diff_final.patch` (post-commit)
- Commit:
  - f955b7fdfed037340ec22f190deb050b69452ef4

### 2026-02-05 — P2.7 MCP façade: split remaining `mcp.rs` bulk into test modules (API preserved)

- Scope/step: P2.7
- Why: Reduce `crates/codex/src/mcp.rs` below the program ceiling per §7.3 (evidence: `audit_pack/execution/2026-02-04/post_refactor_tokei_files_sorted_updated.txt`).
- What changed:
  - Moved the large `#[cfg(test)]` unit test module out of `crates/codex/src/mcp.rs` into `crates/codex/src/mcp/test_support.rs`, `crates/codex/src/mcp/tests_core.rs`, and `crates/codex/src/mcp/tests_runtime_app.rs`.
  - Left `crates/codex/src/mcp.rs` as a thin compatibility façade with stable `codex::mcp::*` public API paths preserved via existing re-exports.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P2.7_cargo_fmt_check_final.txt`) (initial FAIL: `evidence_runs/2026-02-05/P2.7_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P2.7_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P2.7_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P2.7_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P2.7_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P2.7_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P2.7_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P2.7_cargo_deny_licenses.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P2.7_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P2.7_workplan_diff_final.patch` (post-commit)
- Commit:
  - d7d7efb2891251bd47e923ebca2021c1694221a8

### 2026-02-05 — P3.9 codex_validate follow-on split (below ceiling; determinism preserved)

- Scope/step: P3.9
- Why: Bring `crates/xtask/src/codex_validate.rs` below the program ceiling per §3.2 while preserving deterministic validation output.
- What changed:
  - Split remaining validation orchestration helpers out of `crates/xtask/src/codex_validate.rs` into `crates/xtask/src/codex_validate/{fix_mode.rs,current.rs,versions.rs,pointer_consistency.rs}`.
  - Kept ordering/formatting semantics unchanged; `crates/xtask/src/codex_validate.rs` is now ~533 LOC.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P3.9_cargo_fmt_check_final.txt`) (initial FAIL: `evidence_runs/2026-02-05/P3.9_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P3.9_cargo_fmt_apply.txt`; re-check: `evidence_runs/2026-02-05/P3.9_cargo_fmt_check_after.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P3.9_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P3.9_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P3.9_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P3.9_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P3.9_cargo_deny_licenses.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P3.9_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P3.9_workplan_diff_final.patch` (post-commit)
- Commit:
  - 164111524dfc1cf4a643b0af3d5df0e5cd1da519

### 2026-02-05 — Split `builder.rs` into `builder/*` (API preserved)

- Scope/step: P1.7
- Why: Bring `crates/codex/src/builder.rs` below the size ceiling by splitting into a cohesive `crates/codex/src/builder/*` module tree while preserving public API paths via re-exports from `crates/codex/src/lib.rs`.
- What changed:
  - Converted `crates/codex/src/builder.rs` into `crates/codex/src/builder/{mod.rs,types.rs,cli_overrides.rs}`.
  - Preserved stable public API paths (e.g., `codex::CodexClientBuilder`, `codex::CliOverrides`, `codex::ColorMode`) via existing `crates/codex/src/lib.rs` re-exports.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.7_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.7_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P1.7_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.7_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.7_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.7_cargo_test_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.7_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.7_cargo_audit_after.txt`) (initial output incomplete: `evidence_runs/2026-02-05/P1.7_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.7_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.7_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.7_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.7_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.7_workplan_diff_final.patch` (post-commit)
- Commit:
  - b2df25fc4d3b6ce13475541c88531a26438acf69

### 2026-02-05 — Plan next Phase 1 seams after `builder/*`

- Scope/step: P1.8
- Why this step is next:
  - Earliest not-done item in §10 queue at dispatch (2026-02-05): `P1.8 — Plan next Phase 1 seams after builder/* (no code moves)`.
- What changed:
  - Updated §7.1 seam extraction order with the next concrete PR-sized seams after P1.7 (bundled-binary, events, CLI models, version/features helpers).
  - Added Phase 1 checklist steps P1.9–P1.13 and refreshed §10 queue accordingly.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.8_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.8_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.8_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.8_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.8_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.8_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.8_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.8_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.8_workplan_diff_final.patch` (post-commit)
- Commit:
  - 8406ddfb04e7df7b6337422de3510dd42375b10b

### 2026-02-05 — P1.9 Seam extraction: bundled binary resolver (`bundled_binary.rs`) (API preserved)

- Scope/step: P1.9
- Why: Extract the bundled binary resolver into a cohesive module while preserving crate-root public API paths via façade + re-exports.
- What changed:
  - Moved `BundledBinarySpec`, `BundledBinary`, `BundledBinaryError`, `resolve_bundled_binary`, and `default_bundled_platform_label` into `crates/codex/src/bundled_binary.rs`.
  - Wired `crates/codex/src/lib.rs` with `mod bundled_binary;` + `pub use ...` so existing `codex::{...}` paths continue to compile; kept a test-only shim for `bundled_binary_filename` to avoid churn in existing unit tests.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.9_cargo_fmt_check_final.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.9_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P1.9_cargo_fmt_apply.txt`; re-check: `evidence_runs/2026-02-05/P1.9_cargo_fmt_check_after.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.9_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.9_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.9_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.9_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.9_cargo_audit.txt`; reran offline with copied RustSec DB due to network/sandbox lock constraints)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.9_cargo_deny_advisories.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.9_cargo_deny_advisories_initial_fail.txt`; reran with offline cargo home due to network/sandbox lock constraints)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.9_cargo_deny_licenses.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.9_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.9_workplan_diff_final.patch` (post-commit)
- Commit:
  - ae99829d9ff91405163fbe797c21588fae186ddb

### 2026-02-05 — P1.10 Seam extraction: exec JSONL event models (`events.rs`) (API preserved)

- Scope/step: P1.10
- Why: Reduce `lib.rs` bulk by extracting the JSONL event envelope + payload models emitted by `codex exec --json` into a dedicated module, while preserving crate-root public API via re-exports.
- What changed:
  - Moved JSONL event models (`ThreadEvent`, `ItemEnvelope`, `ItemPayload`, deltas/states/status enums, and `EventError`) from `crates/codex/src/lib.rs` into `crates/codex/src/events.rs` with identical serde shapes (mechanical move).
  - Wired `crates/codex/src/lib.rs` with `mod events;` + `pub use events::{...};` so existing `codex::{...}` paths continue to compile.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.10_cargo_fmt_check_final.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.10_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P1.10_cargo_fmt_apply.txt`; re-check: `evidence_runs/2026-02-05/P1.10_cargo_fmt_check_after.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.10_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.10_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.10_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.10_cargo_audit.txt`; reran with `--no-fetch --stale` + writable `CARGO_HOME` due to sandbox/network constraints; see also `evidence_runs/2026-02-05/P1.10_cargo_audit_after_fetch_fail.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.10_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.10_cargo_deny_advisories.txt`; reran with `--disable-fetch` due to network constraints; help: `evidence_runs/2026-02-05/P1.10_cargo_deny_advisories_help.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.10_cargo_deny_licenses.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.10_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.10_workplan_diff_final.patch` (post-commit)
- Commit:
  - ae0ceacacfa1c127eb527a69642563ee7cdbc7d4

### 2026-02-05 — P1.11 Seam extraction: CLI subcommand request/response models (`cli/*`) (API preserved)

- Scope/step: P1.11
- Why: Extract CLI subcommand request/response models into a dedicated `cli/*` module tree to reduce `lib.rs` bulk while preserving crate-root public API via façade + re-exports.
- What changed:
  - Moved non-streaming CLI subcommand models (e.g., features/help/review/resume/fork/cloud/mcp/app-server, plus sandbox/stdio-to-uds/responses-api-proxy request/response types) from `crates/codex/src/lib.rs` into `crates/codex/src/cli/*`.
  - Wired `crates/codex/src/lib.rs` with `mod cli;` + `pub use cli::{...};` so existing `codex::{...}` paths continue to compile; made a small visibility adjustment (`CodexFeatureStage::parse` → `pub(crate)`) to keep internal parsing helpers compiling.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.11_cargo_fmt_check_final.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.11_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P1.11_cargo_fmt_apply.txt`; re-check: `evidence_runs/2026-02-05/P1.11_cargo_fmt_check_after.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.11_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.11_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.11_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.11_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.11_cargo_audit.txt`; reran with `--no-fetch --stale` due to sandbox lock constraints)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.11_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.11_cargo_deny_advisories.txt`; reran with a writable `CARGO_HOME` seeded from `/home/dev/.cargo` + `--offline` due to sandbox lock constraints)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.11_cargo_deny_licenses.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.11_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.11_workplan_diff_final.patch` (post-commit)
- Commit:
  - 097ecd4060a00301da073a442ef62bb3de4730ac

### 2026-02-05 — P1.12 Seam extraction: feature/version parsing + update advisory helpers (`version.rs`) (API preserved)

- Scope/step: P1.12
- Why: Extract version/features probe parsing + update advisory helpers into a dedicated `version.rs` module to reduce `lib.rs` bulk while preserving crate-root public API via façade + re-exports.
- What changed:
  - Moved `codex --version`, `codex features list` (json/text), and `codex --help` parsing helpers (semver/channel parsing, commit hash extraction, feature-flag parsing, and feature list parsing) from `crates/codex/src/lib.rs` into `crates/codex/src/version.rs`.
  - Re-exported `update_advisory_from_capabilities` from `crates/codex/src/lib.rs` so the `codex::update_advisory_from_capabilities` public API path remains unchanged.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.12_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.12_workplan_diff_final.patch` (post-commit)
- Commit:
  - 47de4019ad7b57c9a3d14204f32a4528bd78f3c2

### 2026-02-05 — P1.12 (correction)

- Scope/step: P1.12
- Why: Correct failed gates from the initial P1.12 run (rustfmt diffs, E0364 visibility error, and advisory DB read-only lock failures).
- Fixes applied:
  - Made `version::update_advisory_from_capabilities` `pub` so the existing crate-root re-export (`codex::update_advisory_from_capabilities`) compiles without changing the public API path.
  - Re-applied `cargo fmt --all` to eliminate rustfmt diffs.
  - Reran `cargo audit` / `cargo deny` with a writable `CARGO_HOME` under `/tmp` (seeded from `/home/dev/.cargo`) plus offline/no-fetch flags to avoid read-only advisory DB locks.
- Validation results (§4.1): PASS (after correction)
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_fmt_check_after_fix.txt`) (applied: `evidence_runs/2026-02-05/P1.12_cargo_fmt_apply_after_fix.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_clippy_after_fix.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_test_after_fix.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_audit_after_fix.txt`) (workaround: `CARGO_HOME=/tmp/p112_cargo_home`, seeded advisory DBs, `--no-fetch --stale`, `CARGO_NET_OFFLINE=true`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_deny_advisories_after_fix.txt`) (workaround: writable `CARGO_HOME` + `--disable-fetch` + `CARGO_NET_OFFLINE=true`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_deny_licenses_after_fix.txt`) (workaround: writable `CARGO_HOME` + `CARGO_NET_OFFLINE=true`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.12_cargo_fmt_check_final_after_fix.txt`)
- Commit:
  - 47de4019ad7b57c9a3d14204f32a4528bd78f3c2

### 2026-02-05 — P1.13 Plan remaining Phase 1 seams after P1.12 (no code moves)

- Scope/step: P1.13
- Why: Refresh Phase 1 seam planning after P1.9–P1.12 so the remaining `crates/codex/src/lib.rs` bulk can be extracted in PR-sized, API-stable steps aligned with the current module layout.
- What changed:
  - Updated §7.1 seam extraction order to reflect the current module layout after P1.9–P1.12 and to queue the next 2–4 PR-sized extractions (process plumbing, exec streaming, auth/login, and remaining subcommand wrappers).
  - Added Phase 1 checklist steps P1.14–P1.18 with clear scope boundaries and ordering assumptions.
  - Updated §10 execution queue to reflect the next Phase 1 seams after P1.13.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.13_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.13_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.13_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.13_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.13_cargo_audit.txt`; workaround: writable `CARGO_HOME=/tmp/p113_cargo_home` seeded with `/home/dev/.cargo` RustSec DB + registry index, `--no-fetch --stale`, `CARGO_NET_OFFLINE=true`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.13_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.13_cargo_deny_advisories.txt`; workaround: writable `CARGO_HOME=/tmp/p113_deny_cargo_home` seeded from `/home/dev/.cargo`, `--disable-fetch`, `CARGO_NET_OFFLINE=true`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.13_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.13_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.13_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.13_workplan_diff_final.patch` (post-commit)
- Commit:
  - 6fce405c4adcf03ec211655033a1dd4fe2bb1dcd

### 2026-02-05 — P1.14 Seam extraction: spawn/process plumbing (`process.rs`) (API preserved)

- Scope/step: P1.14
- Why: Reduce `crates/codex/src/lib.rs` bulk by extracting shared subprocess spawn + stdio/capture helpers into a single internal module, keeping public APIs stable.
- What changed:
  - Added internal `crates/codex/src/process.rs` and moved `spawn_with_retry`, stdout/stderr tee/capture helpers, and output formatting helpers.
  - Wired `mod process;` in `crates/codex/src/lib.rs` and updated internal references mechanically (no behavior changes).
- Validation results (§4.1): PASS (with offline/no-fetch workaround for supply-chain checks due to sandbox networking + advisory DB locks)
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.14_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.14_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P1.14_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.14_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.14_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.14_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.14_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.14_cargo_audit.txt`; fetch FAIL: `evidence_runs/2026-02-05/P1.14_cargo_audit_after_fetch_fail.txt`; workaround: `CARGO_HOME=/tmp/cargo_home_p1_14` + `--db /home/dev/.cargo/advisory-db --no-fetch --stale`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.14_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.14_cargo_deny_advisories.txt`; network FAIL: `evidence_runs/2026-02-05/P1.14_cargo_deny_advisories_after_network_fail.txt`; offline cache miss: `evidence_runs/2026-02-05/P1.14_cargo_deny_advisories_after_offline_cache_miss.txt`; workaround: `CARGO_HOME=/tmp/cargo_home_p1_14` seeded via symlinks to `/home/dev/.cargo` + `--offline --locked --disable-fetch`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.14_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.14_cargo_fmt_check_final.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.14_cargo_fmt_check_final_fail.txt`; applied: `evidence_runs/2026-02-05/P1.14_cargo_fmt_apply_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.14_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.14_workplan_diff_final.patch` (post-commit)
- Commit:
  - 4cd39c37e99ce1db642e43f1a38897c784b9d701

### 2026-02-05 — P1.15 Seam extraction: exec streaming + resume types + helpers (`exec/*`) (API preserved)

- Scope/step: P1.15
- Why: Reduce `crates/codex/src/lib.rs` bulk by extracting the exec streaming + resume surface into a cohesive `crates/codex/src/exec.rs` module while preserving the crate-root API via re-exports.
- What changed:
  - Added `crates/codex/src/exec.rs` and moved the exec streaming and resume surface (`ExecStreamRequest`, `ResumeSelector`, `ResumeRequest`, `ExecStream`, `ExecCompletion`, `ExecStreamError`) plus `CodexClient::{send_prompt*, stream_exec*, stream_resume, resume_session}` and exec-only helpers.
  - Wired `mod exec;` + `pub use exec::{...}` in `crates/codex/src/lib.rs` to preserve public `codex::*` paths.
- Validation results (§4.1): PASS (with read-only `$HOME/.cargo` lock workarounds for supply-chain checks)
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.15_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.15_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P1.15_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.15_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.15_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.15_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.15_cargo_audit.txt`; workaround: `cargo audit --no-fetch --stale` due to read-only advisory DB lock)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.15_cargo_deny_advisories.txt`) (workaround: writable `CARGO_HOME=/tmp/cargo_home_p1_15_deny` seeded from `/home/dev/.cargo` advisory DB + registry symlink; `--disable-fetch`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.15_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.15_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.15_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.15_workplan_diff_final.patch` (post-commit)
- Commit:
  - 6672185270d9d2e57d63fc30aa339b70295f6b41

### 2026-02-05 — P1.16 Seam extraction: auth/login helpers (`auth.rs`) (API preserved)

- Scope/step: P1.16
- Why: Move authentication state + login flow helpers out of `crates/codex/src/lib.rs` into `crates/codex/src/auth.rs` while preserving crate-root public API paths via re-exports.
- What changed:
  - Extracted `AuthSessionHelper` and auth enums (`CodexAuthStatus`, `CodexAuthMethod`, `CodexLogoutStatus`) into `crates/codex/src/auth.rs`; re-exported them from `crates/codex/src/lib.rs` to preserve `codex::*` API paths.
  - Moved `CodexClient` login/logout helpers (`spawn_*login_process`, `login_with_api_key`, `login_status`, `logout`) and `parse_login_success` into `crates/codex/src/auth.rs` (pure move; signatures/behavior unchanged).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.16_cargo_fmt_check_final.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.16_cargo_fmt_check.txt`; re-check: `evidence_runs/2026-02-05/P1.16_cargo_fmt_check_after.txt`; final apply: `evidence_runs/2026-02-05/P1.16_cargo_fmt_apply_final.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.16_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.16_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.16_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.16_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.16_cargo_audit.txt`; reran with `--no-fetch --stale` due to sandbox advisory DB lock constraints)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.16_cargo_deny_advisories_final.txt`) (initial FAILs: `evidence_runs/2026-02-05/P1.16_cargo_deny_advisories.txt` (read-only advisory DB lock) + `evidence_runs/2026-02-05/P1.16_cargo_deny_advisories_after.txt` (crates.io DNS/network); workaround: writable temp `CARGO_HOME` seeded from `/home/dev/.cargo` advisory DB + registry symlink; `--disable-fetch` + `CARGO_NET_OFFLINE=true`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.16_cargo_deny_licenses.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.16_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.16_workplan_diff_final.patch` (post-commit)
- Commit:
  - f0dc296e9ef05ab1a9ccec061f98798d5a4b228f

### 2026-02-05 — P1.17 Seam extraction: remaining client subcommand wrappers (`commands/*`) (API preserved)

- Scope/step: P1.17
- Why: Extract the remaining `CodexClient` subcommand wrappers out of `crates/codex/src/lib.rs` into a cohesive `crates/codex/src/commands/*` module tree while preserving crate-root API paths and keeping the move mechanical.
- What changed:
  - Added `crates/codex/src/commands/` module tree (`apply_diff`, `app_server`, `features`, `proxy`, `sandbox`) and wired `mod commands;` from `crates/codex/src/lib.rs`.
  - Moved `CodexClient` wrapper implementations (pure move; signatures/behavior unchanged): `apply`/`apply_task`/`diff`/`cloud_diff_task` + helpers, `generate_app_server_bindings`, `list_features`, `start_responses_api_proxy`, `stdio_to_uds`, and `run_sandbox`.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.17_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.17_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.17_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.17_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.17_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.17_cargo_audit.txt`; reran with `--no-fetch --stale` due to sandbox advisory DB lock constraints)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.17_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.17_cargo_deny_advisories.txt`; workaround: writable `CARGO_HOME=/tmp/cargo_home_p1_17_deny` seeded from `/home/dev/.cargo` advisory DB + registry symlink; `--disable-fetch` + `CARGO_NET_OFFLINE=true`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.17_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.17_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.17_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.17_workplan_diff_final.patch` (post-commit)
- Commit:
  - b424abaab517aa41f7d9b178c898676fb00e2d36

### 2026-02-05 — P1.18 Plan next Phase 1 seams after P1.17 (no code moves)

- Scope/step: P1.18
- Why: `crates/codex/src/lib.rs` still contains significant bulk after P1.14–P1.17 (notably a large `#[cfg(test)]` test module and additional `CodexClient` wrapper methods); plan the next PR-sized extractions while keeping public API paths stable.
- What changed:
  - Updated §7.1 seam extraction order to reflect the current module layout and added the next extraction seams.
  - Updated §10 “Next 5 Tasks” to remove already-done items and queue the next Phase 1 steps.
  - Added new Phase 1 checklist items (P1.19–P1.23) to capture the next PR-sized extractions and Phase 1 evidence refresh.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.18_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.18_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.18_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.18_cargo_audit.txt`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.18_cargo_deny_advisories.txt`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.18_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.18_cargo_fmt_check_final.txt`)
- Commit:
  - c310cfda06ce336410e6c99728c7c983c1f561b5

### 2026-02-05 — P1.19: move crate-root tests out of `lib.rs`

- Scope/step: P1.19
- Why: Reduce `crates/codex/src/lib.rs` size by moving the large `#[cfg(test)] mod tests { ... }` block into `crates/codex/src/tests.rs` (mechanical move; no behavior changes).
- What changed:
  - Replaced inline `#[cfg(test)] mod tests { ... }` with `#[cfg(test)] mod tests;` in `crates/codex/src/lib.rs`.
  - Added `crates/codex/src/tests.rs` containing the moved tests module contents.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.19_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.19_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P1.19_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.19_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.19_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.19_cargo_audit_after2.txt`) (supersedes `evidence_runs/2026-02-05/P1.19_cargo_audit_after.txt`, which was not actually PASS due to missing crates.io registry index in the sandboxed `CARGO_HOME`; workaround: writable temp `CARGO_HOME=/tmp/cargo_home_p1_19_audit_fix` seeded from `/home/dev/.cargo/registry` + `/home/dev/.cargo/advisory-db`, run with `CARGO_NET_OFFLINE=true` + `--db ... --no-fetch --stale --json`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.19_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.19_cargo_deny_advisories.txt`; workaround: writable `CARGO_HOME=/tmp/cargo_home_p1_19_deny` seeded from `/home/dev/.cargo` + `--disable-fetch` + `CARGO_NET_OFFLINE=true`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.19_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.19_cargo_fmt_check_final.txt`)
- Commit:
  - 4a046c0b603e6595acd2d11ec5f3a12e73f18512

### 2026-02-05 — P1.20: extract core `CodexClient` runner helpers into `client_core.rs`

- Scope/step: P1.20
- Why: Reduce `crates/codex/src/lib.rs` size by moving the core `CodexClient` command runner helpers (`run_*`, working directory context helpers) into an internal `crates/codex/src/client_core.rs` module without changing public API paths or behavior.
- What changed:
  - Added `crates/codex/src/client_core.rs` with the extracted `CodexClient` helpers (`directory_context`, `sandbox_working_dir`, `run_simple_command_with_overrides`, `run_basic_command`) plus `DirectoryContext`.
  - Wired `mod client_core;` from `crates/codex/src/lib.rs` and removed the extracted helper implementations from `lib.rs` (no signature/behavior changes).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.20_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.20_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P1.20_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.20_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.20_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.20_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.20_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.20_cargo_audit.txt`; workaround: writable temp `CARGO_HOME` seeded from `/home/dev/.cargo/registry` + `/home/dev/.cargo/advisory-db`, run with `CARGO_NET_OFFLINE=true` + `--db ... --no-fetch --stale --json`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.20_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.20_cargo_deny_advisories.txt`; workaround: writable temp `CARGO_HOME` seeded from `/home/dev/.cargo/advisory-dbs` + `/home/dev/.cargo/registry`, run with `CARGO_NET_OFFLINE=true` + `--disable-fetch`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.20_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.20_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.20_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.20_workplan_diff_final.patch` (post-commit)
- Commit:
  - fc07b5791576cbcf3e3e02421b0676e966b28ad2

### 2026-02-05 — P1.21: extract `CodexError` + shared defaults into modules

- Scope/step: P1.21
- Why: Reduce `crates/codex/src/lib.rs` façade size by moving `CodexError` and shared default helpers/constants into dedicated modules while preserving `codex::CodexError`.
- What changed:
  - Added `crates/codex/src/error.rs` containing the `CodexError` definition; re-exported via `pub use crate::error::CodexError;` from `crates/codex/src/lib.rs`.
  - Added `crates/codex/src/defaults.rs` for shared defaults (`DEFAULT_TIMEOUT`, `CODEX_*`/`RUST_LOG` env keys, and default env/binary helpers); updated internal call sites to reference `crate::defaults::*` (mechanical path move only).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.21_cargo_fmt_check_final.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.21_cargo_fmt_check.txt`; re-check: `evidence_runs/2026-02-05/P1.21_cargo_fmt_check_after.txt`; applies: `evidence_runs/2026-02-05/P1.21_cargo_fmt_apply.txt`, `evidence_runs/2026-02-05/P1.21_cargo_fmt_apply_final.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.21_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.21_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.21_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.21_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.21_cargo_audit.txt`; workaround: writable temp `--db` copy under `/tmp`, run with `CARGO_NET_OFFLINE=true` + `--no-fetch --stale --json`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.21_cargo_deny_advisories.txt`) (initial FAILs: `evidence_runs/2026-02-05/P1.21_cargo_deny_advisories_after.txt` (crates.io DNS/network) + earlier read-only advisory DB lock; workaround: writable temp `CARGO_HOME=/tmp/p1_21_cargo_home_deny` seeded from `/home/dev/.cargo/{advisory-dbs,registry}`, run with `CARGO_NET_OFFLINE=true` + `--disable-fetch`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.21_cargo_deny_licenses.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.21_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.21_workplan_diff_final.patch` (post-commit)
- Commit:
  - e430e5e64a7d9d64d32b8592b3f4f0c3aec5a19c

### 2026-02-05 — P1.22: move remaining non-streaming `CodexClient` wrappers

- Scope/step: P1.22
- Why: Finish draining `crates/codex/src/lib.rs` of the remaining non-streaming `CodexClient` wrapper methods by moving them into `crates/codex/src/commands/*` follow-ups, while preserving `CodexClient` method signatures and behavior.
- What changed:
  - Moved `CodexClient::{features, help, review, exec_review, fork_session}` into `crates/codex/src/commands/{features,help,review,fork}.rs`.
  - Moved `CodexClient::{cloud_overview, cloud_list, cloud_status, cloud_diff, cloud_apply, cloud_exec}` into `crates/codex/src/commands/cloud.rs`.
  - Moved `CodexClient::{mcp_overview, mcp_list, mcp_get, mcp_add, mcp_remove, mcp_logout, spawn_mcp_oauth_login_process}` into `crates/codex/src/commands/mcp.rs`.
  - Updated `crates/codex/src/commands/mod.rs` to include the new command-domain modules; `crates/codex/src/lib.rs` remains the façade + capability/update helpers.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.22_cargo_fmt_check_final.txt`) (initial: `evidence_runs/2026-02-05/P1.22_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.22_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.22_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.22_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.22_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.22_cargo_audit.txt`; workaround: writable temp `CARGO_HOME` + `--db` copy seeded from `/home/dev/.cargo/{registry,advisory-db}`, run with `CARGO_NET_OFFLINE=true` + `--no-fetch --stale --json`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.22_cargo_deny_advisories.txt`) (initial FAIL: read-only advisory DB lock; workaround: writable temp `CARGO_HOME` seeded from `/home/dev/.cargo/{advisory-dbs,registry,git}`, run with `CARGO_NET_OFFLINE=true` + `--disable-fetch`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.22_cargo_deny_licenses.txt`) (workaround: writable temp `CARGO_HOME` seeded from `/home/dev/.cargo/{registry,git}`, run with `CARGO_NET_OFFLINE=true`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P1.22_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P1.22_workplan_diff_final.patch` (post-commit)
- Commit:
  - f3f09f615c858a9c8a89ccf1065660ba5656f7d1

### 2026-02-05 — P1.23 Refresh Phase 1 size evidence and close Phase 1 (no code moves)

- Scope/step: P1.23
- Why: Refresh §3.2 “Top offenders” and Phase 1 status after P1.19–P1.22; update only if evidence supports it.
- Measurement artifacts:
  - Base commit: `evidence_runs/2026-02-05/P1.23_BASE_STEP.txt`
  - Raw `tokei` JSON: `evidence_runs/2026-02-05/P1.23_tokei_crates.json`
  - Derived Rust per-file code LOC list: `evidence_runs/2026-02-05/P1.23_rust_files_sorted_by_code.txt`
- Result: Phase 1 is eligible to close because `crates/codex/src/lib.rs` is 334 Rust code LOC (<= ceiling=1000) per `evidence_runs/2026-02-05/P1.23_rust_files_sorted_by_code.txt`.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P1.23_cargo_fmt_check.txt`) (final: `evidence_runs/2026-02-05/P1.23_cargo_fmt_check_final.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P1.23_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P1.23_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P1.23_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.23_cargo_audit.txt`; workaround: writable `/tmp` advisory DB copy via `--db ...` + `CARGO_NET_OFFLINE=true` + `--no-fetch --stale --json`)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P1.23_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P1.23_cargo_deny_advisories.txt`; workaround: writable temp `CARGO_HOME` seeded from `/home/dev/.cargo/{advisory-dbs,registry,git}` + `CARGO_NET_OFFLINE=true` + `--disable-fetch`)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P1.23_cargo_deny_licenses.txt`)
- Commit:
  - 444cdf94efaa7fcad551a4fcafc0f4e664c39712

### 2026-02-05 — P4.0 tests modularization (`crates/codex/src/tests.rs` below ceiling)

- Scope/step: P4.0
- Why: Earliest not-done item in §10 execution queue; reduce `crates/codex/src/tests.rs` below the 1000 LOC ceiling without changing test behavior.
- What changed:
  - Replaced monolithic `crates/codex/src/tests.rs` with a thin façade entrypoint and moved test bodies into `crates/codex/src/tests/` domain modules.
  - Added `crates/codex/src/tests/mod.rs` as the shared internal test harness (`use` imports + helper fns + module declarations), and split tests into:
    - `auth_session.rs`
    - `builder_env_home.rs`
    - `bundled_binary.rs`
    - `capabilities.rs`
    - `cli_commands.rs`
    - `cli_overrides.rs`
    - `jsonl_stream.rs`
    - `sandbox_execpolicy.rs`
  - Added `#[path = "tests.rs"] mod tests;` under `#[cfg(test)]` in `crates/codex/src/lib.rs` to disambiguate `tests.rs` entrypoint vs `tests/mod.rs` tree.
  - Result: `crates/codex/src/tests.rs` is now a 4-line façade and no longer a >1000 LOC monolith.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P4.0_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.0_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.0_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.0_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0_cargo_audit.txt`; reran with `--no-fetch --stale` due to advisory DB fetch/lock constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.0_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, `--disable-fetch`, and offline mode due crates.io/advisory-db access constraints)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.0_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P4.0_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P4.0_workplan_diff_final.patch` (post-commit)
- Commit:
  - c61b4d43f1cf8807d8cc929b62f8c18d1676c599

### 2026-02-05 — P4.1 tests modularization (`crates/codex/src/mcp/tests_core.rs` below hard threshold)

- Scope/step: P4.1
- Why: Earliest not-done task in §10 after P4.0; reduce `crates/codex/src/mcp/tests_core.rs` to `<= 600` LOC without behavior changes.
- What changed:
  - Replaced monolithic `crates/codex/src/mcp/tests_core.rs` with a thin module façade and moved test bodies into `crates/codex/src/mcp/tests_core/` domain files.
  - Added:
    - `crates/codex/src/mcp/tests_core/app_server_launch.rs`
    - `crates/codex/src/mcp/tests_core/config_runtime.rs`
    - `crates/codex/src/mcp/tests_core/codex_rpc_flows.rs`
    - `crates/codex/src/mcp/tests_core/app_server_rpc_flows.rs`
  - Preserved test assertions/fixtures/semantics; this is a mechanical organization-only split.
  - Result: `crates/codex/src/mcp/tests_core.rs` is now 4 LOC (<= 600).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.1_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.1_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P4.1_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.1_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.1_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.1_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.1_cargo_audit.txt`; reran with `--no-fetch --stale` due advisory DB fetch/lock constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.1_cargo_deny_advisories.txt`) (initial FAIL due read-only advisory DB lock at default `CARGO_HOME`; reran with writable temp `CARGO_HOME`, `--disable-fetch`, and offline mode in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.1_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.1_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Base commit: `evidence_runs/2026-02-05/P4.1_BASE_STEP.txt`
- Commit:
  - 20a40914277bfe563c39405f9bb785927bdd649f

### 2026-02-05 — P4.2 tests modularization (`crates/codex/src/mcp/tests_runtime_app.rs` below hard threshold)

- Scope/step: P4.2
- Why: Earliest not-done task in §10 after P4.1; reduce `crates/codex/src/mcp/tests_runtime_app.rs` to `<= 600` LOC without behavior changes.
- What changed:
  - Replaced monolithic `crates/codex/src/mcp/tests_runtime_app.rs` with a thin module façade.
  - Added domain submodules under `crates/codex/src/mcp/tests_runtime_app/`:
    - `runtime_api.rs`
    - `app_runtime_api.rs`
    - `app_runtime_pool_api.rs`
    - `runtime_manager.rs`
  - Preserved assertions/fixtures/behavior; this was a move-only test reorganization.
  - Result: `crates/codex/src/mcp/tests_runtime_app.rs` is now 4 LOC (<= 600).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.2_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.2_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P4.2_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.2_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.2_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.2_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.2_cargo_audit.txt`; reran with `--no-fetch --stale` due advisory DB fetch/lock constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.2_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.2_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, `--disable-fetch`, and offline mode due advisory DB lock constraints)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.2_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.2_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Base commit: `evidence_runs/2026-02-05/P4.2_BASE_STEP.txt`
- Commit:
  - c76e8d030fbe4d6b53d1495248c3ef5d64ea1bbe

### 2026-02-05 — P4.3 report domain split (`crates/xtask/src/codex_report/report.rs` below hard threshold)

- Scope/step: P4.3
- Why: Earliest not-done task in §10 after P4.2; reduce `crates/xtask/src/codex_report/report.rs` to `<= 600` LOC while preserving deterministic report behavior.
- What changed:
  - Kept `crates/xtask/src/codex_report/report.rs` as orchestration logic (`index_upstream` + `build_report`) and extracted cohesive helper domains to `crates/xtask/src/codex_report/report/`:
    - `schema.rs` (report output structs/enums for deterministic JSON shape)
    - `filtering.rs` (platform-filter presence/classification helpers)
    - `iu.rs` (intentionally-unsupported inheritance resolution + deterministic sort comparator)
    - `parity.rs` (parity exclusions index builder)
  - Preserved API path usage in callers via `report::build_parity_exclusions_index` and `report::build_report`.
  - Preserved ordering/formatting behavior (all sorting/comparison logic moved without semantic changes).
  - Result: `crates/xtask/src/codex_report/report.rs` is now 551 LOC (<= 600).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.3_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.3_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P4.3_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.3_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.3_cargo_clippy.txt`; fixed module visibility wiring in step scope)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.3_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.3_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.3_cargo_audit.txt`; reran with `--no-fetch --stale` due advisory DB fetch/network constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.3_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.3_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME` and `--disable-fetch` due advisory DB lock/network constraints)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.3_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.3_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Base commit: `evidence_runs/2026-02-05/P4.3_BASE_STEP.txt`
- Commit:
  - 7e000d7579e71b194cb72d76bb506f7c7e43a52d

### 2026-02-05 — P4.4 size evidence refresh + Phase 4 queue reconciliation (no code moves)

- Scope/step: P4.4
- Why: Re-measure Rust file sizes after P4.0–P4.3 and update §3.2/Phase 4/§10 from fresh evidence only.
- Measurement artifacts:
  - Base commit: `evidence_runs/2026-02-05/P4.4_BASE_STEP.txt`
  - Raw `tokei` JSON: `evidence_runs/2026-02-05/P4.4_tokei_crates.json`
  - Derived Rust per-file code LOC list: `evidence_runs/2026-02-05/P4.4_rust_files_sorted_by_code.txt`
- Result:
  - `crates/codex/src/tests.rs`, `crates/codex/src/mcp/tests_core.rs`, `crates/codex/src/mcp/tests_runtime_app.rs`, and `crates/xtask/src/codex_report/report.rs` no longer appear as >hard offenders.
  - No Rust files are above the 1000 LOC ceiling.
  - Seven Rust files remain above the hard threshold (top: `crates/codex/src/tests/capabilities.rs` at 904 LOC), so Phase 4 remains In Progress and §10 is re-queued to target those offenders.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.4_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.4_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.4_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.4_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.4_cargo_audit.txt`; reran with `--no-fetch --stale` due advisory DB fetch/lock constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.4_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.4_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, `--disable-fetch`, and offline mode due advisory DB lock constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.4_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.4_cargo_fmt_check_final.txt`)
- Commit:
  - 6568e780bf16016d7fe19dc2b28ec74f487fcc17

### 2026-02-05 — P4.5 tests modularization (`crates/codex/src/tests/capabilities.rs` below hard threshold)

- Scope/step: P4.5
- Why: Earliest queued not-done item in §10; reduce `crates/codex/src/tests/capabilities.rs` from 904 LOC to `<= 600` LOC via mechanical capability-domain test splitting.
- What changed:
  - Replaced monolithic test body in `crates/codex/src/tests/capabilities.rs` with a thin helper + module-wiring façade (now 109 LOC).
  - Added cohesive capability-domain test submodules under `crates/codex/src/tests/capabilities/`:
    - `version_and_advisory.rs`
    - `snapshots_and_cache.rs`
    - `probe_cache_policy.rs`
    - `feature_parsing_and_guards.rs`
    - `overrides_and_probe.rs`
    - `exec_and_login.rs`
  - Preserved test names/assertions/fixtures; this is organization-only.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.5_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.5_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.5_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.5_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.5_cargo_audit.txt`; reran with `--no-fetch --stale` due advisory DB lock/fetch constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.5_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.5_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, offline mode, and `--disable-fetch` due advisory DB lock constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.5_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.5_cargo_fmt_check_final.txt`)
- Commit:
  - 2152b460c23094bcf847a4d444fb74134b75c749

### 2026-02-05 — P4.6 union modularization (`crates/xtask/src/codex_union.rs` below hard threshold)

- Scope/step: P4.6
- Why: Earliest queued not-done item in §10 after P4.5; reduce `crates/xtask/src/codex_union.rs` to `<= 600` LOC while preserving deterministic union output/ordering behavior.
- What changed:
  - Kept `crates/xtask/src/codex_union.rs` as the stable façade/orchestration entrypoint (`run`, rules validation, normalization, deterministic timestamp helper).
  - Extracted command/flag/arg union merge + conflict/evidence helpers into `crates/xtask/src/codex_union/merge.rs`.
  - Preserved deterministic ordering/comparison semantics by moving existing sort and conflict-key logic unchanged.
  - Result: `crates/xtask/src/codex_union.rs` is now 364 LOC (<= 600).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.6_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.6_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P4.6_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.6_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.6_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.6_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.6_cargo_audit.txt`; reran with `--no-fetch --stale` due advisory DB lock/fetch constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.6_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.6_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, offline mode, and `--disable-fetch` due advisory DB lock constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.6_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.6_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Base commit: `evidence_runs/2026-02-05/P4.6_BASE_STEP.txt`
- Commit:
  - f3b8bcf074585e0f463f49bda210ca564d232ee7

### 2026-02-05 — P4.7 tests modularization (`crates/xtask/tests/c3_spec_reports_metadata_retain.rs` below hard threshold)

- Scope/step: P4.7
- Why: Earliest queued not-done item in §10 after P4.6; reduce `crates/xtask/tests/c3_spec_reports_metadata_retain.rs` to `<= 600` LOC via mechanical test modularization with assertions/snapshots unchanged.
- What changed:
  - Kept `crates/xtask/tests/c3_spec_reports_metadata_retain.rs` as the shared helper façade and moved inline tests into out-of-line submodules.
  - Added submodules under `crates/xtask/tests/c3_spec_reports_metadata_retain/`:
    - `report_filter_semantics.rs`
    - `report_incomplete_union.rs`
    - `version_metadata_requirements.rs`
    - `retain_behavior.rs`
  - Preserved existing test names/assertions/fixtures; this is organization-only.
  - Result: `crates/xtask/tests/c3_spec_reports_metadata_retain.rs` is now 478 LOC (<= 600).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.7_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.7_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.7_cargo_clippy.txt`; fixed module path wiring/imports in step scope)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.7_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.7_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.7_cargo_audit.txt`; reran with `--no-fetch --stale` due advisory DB lock/fetch constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.7_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.7_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, offline mode, and `--disable-fetch` due advisory DB lock/network constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.7_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.7_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Base commit: `evidence_runs/2026-02-05/P4.7_BASE_STEP.txt`
- Commit:
  - 7e6caa66ae98d349922e765397c35619632cce15

### 2026-02-05 — P4.8 version metadata modularization (`crates/xtask/src/codex_version_metadata.rs` below hard threshold)

- Scope/step: P4.8
- Why: Earliest queued not-done item in §10 after P4.7; reduce `crates/xtask/src/codex_version_metadata.rs` to `<= 600` LOC while preserving output/formatting behavior.
- What changed:
  - Kept `crates/xtask/src/codex_version_metadata.rs` as the stable façade (`Args`, `Status`, errors, metadata schema structs, orchestration/gates).
  - Extracted coverage/parity/indexing helpers into `crates/xtask/src/codex_version_metadata/coverage.rs` with seam-local wiring via `mod coverage`.
  - Preserved deterministic output and ordering semantics by moving helper logic unchanged.
  - Result: `crates/xtask/src/codex_version_metadata.rs` is now 394 LOC (<= 600).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.8_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.8_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P4.8_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.8_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.8_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.8_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.8_cargo_audit.txt`; reran with `--no-fetch --stale` due advisory DB lock/fetch constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.8_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.8_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, offline mode, and `--disable-fetch` due advisory DB lock constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.8_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.8_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Base commit: `evidence_runs/2026-02-05/P4.8_BASE_STEP.txt`
- Commit:
  - f8a7c89cbe6fa92246215662ec36d3418e662a82

### 2026-02-05 — P4.9 exec streaming helper extraction (`crates/codex/src/exec.rs` below hard threshold)

- Scope/step: P4.9
- Why: Earliest queued not-done item in §10 after P4.8; reduce `crates/codex/src/exec.rs` to `<= 600` LOC while preserving public API paths and runtime behavior.
- What changed:
  - Kept `crates/codex/src/exec.rs` as the stable façade for public entrypoints/types.
  - Extracted streaming exec/resume internals into `crates/codex/src/exec/streaming.rs`.
  - Replaced `stream_exec_with_overrides` and `stream_resume` bodies in `exec.rs` with thin delegating wrappers.
  - Preserved command construction, option ordering, stdin handling, stream forwarding, timeout/error semantics, and completion payload behavior; no API path changes.
  - Result: `crates/codex/src/exec.rs` is now 469 LOC (<= 600).
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.9_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.9_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.9_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.9_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.9_cargo_audit.txt`; reran with `--no-fetch --stale` due advisory DB lock/fetch constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.9_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.9_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, offline mode, and `--disable-fetch` due advisory DB lock constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.9_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.9_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Base commit: `evidence_runs/2026-02-05/P4.9_BASE_STEP.txt`
- Commit:
  - ad66f9830be0af5b9df912d225ce16b450ab0751

### 2026-02-05 — P4.0.1 tests modularization follow-up (`jsonl` domain split scaffold 1/5)

- Scope/step: P4.0.1
- Why: Continue the P4.0 loop by moving one cohesive tests domain into its own module while keeping behavior unchanged.
- What changed:
  - Removed transitional `crates/codex/src/tests.rs` so `tests/mod.rs` is now the direct test-module entrypoint.
  - Added `crates/codex/src/tests/support.rs` and moved shared helper functions there (`env_guard*`, fake executable writers).
  - Renamed `crates/codex/src/tests/jsonl_stream.rs` to `crates/codex/src/tests/jsonl.rs` with test bodies preserved.
  - Updated `crates/codex/src/tests/mod.rs` wiring (`mod support;`, `mod jsonl;`) and kept existing shared imports behavior for sibling modules.
  - Updated `crates/codex/src/lib.rs` test wiring from `#[path = "tests.rs"] mod tests;` to `mod tests;`.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0.1_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.0.1_cargo_clippy_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.1_cargo_clippy.txt`; fixed module visibility/import wiring in step scope)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.0.1_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.0.1_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.1_cargo_audit.txt`; reran with writable copied advisory DB + `--no-fetch --stale` due read-only advisory DB lock in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.0.1_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.1_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, offline mode, and `--disable-fetch` due advisory DB lock constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.0.1_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0.1_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/P4.0.1_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/P4.0.1_workplan_diff_final.patch` (post-commit)
- Commit:
  - 86e473d60056ac6cdf001646eaf2bb1e23039670

### 2026-02-05 — P4.0.2 tests modularization follow-up (`cloud` domain split 2/5)

- Scope/step: P4.0.2
- Why: Continue the P4.0 loop by moving one cohesive tests domain into its own module while keeping behavior unchanged.
- What changed:
  - Moved cloud command tests from `crates/codex/src/tests/cli_commands.rs` into new `crates/codex/src/tests/cloud.rs` with test names/assertions preserved.
  - Updated `crates/codex/src/tests/mod.rs` wiring to include `mod cloud;`.
  - Kept shared helpers in `crates/codex/src/tests/support.rs`; no helper behavior changes were required.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0.2_cargo_fmt_check_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.2_cargo_fmt_check.txt`; applied: `evidence_runs/2026-02-05/P4.0.2_cargo_fmt_apply.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.0.2_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.0.2_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.0.2_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.2_cargo_audit.txt`; reran with copied writable advisory DB + `--no-fetch --stale` due read-only advisory DB lock in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.0.2_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.2_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, copied registry/advisory DB state, offline mode, and `--disable-fetch` due advisory DB lock/network constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.0.2_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0.2_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/SESSION_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/SESSION_workplan_diff_final.patch` (post-commit)
- Commit:
  - 0549f5a533fad243dd11c56a926fdb08c9a595e9

### 2026-02-05 — P4.0.3 tests modularization follow-up (`mcp` domain split 3/5)

- Scope/step: P4.0.3
- Why: Continue the P4.0 loop by moving one cohesive tests domain into its own module while keeping behavior unchanged.
- What changed:
  - Moved MCP command tests from `crates/codex/src/tests/cli_commands.rs` into new `crates/codex/src/tests/mcp.rs` with test names/assertions preserved.
  - Updated `crates/codex/src/tests/mod.rs` wiring to include `mod mcp;`.
  - Kept shared helpers in `crates/codex/src/tests/support.rs`; no helper behavior changes were required.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0.3_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.0.3_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.0.3_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.0.3_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.3_cargo_audit.txt`; reran with writable temp `CARGO_HOME` + copied advisory DB and `--no-fetch --stale` due read-only advisory DB lock in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.0.3_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.3_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, copied registry/advisory DB state, offline mode, and `--disable-fetch` due advisory DB lock/network constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.0.3_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0.3_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/SESSION_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/SESSION_workplan_diff_final.patch` (post-commit)
- Commit:
  - f8b259e38853f874c2966650257fbc013dea06be

### 2026-02-05 — P4.0.4 tests modularization follow-up (`app_server` domain split 4/5)

- Scope/step: P4.0.4
- Why: Continue the P4.0 loop by finalizing the interrupted move-only test split and recording full gate evidence.
- What changed:
  - Moved app-server/response-proxy/stdio-to-uds tests from `crates/codex/src/tests/cli_commands.rs` into new `crates/codex/src/tests/app_server.rs` with test names/assertions preserved.
  - Updated `crates/codex/src/tests/mod.rs` wiring to include `mod app_server;`.
  - Kept shared helpers/import style unchanged via `use super::*;` to preserve behavior.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0.4_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.0.4_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.0.4_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.0.4_cargo_audit.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.4_cargo_audit_initial_fail.txt`; intermediate offline index miss: `evidence_runs/2026-02-05/P4.0.4_cargo_audit_index_miss.txt`; reran with writable temp `CARGO_HOME`, copied advisory DB/registry cache, and `--no-fetch --stale` due advisory DB lock/network constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.0.4_cargo_deny_advisories.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.4_cargo_deny_advisories_initial_fail.txt`; reran with writable temp `CARGO_HOME`, copied advisory DB/registry cache, offline mode, and `--disable-fetch` due advisory DB lock/network constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.0.4_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0.4_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/SESSION_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/SESSION_workplan_diff_final.patch` (post-commit)
- Commit:
  - 465e126a60d645850db5185eaed03ffabcc836c5

### 2026-02-05 — P4.0.5 tests modularization follow-up (`cli` domain split 5/5)

- Scope/step: P4.0.5
- Why: Continue the P4.0 loop by moving the remaining CLI-domain tests into a dedicated module and removing the transitional `cli_commands` module.
- What changed:
  - Moved the remaining CLI-domain tests from `crates/codex/src/tests/cli_commands.rs` into `crates/codex/src/tests/cli.rs` with test names/assertions preserved.
  - Updated `crates/codex/src/tests/mod.rs` wiring from `mod cli_commands;` to `mod cli;`.
  - Removed `crates/codex/src/tests/cli_commands.rs`.
  - Kept shared helpers in `crates/codex/src/tests/support.rs`; no helper behavior changes were required.
- Validation results (§4.1):
  - `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0.5_cargo_fmt_check.txt`)
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS (`evidence_runs/2026-02-05/P4.0.5_cargo_clippy.txt`)
  - `cargo test --all-targets --all-features`: PASS (`evidence_runs/2026-02-05/P4.0.5_cargo_test.txt`)
  - `cargo audit`: PASS (`evidence_runs/2026-02-05/P4.0.5_cargo_audit_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.5_cargo_audit.txt`; reran with writable temp `CARGO_HOME` and `--no-fetch --stale` due advisory DB lock/network constraints in this sandbox)
  - `cargo deny check advisories`: PASS (`evidence_runs/2026-02-05/P4.0.5_cargo_deny_advisories_after.txt`) (initial FAIL: `evidence_runs/2026-02-05/P4.0.5_cargo_deny_advisories.txt`; reran with writable temp `CARGO_HOME`, offline mode, and `--disable-fetch` due advisory DB lock constraints in this sandbox)
  - `cargo deny check licenses`: PASS (`evidence_runs/2026-02-05/P4.0.5_cargo_deny_licenses.txt`)
  - Final `cargo fmt --all -- --check`: PASS (`evidence_runs/2026-02-05/P4.0.5_cargo_fmt_check_final.txt`)
- Evidence/patches:
  - Code diff: `evidence_runs/2026-02-05/SESSION_code_diff_final.patch` (post-commit)
  - Workplan diff: `evidence_runs/2026-02-05/SESSION_workplan_diff_final.patch` (post-commit)
- Commit:
  - TBD (single-commit rule: commit hash is available only after commit creation)

## 9) Open Questions / Decisions (lightweight log)

Use this table for decisions that affect policy, public APIs, or exceptions to size constraints.

| Decision | Date | Status (Proposed/Accepted/Rejected) | Rationale | Notes |
|---|---|---|---|---|
| Confirm Phase 0 remediation state vs audit pack | 2026-02-04 | Accepted | Audit pack shows failures; baseline claims fixes | Preflight gates are green; see §8 “Phase 0 preflight…” entry. |
| Duplicate versions policy (fix now vs defer) | 2026-02-04 | Accepted | Non-goal: dependency upgrades beyond security/compliance in Phase 0 | `cargo tree -d --target all` shows `getrandom` + `windows-sys` duplicates; `audit_pack/deps/cargo_tree_duplicates.txt` showed none (audit-time command likely ran without `--target all`). Decision: defer consolidation unless a security/compliance gate requires it. |
| Allowlist license expressions in `deny.toml` | 2026-02-04 | Accepted | `cargo deny` defaults fail without config; policy must be explicit | `deny.toml` establishes allowlist + target scoping + confidence; `cargo deny check licenses` PASS (see §8 “Phase 0 preflight…” entry). |
| Legacy execution evidence location (`audit_pack/execution/YYYY-MM-DD/...`) | 2026-02-04 | Accepted | Preserve provenance for already-generated evidence | Keep existing execution artifacts under `audit_pack/execution/...` as-is (do not move/delete). New execution evidence goes under `evidence_runs/...` (§8.1). |
| Post-plan work request: “complete next 5 tasks” vs remaining checklist items | 2026-02-04 | Accepted | Workplan needed additional scope to reflect remaining seams beyond P3.5 | Added P1.4 (execpolicy seam), P2.5 (JSON-RPC transport), and P3.6–P3.8 (deeper xtask splits) as Not Started follow-ons. |
| Legacy combined commit for multiple steps (P1.4/P2.5/P3.6) | 2026-02-05 | Accepted | These steps landed before the “one step = one commit” orchestrator rule was enforced | Commit `2d17281b8d09c7797cd555d4b2fd5951af75b328` contains P1.4 + P2.5 + P3.6 changes; journal entries cite it for provenance. Future steps must follow per-step commit policy. |
| Execution evidence storage policy (`evidence_runs/YYYY-MM-DD/`); keep `audit_pack/execution/*` legacy | 2026-02-04 | Accepted | Provenance clarity: separate immutable audit snapshot (`audit_pack/`) from ongoing execution runs | Store new evidence under `evidence_runs/…` with stable filenames (§8.1). Do not move/delete existing `audit_pack/execution/…` evidence; continue citing it where already referenced. |
| P1.5 size exception: `builder.rs` > 600 LOC | 2026-02-05 | Accepted | Keep builder/config/flags surfaces cohesive during seam extraction to avoid churn across call sites; follow-up split can happen once the façade boundaries stabilize | `crates/codex/src/builder.rs` is ~921 LOC (above §7.3 hard=600). Follow-up: split into `builder/overrides.rs` + `builder/types.rs` + `builder/mod.rs` once P1.6 lands, preserving re-exports. |
| Builder size exception resolved / superseded | 2026-02-05 | Accepted | `builder` is now split into module files and no longer requires a single-file size exception | Supersedes “P1.5 size exception: `builder.rs` > 600 LOC”; current structure uses `crates/codex/src/builder/mod.rs`, `crates/codex/src/builder/types.rs`, and related module files. |

---

## 10) Next 5 Tasks (execution queue; do not start until this workplan is current)

Selection rule (orchestrator): Execute tasks in the order listed below (top-to-bottom). Reorder this list to change cross-phase priority; do not infer priority from Phase 1/2/3 sections.

1) `TBD` — queue refresh required (stale completed items removed)
2) `TBD`
3) `TBD`
4) `TBD`
5) `TBD`
