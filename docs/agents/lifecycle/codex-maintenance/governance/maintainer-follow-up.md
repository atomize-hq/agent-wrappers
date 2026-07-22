# Maintainer follow-up — Codex `0.144.6`

## Blocking support-surface contract mismatch

The packet has trustworthy snapshots for its required target
`x86_64-unknown-linux-musl` and for `aarch64-apple-darwin`. The deterministic
union and coverage reports were generated from those snapshots. The union is
intentionally incomplete only because the packet has no
`aarch64-unknown-linux-musl` or `x86_64-pc-windows-msvc` snapshot.

Those reports contradict the frozen `support_surface_audit` classification:
`coverage.any.json` records 22 missing commands, 38 missing flags, and eight
missing positional arguments relative to the regenerated wrapper coverage.
The baseline `0.125.0` `coverage.any.json` had zero missing commands, flags,
and arguments. Examples include `archive`, `delete`, `doctor`, the
`app-server daemon` subtree, plugin commands, remote-control commands, and
their related options. These are not the two preexisting `codex completion`
gaps that the request allows to be deferred.

`cargo run -p xtask -- refresh-agent --request
docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml
--write` therefore refuses to refresh the packet: its derived audit no longer
matches the frozen `support_surface_audit` block. Do not claim this run has no
newly discovered non-TUI debt, do not promote `0.144.6`, and do not create a
closeout. A maintainer must regenerate/re-authorize the maintenance request
with the observed rows and either supply bounded uplifts or record an allowed
deferral for each row before this packet can continue.

## Materialized evidence

- `snapshots/0.144.6/union.json` is a partial union with the required target
  present.
- `reports/0.144.6/coverage.any.json` and the two available per-target reports
  contain the observed deltas.
- `versions/0.144.6.json` is `reported`, not `validated` or `supported`.
- The current validated baseline and all promotion pointers remain `0.125.0`.

## Gate record

The declared commands passed after the unsupported publication rows were
materialized: `cargo fmt --all`, `cargo run -p xtask -- codex-validate --root
cli_manifests/codex`, `cargo run -p xtask -- support-matrix --check`, `cargo
run -p xtask -- capability-matrix --check`, `cargo run -p xtask --
capability-matrix-audit`, and `make preflight`.

Passing those repository gates does not resolve the frozen audit mismatch or
authorize closeout.
