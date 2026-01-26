# C0-spec – Snapshot schema + generator (v1)

Source: `docs/adr/0001-codex-cli-parity-maintenance.md`

## Decisions (no ambiguity)
- Snapshot generator implementation location: create a new workspace crate at `crates/xtask/` with package name `xtask` and a binary named `xtask`.
- Snapshot generator command (canonical):
  - `cargo run -p xtask -- codex-snapshot --codex-binary <PATH_TO_CODEX> --out-dir cli_manifests/codex --capture-raw-help --supplement cli_manifests/codex/supplement/commands.json`
- Snapshot generator must not download binaries and must not access the network.
- Snapshot generator must not modify `cli_manifests/codex/min_supported.txt` or `cli_manifests/codex/latest_validated.txt`; those pointers are updated explicitly by humans or CI workflows (C1).

## Task Breakdown (no ambiguity)
- `C0-code` (non-test changes):
  - Create `crates/xtask/` implementing the canonical `codex-snapshot` command.
  - Update `cli_manifests/codex/README.md` to include schema v1, the canonical command, and the canonical on-disk layout.
  - Add `cli_manifests/codex/supplement/commands.json` with `version: 1` and at least one documented example entry (it may be a no-op supplement initially).
- `C0-test` (tests only):
  - Add tests under `crates/xtask/tests/` plus any test-only fixtures under `crates/xtask/tests/fixtures/` to validate:
    - `commands` ordering follows “Deterministic ordering rules (v1)” below
    - `flags` ordering follows “Deterministic ordering rules (v1)” below
    - supplement entries from `cli_manifests/codex/supplement/commands.json` are applied and recorded in `known_omissions`
    - snapshot JSON is deterministic when `collected_at` is fixed in test
- `C0-integ`:
  - Merge `C0-code` + `C0-test`, reconcile to this spec, and run:
    - `cargo fmt`
    - `cargo clippy --workspace --all-targets -- -D warnings`
    - `cargo test -p xtask`
    - `make preflight`

## Scope
- Create/own the snapshot storage layout under `cli_manifests/codex/`:
  - `cli_manifests/codex/min_supported.txt` (authoritative; single semver line)
  - `cli_manifests/codex/latest_validated.txt` (authoritative; single semver line)
  - `cli_manifests/codex/current.json` (generated snapshot for `latest_validated.txt`)
  - `cli_manifests/codex/README.md` (schema + generation instructions + conventions)
  - Optional generated/debug artifacts:
    - `cli_manifests/codex/raw_help/<version>/**` (raw `--help` captures)
    - `cli_manifests/codex/supplement/**` (hand-maintained help-gap supplements)
- Implement the **snapshot generator** as specified in “Decisions (no ambiguity)”.
- The generator must be:
  - **Exhaustive & recursive**: enumerate commands/subcommands from `codex --help`, then run `--help` for every discovered command path until leaf commands are reached.
  - **Diff-first deterministic**: stable ordering of arrays (especially `commands`), canonicalized whitespace where appropriate, and stable serialization so PR diffs are meaningful.
  - **Help-gap aware**: support a small, explicit supplement mechanism for known omissions in help text (example: `sandbox` platform variants not shown in `--help`) and surface applied supplements in the snapshot for review.
  - **Safe-by-default**: no binary downloads; no network; no writes outside the intended output directory (plus OS temp dir if needed).

### Canonical on-disk layout (no ambiguity)
- Snapshot JSON:
  - `cli_manifests/codex/current.json`
  - Formatting: UTF-8, LF newlines, 2-space pretty JSON, trailing newline.
- Raw help captures (when `--capture-raw-help` is set):
  - Root help: `cli_manifests/codex/raw_help/<semantic_version>/help.txt`
  - Per-command help:
    - `cli_manifests/codex/raw_help/<semantic_version>/commands/<token1>/<token2>/help.txt`
    - Example: `cli_manifests/codex/raw_help/0.77.0/commands/exec/resume/help.txt`
- Supplement file (hand-maintained):
  - `cli_manifests/codex/supplement/commands.json`
  - Schema:
    - `version` (int): must be `1`
    - `commands` (array): each item:
      - `path` (array of strings): command/subcommand tokens (same shape as snapshot `commands[].path`)
      - `platforms` (array of strings, optional): `linux|macos|windows|wsl`
      - `note` (string): why this supplement exists / what help omission it covers

### Deterministic ordering rules (v1)
To keep diffs meaningful, snapshot generation must apply the following stable ordering rules before serialization:
- `commands` are sorted lexicographically by `path` tokens:
  - Compare token-by-token using Rust’s default string ordering (byte/Unicode scalar ordering; case-sensitive).
  - If one path is a strict prefix of the other, the shorter path sorts first (e.g., `["exec"]` before `["exec","resume"]`).
- `flags` are sorted by `(long, short)` with explicit handling for missing keys:
  - Primary key: `long` (missing sorts after present).
  - Secondary key: `short` (missing sorts after present).
  - Ties are broken by stable sort (preserve original discovery order).

### Supplement application semantics (v1)
The supplement mechanism exists to make help gaps explicit and reviewable without relying on heuristic parsing.
- Each entry in `cli_manifests/codex/supplement/commands.json` represents a command path that should exist in the snapshot even if it is missing from `--help` output.
- Application rules:
  - If the supplemented `path` is missing from the discovered command set, insert a `commands[]` entry with that `path`.
  - If `platforms` is present, set/override `commands[].platforms` to those values.
  - Preserve help-derived fields (`about`, `usage`, `flags`, `args`) when available; supplements should not fabricate help text.
- `known_omissions` recording:
  - For every applied supplement entry, append an identifier string of the form:
    - `supplement/commands.json:v1:<path-joined-by-space>`
    - Example: `supplement/commands.json:v1:exec resume`

### Snapshot schema (v1)
`cli_manifests/codex/current.json` fields (required unless marked optional):
- `snapshot_schema_version` (int): must be `1`.
- `tool` (string): must be `codex-cli`.
- `collected_at` (RFC3339 string): snapshot generation timestamp.
- `binary` (object):
  - `sha256` (string)
  - `size_bytes` (int)
  - `platform` (object): `os` (string), `arch` (string)
  - `version_output` (string)
  - `semantic_version` (string, optional)
  - `channel` (string, optional): `stable|beta|nightly|unknown` (when derivable)
  - `commit` (string, optional)
- `commands` (array; stable-sorted):
  - `path` (array of strings): command/subcommand tokens (e.g., `["exec","resume"]`)
  - `about` (string, optional)
  - `usage` (string, optional)
  - `stability` (string, optional): `stable|experimental|beta|deprecated|unknown` (when derivable)
  - `platforms` (array of strings, optional): if command is platform-specific (e.g., `["linux","macos"]`)
  - `args` (array of objects, optional): positional args as discoverable from help; each item:
    - `name` (string): positional arg name as shown in help
    - `required` (bool)
    - `variadic` (bool)
    - `note` (string, optional): any extra help-derived notes
  - `flags` (array of objects, optional): stable-sorted by `long` then `short`; each item:
    - `long` (string, optional): full token including leading `--` (example: `--json`)
    - `short` (string, optional): full token including leading `-` (example: `-j`)
    - `takes_value` (bool)
    - `value_name` (string, optional): as shown in help (example: `PATH`)
    - `repeatable` (bool, optional)
    - `stability` (string, optional): `stable|experimental|beta|deprecated|unknown` (when derivable)
    - `platforms` (array of strings, optional): `linux|macos|windows|wsl`
- `features` (object, optional):
  - `supports_json` (bool, optional): whether `codex features list --json` is accepted
  - `raw_text` (string, optional)
  - `raw_json` (object, optional)
- `known_omissions` (array of strings, optional): identifiers for any supplements applied (for review visibility)

## Acceptance Criteria
- `cli_manifests/codex/README.md` documents:
  - the v1 schema above,
  - how to generate/update the snapshot locally given a `codex` binary path,
  - conventions for raw captures and supplements.
- Snapshot generator can produce a valid `cli_manifests/codex/current.json` for a supplied `codex` binary path, and the output is deterministic except for `collected_at`.
- Generator enumerates help recursively and captures enough raw help output to debug parsing drift (either always or behind an explicit flag).
- Supplement mechanism exists and, when used, is recorded in `known_omissions`.

## Out of Scope
- GitHub workflows (Release Watch / Update Snapshot) and CI gating (C1).
- JSONL and server notification compatibility work (C2).
- Playbook/promotion criteria documentation beyond what’s strictly needed to explain snapshot generation (C3).
- Multi-CLI manifests (explicit non-goal in ADR 0001).
