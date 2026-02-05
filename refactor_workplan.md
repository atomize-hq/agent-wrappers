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
  - As of 2026-02-05, the latest post-refactor `tokei`/duplicates artifacts are stored under `audit_pack/execution/2026-02-04/` (see list below); until they are refreshed into `evidence_runs/`, treat those `audit_pack/execution/...` paths as canonical for §3.1/§3.2/§6.3.

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

Post-refactor top 10 from `audit_pack/execution/2026-02-04/post_refactor_tokei_files_sorted_updated.txt` (Rust per-file code LOC):
- `crates/codex/src/lib.rs` — 8,677 LOC (>> ceiling)
- `crates/codex/src/mcp.rs` — 2,191 LOC (>> ceiling)
- `crates/xtask/src/codex_validate.rs` — 1,148 LOC (>> ceiling)
- `crates/xtask/src/codex_report/report.rs` — 922 LOC (> hard)
- `crates/xtask/src/codex_union.rs` — 799 LOC (> hard)
- `crates/xtask/tests/c3_spec_reports_metadata_retain.rs` — 742 LOC (> hard)
- `crates/xtask/src/codex_version_metadata.rs` — 721 LOC (> hard)
- `crates/codex/tests/cli_e2e.rs` — 669 LOC (> hard)
- `crates/xtask/src/codex_snapshot/discovery.rs` — 607 LOC (> hard)
- `crates/codex/src/mcp/config.rs` — 538 LOC (> soft)

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

Phase Status: [ ] Not Started  [x] In Progress  [ ] Done  
Last Updated: 2026-02-05  
Reason: `crates/codex/src/lib.rs` remains above the program ceiling per §3.2 (evidence: `audit_pack/execution/2026-02-04/post_refactor_tokei_files_sorted_updated.txt`).

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

---

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

Notes / dependencies:
- `capabilities.rs` is used by the builder/client for cache policies, overrides, and probes; extract before apply/diff to reduce cross-cutting churn.
- `apply_diff.rs` depends on command execution plumbing but should remain API-stable via re-exports from `lib.rs`.

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
  - TBD_POST_COMMIT

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
  - TBD_POST_COMMIT

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
  - TBD_POST_COMMIT

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

---

## 10) Next 5 Tasks (execution queue; do not start until this workplan is current)

Selection rule (orchestrator): Execute tasks in the order listed below (top-to-bottom). Reorder this list to change cross-phase priority; do not infer priority from Phase 1/2/3 sections.

1) (TBD) Next Phase 1 seam extraction after P1.6
2) (TBD) Next Phase 2 follow-on after P2.7
3) (TBD) Next Phase 3 follow-on after P3.9
4) (TBD) Next cross-phase follow-on
5) (TBD) Next execution item
