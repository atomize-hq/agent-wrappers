# Codex CLI Manifests (`cli_manifests/codex`)

This directory is the home for **Codex CLI snapshot artifacts** used to maintain wrapper parity over time.

Source of truth for the policy is `docs/adr/0001-codex-cli-parity-maintenance.md`.

## Ops Playbook

Maintainer runbook (Release Watch triage, snapshot update + review checklist, intentionally-unwrapped policy, and promotion criteria): [OPS_PLAYBOOK.md](OPS_PLAYBOOK.md)

## Generate / Update (`current.json`)

Canonical generator command:

`cargo run -p xtask -- codex-snapshot --codex-binary <PATH_TO_CODEX> --out-dir cli_manifests/codex --capture-raw-help --supplement cli_manifests/codex/supplement/commands.json`

Notes:
- The generator must be pointed at a local `codex` binary; it does not download binaries and should not use the network.
- Recommended local convention (gitignored): `./.codex-bins/<version>/codex-x86_64-unknown-linux-musl`

## On-disk Layout (v1)

- `min_supported.txt` — minimum supported Codex CLI version (single semver line).
- `latest_validated.txt` — latest Codex CLI version that passed the validation matrix (single semver line).
- `current.json` — generated snapshot for `latest_validated.txt`.

Optional/generated:

- `raw_help/<semantic_version>/**` — raw `--help` captures (for debugging help parser drift):
  - Root help: `raw_help/<semantic_version>/help.txt`
  - Per-command help: `raw_help/<semantic_version>/commands/<token1>/<token2>/help.txt`
- `supplement/commands.json` — small, explicit hand-maintained supplements for known “help omissions”.

## Conventions

- Keep JSON deterministic: stable sort order, avoid timestamps in fields that would churn diffs unnecessarily (use `collected_at` only).
- Treat `min_supported.txt` and `latest_validated.txt` as the only authoritative pointers.
- Avoid large “history dumps” unless we have a concrete need; prefer keeping only the current snapshot plus raw help for the validated version.

## Snapshot Schema (v1)

`current.json` is a single JSON object with:
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
  - `platforms` (array of strings, optional): `linux|macos|windows|wsl`
  - `args` (array, optional): help-derived positional args (when discoverable)
  - `flags` (array, optional): help-derived flags/options:
    - `long` (string, optional): includes leading `--` (example: `--json`)
    - `short` (string, optional): includes leading `-` (example: `-j`)
    - `takes_value` (bool)
    - `value_name` (string, optional): as shown in help (example: `PATH`)
    - `repeatable` (bool, optional)
    - `stability` (string, optional)
    - `platforms` (array, optional): `linux|macos|windows|wsl`
- `features` (object, optional): reserved for feature-probe output.
- `known_omissions` (array of strings, optional): records applied supplements for review visibility.

## Deterministic Ordering Rules (v1)

- `commands` are sorted lexicographically by `path` tokens (token-by-token; shorter prefix first).
- `flags` are sorted by `(long, short)`:
  - missing `long` sorts after present
  - missing `short` sorts after present
  - ties preserve original discovery order (stable sort)

## Help Supplements (v1)

`supplement/commands.json` exists to make known help gaps explicit and reviewable (without heuristics).

Schema:
- `version` (int): must be `1`
- `commands` (array):
  - `path` (array of strings): command tokens
  - `platforms` (array of strings, optional): `linux|macos|windows|wsl`
  - `note` (string): why this supplement exists

Application semantics:
- If a supplemented `path` is missing from `codex --help` discovery, the generator inserts a `commands[]` entry with that `path`.
- If `platforms` is present, it sets/overrides `commands[].platforms`.
- Help-derived fields (`about`, `usage`, `flags`, `args`) are never fabricated by supplements.
- For each applied supplement, `known_omissions` appends: `supplement/commands.json:v1:<path-joined-by-space>`.
