# Refactor Workplan / Status (Master)

Last Updated: 2026-02-04  
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
6) Evidence must be cited via exact `audit_pack/` file paths (do not hand-wave).

### 1.3 Non-goals (unless explicitly added later)

- No dependency upgrades beyond what is required for security/compliance in Phase 0.
- No behavior changes, no UX changes, no “cleanup” refactors unrelated to the seam being extracted.
- No CI redesign (we only use what is in the audit pack as baseline; CI specifics remain out of scope).

### 1.4 Audit Baseline Summary (verbatim program baseline)

- Workspace: crates/codex + crates/xtask
- Critical: bytes 1.11.0 vuln RUSTSEC-2026-0007 → fix >= 1.11.1
- Critical: cargo-deny license check fails without explicit deny.toml policy
- Medium: duplicate crate versions getrandom/windows-sys
- High: “god modules” above thresholds; p75=302 p90=746 soft=302 hard=746
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

---

## 3) Baseline Metrics (and current failure signals)

### 3.1 Maintainability thresholds (from audit)

From `audit_pack/metrics/loc_summary.txt`:
- Rust files: 70
- Total Rust LOC: 31,217
- Median LOC: 121
- p75 LOC: 302  → **soft threshold = 302**
- p90 LOC: 746  → **hard threshold = 746**
- **absolute ceiling (program policy) = 1000**

### 3.2 Top offenders (largest Rust files)

From `audit_pack/metrics/loc_summary.txt` (Top 20):
- `crates/codex/src/lib.rs` — 9,850 LOC (>> ceiling)
- `crates/codex/src/mcp.rs` — 4,278 LOC (>> ceiling)
- `crates/xtask/src/codex_validate.rs` — 2,543 LOC (>> ceiling)
- `crates/xtask/src/codex_report.rs` — 1,543 LOC (>> ceiling)
- `crates/xtask/src/codex_snapshot.rs` — 1,303 LOC (>> ceiling)
- `crates/xtask/src/codex_union.rs` — 884 LOC (> hard)
- `crates/xtask/src/codex_version_metadata.rs` — 785 LOC (> hard)
- `crates/xtask/tests/c3_spec_reports_metadata_retain.rs` — 742 LOC (~ hard boundary)
- `crates/codex/tests/cli_e2e.rs` — 669 LOC (> soft)
- `crates/codex/src/wrapper_coverage_manifest.rs` — 511 LOC (> soft)

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

*(Repeat P1.x as needed until `lib.rs` is within the program ceiling or is a thin façade with the bulk in modules.)*

---

#### Phase 2 — Split `crates/codex/src/mcp.rs` into `crates/codex/src/mcp/*` (API stable)

**Phase goal:** Split `mcp.rs` into a module tree while maintaining stable public APIs via re-exports from `mcp` (and/or `lib.rs`).

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

---

### Workstream C — `crates/xtask` Maintainability (determinism preserved)

#### Phase 3 — Split xtask “rule engine” files by domain sections; keep determinism

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

Status: [ ] Not Started  [ ] In Progress  [ ] Done  
Last Updated: YYYY-MM-DD

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

Evidence source: `audit_pack/deps/cargo_tree_duplicates.txt` (audit time).

| Crate | Duplicate versions? | Evidence | Decision (fix/defer) | Rationale | Status |
|---|---:|---|---|---|---|
| getrandom | Yes (`0.2.17`, `0.3.4`) | Audit-time: `audit_pack/deps/cargo_tree_duplicates.txt` (no duplicates printed). Current (2026-02-04): `cargo tree -d --target all` shows duplicates via `jsonschema`/`ahash` and `tempfile`. | Defer | Resolving likely requires upgrading transitive crates (non-goal in Phase 0 unless required for security/compliance); keep note and revisit if it becomes a policy requirement. | Done |
| windows-sys | Yes (`0.60.2`, `0.61.2`) | Audit-time: `audit_pack/deps/cargo_tree_duplicates.txt` (no duplicates printed). Current (2026-02-04): `cargo tree -d --target all` shows duplicates via `tokio`/`reqwest`/`socket2` and CLI deps. | Defer | Same rationale as above; no gate currently failing and no security advisory forcing consolidation. | Done |

---

## 7) Modularization Strategy (Boundaries, API stability, size policy)

### 7.1 `crates/codex/src/lib.rs` seam extraction order (Phase 1)

**Rule:** `lib.rs` remains the façade; extracted modules keep stable public API via `pub use` re-exports from `lib.rs`. Do not change public item paths without an explicit migration plan (§9 decision log).

Seam order (defined in P1.0; extract in PR-sized steps):
1) Home/env plumbing → `home.rs` (P1.1; already present in working tree).
2) Capability probing + caching → `capabilities.rs` (P1.2).
3) Apply/diff request + artifacts → `apply_diff.rs` (P1.3).
4) Execpolicy modeling + parsing → `execpolicy.rs` (planned; define when scheduled).
5) Builder/config/flags surfaces → one or more cohesive modules (planned; define when scheduled).

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
5) **Stdio JSON-RPC transport** → `crates/codex/src/mcp/jsonrpc.rs` (scheduled after the above)
   - `codex mcp-server` / `codex app-server` spawn + request/response plumbing + notification fan-out
   - Keep the higher-level clients (`CodexMcpServer`, `AppServer`) in `mcp.rs` until transport move is complete.

Notes / dependencies:
- Phase 2 exists because `crates/codex/src/mcp.rs` is a “god module” at audit time (4,278 LOC). Evidence: `audit_pack/metrics/loc_summary.txt`.
- Extract in the order above to avoid cycles: protocol is leaf-ish; config is depended on by runtime/app; runtime/app depend on protocol; JSON-RPC transport is used by the high-level clients.

### 7.3 File size policy (applies during refactor)

Derived from `audit_pack/metrics/loc_summary.txt`:
- Soft threshold: 302 LOC (p75)
- Hard threshold: 746 LOC (p90)
- Absolute ceiling: 1000 LOC

Policy:
- New files should target **≤ 302 LOC**.
- New files may exceed soft threshold if cohesive, but should stay **≤ 746 LOC**.
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

---

## 8) Execution Journal (append-only)

Add entries as work lands. Format:

### YYYY-MM-DD — <short title>

- Scope/step: P?.? (<phase step id>)
- Why: <reason / link to evidence>
- What changed:
  - <bullet list of changes; keep concise>
- Validation results (paste concise results, not full logs):
  - `cargo fmt --all -- --check`: PASS/FAIL
  - `cargo clippy --all-targets --all-features -- -D warnings`: PASS/FAIL
  - `cargo test --all-targets --all-features`: PASS/FAIL
  - `cargo audit`: PASS/FAIL (list advisories if FAIL)
  - `cargo deny check advisories`: PASS/FAIL
  - `cargo deny check licenses`: PASS/FAIL
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

---

## 9) Open Questions / Decisions (lightweight log)

Use this table for decisions that affect policy, public APIs, or exceptions to size constraints.

| Decision | Date | Status (Proposed/Accepted/Rejected) | Rationale | Notes |
|---|---|---|---|---|
| Confirm Phase 0 remediation state vs audit pack | 2026-02-04 | Accepted | Audit pack shows failures; baseline claims fixes | Preflight gates are green; see §8 “Phase 0 preflight…” entry. |
| Duplicate versions policy (fix now vs defer) | 2026-02-04 | Accepted | Non-goal: dependency upgrades beyond security/compliance in Phase 0 | `cargo tree -d --target all` shows `getrandom` + `windows-sys` duplicates; `audit_pack/deps/cargo_tree_duplicates.txt` showed none (audit-time command likely ran without `--target all`). Decision: defer consolidation unless a security/compliance gate requires it. |
| Allowlist license expressions in `deny.toml` | 2026-02-04 | Accepted | `cargo deny` defaults fail without config; policy must be explicit | `deny.toml` establishes allowlist + target scoping + confidence; `cargo deny check licenses` PASS (see §8 “Phase 0 preflight…” entry). |
| Execution evidence location (`audit_pack/execution/YYYY-MM-DD/...`) | 2026-02-04 | Accepted | Program requires evidence citations via `audit_pack/` paths | Add execution logs as new files under `audit_pack/execution/` while keeping the audit-time baseline evidence files in §2 unchanged. |
