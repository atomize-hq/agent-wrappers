# Codex Wrapper (Rust)

This repository provides a Rust wrapper around the OpenAI Codex CLI (`codex`) with:

- Typed streaming of `codex exec --json` events (`ThreadEvent`)
- Offline parsing of saved JSONL logs into the same `ThreadEvent` model
- Capability probing and compatibility shims for upstream drift
- Tooling and artifacts for release-trailing parity maintenance (`cli_manifests/codex/`)

## Start here

- Wrapper API docs: `crates/codex/README.md`
- Examples index: `crates/codex/EXAMPLES.md`
- Documentation index: `docs/README.md`

## Operations / parity maintenance

- Ops playbook: `cli_manifests/codex/OPS_PLAYBOOK.md`
- CLI snapshot artifacts: `cli_manifests/codex/README.md`
- Decisions (ADRs): `docs/adr/`
- Normative contracts: `docs/specs/`

