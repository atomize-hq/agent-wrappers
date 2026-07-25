# Repository Guidelines

## Project Structure & Module Organization

- Rust workspace at the repo root (`Cargo.toml`), with crates under `crates/`:
  - `crates/codex/` and `crates/claude_code/` — CLI wrapper libraries
  - `crates/agent_api/` — unified API surface
  - `crates/wrapper_events/` — event/adapter utilities
  - `crates/xtask/` — project automation (`cargo run -p xtask -- ...`)
- `cli_manifests/**` — committed parity artifacts (snapshots, reports, pointer files).
- `docs/` — ADRs in `docs/adr/`, plus the **canonical contracts** in `docs/specs/**` (Normative).
- `scripts/` — repo hygiene and CI helper scripts.

## Build, Test, and Development Commands

- Format: `make fmt` (check-only: `make fmt-check`)
- Lint: `make clippy` (workspace, all targets/features; warnings are errors)
- Build/typecheck: `make check`
- Tests: `make test` (or targeted: `cargo test -p codex`)
- Integration gate: `make preflight` (runs hygiene + fmt/clippy/check/test + LOC cap + security)
- Artifacts/tools: `cargo run -p xtask -- capability-matrix` and `cargo run -p xtask -- manifest-validate --root cli_manifests/codex`

## Coding Style & Naming Conventions

- Rust edition: 2021 (workspace `rust-version` is `1.78`).
- Unsafe is forbidden across crates (`#![forbid(unsafe_code)]`).
- Use `rustfmt` defaults; keep `clippy` clean.
- Prefer `snake_case` for modules/files; keep changes small and extraction-friendly (see `make loc-check`).

## Testing Guidelines

- Unit tests live in `src/` with `#[cfg(test)]`; integration tests live in `crates/*/tests/*.rs`.
- Use targeted runs while iterating, e.g. `cargo test -p xtask --test c0_spec_validate -- --nocapture`.

## Commit & Pull Request Guidelines

- Follow the repo’s established commit style: `feat:`, `fix:`, `chore:`, `docs:`, `ci:`; optional scope is common (e.g. `fix(claude_code): ...`).
- PRs should include: what changed, why, and pointers to the canonical contract in `docs/specs/**` when behavior/format changes are involved (ADRs are supporting rationale).
- If you touch `cli_manifests/**`, run `cargo run -p xtask -- manifest-validate --root cli_manifests/<agent>` locally before pushing.
- The parity engines are agent-agnostic and neutrally named (`manifest-union`, `manifest-report`, `manifest-validate`, `manifest-version-metadata`, `manifest-retain`); the historical `codex-*` / `claude-union` names remain as back-compat aliases that preserve their original default `--root`.

## Security, Docs, and Repo Hygiene

- Don’t commit generated or scratch artifacts: `target/`, `wt/`, `_download/`, `_extract/`, repo-root `*.log`, or `cli_manifests/codex/raw_help/`. Verify with `make hygiene`.
- Docs rule: if an ADR and a contract conflict, `docs/specs/**` (Normative) wins—update ADRs to point at the contract.
